use crate::client::WebTorrent;
use crate::error::{Result, WebTorrentError};
use crate::torrent::Torrent;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tracing::{debug, error, info, warn};
use hex;

/// HTTP server for serving torrent files
pub struct Server {
    #[allow(dead_code)]
    client: Arc<WebTorrent>,
    destroyed: Arc<RwLock<bool>>,
    #[allow(dead_code)]
    handle: Option<tokio::task::JoinHandle<()>>,
    port: u16,
}

impl Server {
    pub async fn new(client: Arc<WebTorrent>, port: u16) -> Result<Self> {
        let destroyed = Arc::new(RwLock::new(false));
        let destroyed_clone = Arc::clone(&destroyed);
        let client_clone = Arc::clone(&client);

        // Start HTTP server
        // Use spawn_local or ensure the future is Send
        // For now, we'll use a simpler approach that doesn't require Send
        let handle = tokio::task::spawn(async move {
            let addr = format!("0.0.0.0:{}", port);
            match TcpListener::bind(&addr).await {
                Ok(listener) => {
                    info!("HTTP server listening on {}", addr);
                    loop {
                        // Check if destroyed
                        if *destroyed_clone.read().await {
                            break;
                        }

                        // Accept connections
                        match listener.accept().await {
                            Ok((stream, addr)) => {
                                debug!("HTTP connection from {}", addr);
                                let client_task = Arc::clone(&client_clone);
                                // Spawn task - ensure it's Send by only using Send types
                                tokio::task::spawn(async move {
                                    if let Err(e) = Self::handle_http_request(client_task, stream).await {
                                        error!("Error handling HTTP request from {}: {}", addr, e);
                                    }
                                });
                            }
                            Err(e) => {
                                if *destroyed_clone.read().await {
                                    break;
                                }
                                error!("Error accepting HTTP connection: {}", e);
                                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to bind HTTP server on {}: {}", addr, e);
                }
            }
        });

        Ok(Self {
            client,
            destroyed,
            handle: Some(handle),
            port,
        })
    }

    async fn handle_http_request(
        client: Arc<WebTorrent>,
        mut stream: TcpStream,
    ) -> Result<()> {
        // Read HTTP request
        let mut buffer = [0u8; 8192];
        let n = stream.read(&mut buffer).await.map_err(|e| {
            WebTorrentError::Network(format!("Failed to read HTTP request: {}", e))
        })?;

        if n == 0 {
            return Ok(()); // Connection closed
        }

        let request = String::from_utf8_lossy(&buffer[..n]);
        let lines: Vec<&str> = request.lines().collect();

        if lines.is_empty() {
            return Ok(());
        }

        // Parse request line (e.g., "GET /info_hash HTTP/1.1")
        let request_line = lines[0];
        let parts: Vec<&str> = request_line.split_whitespace().collect();
        
        if parts.len() < 2 {
            return Self::send_error_response(&mut stream, 400, "Bad Request").await;
        }

        let method = parts[0];
        let path = parts[1];

        // Handle OPTIONS request (CORS preflight)
        if method == "OPTIONS" {
            return Self::send_cors_preflight_response(&mut stream).await;
        }

        // Parse headers
        let mut range_header: Option<String> = None;
        for line in lines.iter().skip(1) {
            if line.is_empty() {
                break; // End of headers
            }
            if line.to_lowercase().starts_with("range:") {
                range_header = Some(line[6..].trim().to_string());
            }
        }

        if method != "GET" && method != "HEAD" {
            return Self::send_error_response(&mut stream, 405, "Method Not Allowed").await;
        }

        // Parse path: /<info_hash> or /<info_hash>/<file_index>
        let path_parts: Vec<&str> = path.trim_start_matches('/').split('/').collect();
        
        if path_parts.is_empty() || path_parts[0].is_empty() {
            return Self::send_error_response(&mut stream, 404, "Not Found").await;
        }

        let info_hash_str = path_parts[0];
        let info_hash = hex::decode(info_hash_str)
            .map_err(|_| WebTorrentError::InvalidInfoHash("Invalid hex encoding".to_string()))?;

        if info_hash.len() != 20 {
            return Self::send_error_response(&mut stream, 400, "Invalid info hash length").await;
        }

        let mut hash = [0u8; 20];
        hash.copy_from_slice(&info_hash);

        // Find torrent
        let torrent = if let Some(t) = client.get(&hash).await {
            t
        } else {
            return Self::send_error_response(&mut stream, 404, "Torrent not found").await;
        };

        // Handle file index if provided
        if path_parts.len() > 1 {
            // Serve specific file
            let file_index: usize = path_parts[1].parse().map_err(|_| {
                WebTorrentError::InvalidTorrent("Invalid file index".to_string())
            })?;

            let files = torrent.files();
            if file_index >= files.len() {
                return Self::send_error_response(&mut stream, 404, "File not found").await;
            }

            Self::serve_file(&mut stream, torrent.clone(), file_index, method == "HEAD", range_header).await?;
        } else {
            // Serve torrent metadata or list files
            Self::serve_torrent_info(&mut stream, torrent.clone(), method == "HEAD").await?;
        }

        Ok(())
    }

    async fn serve_torrent_info(
        stream: &mut TcpStream,
        torrent: Arc<Torrent>,
        is_head: bool,
    ) -> Result<()> {
        let name = torrent.name();
        let files = torrent.files();
        
        // Create simple HTML listing
        let mut html = format!(
            "<!DOCTYPE html><html><head><title>{}</title></head><body>",
            html_escape::encode_text(name)
        );
        html.push_str(&format!("<h1>{}</h1>", html_escape::encode_text(name)));
        html.push_str("<ul>");
        
        for (i, file) in files.iter().enumerate() {
            let file_name = file.path();
            html.push_str(&format!(
                "<li><a href=\"/{}/{}\">{}</a> ({})</li>",
                hex::encode(torrent.info_hash()),
                i,
                html_escape::encode_text(file_name),
                file.length()
            ));
        }
        
        html.push_str("</ul></body></html>");

        let response = format!(
            "HTTP/1.1 200 OK\r\n\
             Content-Type: text/html; charset=utf-8\r\n\
             Content-Length: {}\r\n\
             Access-Control-Allow-Origin: *\r\n\
             Access-Control-Allow-Methods: GET, HEAD, OPTIONS\r\n\
             \r\n",
            html.len()
        );

        stream.write_all(response.as_bytes()).await.map_err(|e| {
            WebTorrentError::Network(format!("Failed to write HTTP response: {}", e))
        })?;

        if !is_head {
            stream.write_all(html.as_bytes()).await.map_err(|e| {
                WebTorrentError::Network(format!("Failed to write HTML: {}", e))
            })?;
        }

        Ok(())
    }

    async fn serve_file(
        stream: &mut TcpStream,
        torrent: Arc<Torrent>,
        file_index: usize,
        is_head: bool,
        range_header: Option<String>,
    ) -> Result<()> {
        let files = torrent.files();
        if file_index >= files.len() {
            return Self::send_error_response(stream, 404, "File not found").await;
        }

        let file = &files[file_index];
        let file_name = file.path();
        let file_length = file.length();
        let file_offset = file.offset();
        let piece_length = torrent.piece_length().await;

        // Determine MIME type from file extension
        let content_type = Self::get_mime_type(file_name);

        // Parse range header if present
        let (range_start, range_end) = if let Some(ref range) = range_header {
            Self::parse_range_header(range, file_length)?
        } else {
            (0, file_length)
        };

        // Validate range
        if range_start >= file_length || range_end > file_length || range_start > range_end {
            return Self::send_error_response(stream, 416, "Range Not Satisfiable").await;
        }

        let content_length = range_end - range_start;
        let is_partial = range_header.is_some();

        // Build response headers
        let mut headers = Vec::new();
        
        if is_partial {
            headers.push(format!("HTTP/1.1 206 Partial Content\r\n"));
            headers.push(format!("Content-Range: bytes {}-{}/{}\r\n", range_start, range_end - 1, file_length));
        } else {
            headers.push(format!("HTTP/1.1 200 OK\r\n"));
        }
        
        headers.push(format!("Content-Type: {}\r\n", content_type));
        headers.push(format!("Content-Length: {}\r\n", content_length));
        headers.push(format!("Accept-Ranges: bytes\r\n"));
        headers.push(format!("Content-Disposition: inline; filename=\"{}\"\r\n", 
            file_name.replace('"', "\\\"")));
        
        // CORS headers
        headers.push(format!("Access-Control-Allow-Origin: *\r\n"));
        headers.push(format!("Access-Control-Allow-Methods: GET, HEAD, OPTIONS\r\n"));
        headers.push(format!("Access-Control-Allow-Headers: Range\r\n"));
        headers.push(format!("Access-Control-Expose-Headers: Content-Range, Accept-Ranges\r\n"));
        
        headers.push(format!("\r\n"));

        let response = headers.join("");
        stream.write_all(response.as_bytes()).await.map_err(|e| {
            WebTorrentError::Network(format!("Failed to write HTTP response: {}", e))
        })?;

        if !is_head {
            // Stream file data from store
            if let Some(store) = torrent.store() {
                Self::stream_file_data(stream, &store, torrent.clone(), file_index, file_offset, 
                    piece_length, range_start, content_length).await?;
            } else {
                warn!("No store available for torrent - cannot serve file data");
            }
        }

        Ok(())
    }

    /// Parse Range header (e.g., "bytes=0-1023" or "bytes=1024-")
    fn parse_range_header(range: &str, file_length: u64) -> Result<(u64, u64)> {
        // Format: "bytes=start-end" or "bytes=start-"
        if !range.starts_with("bytes=") {
            return Err(WebTorrentError::InvalidTorrent("Invalid range format".to_string()));
        }

        let range_spec = &range[6..]; // Skip "bytes="
        let parts: Vec<&str> = range_spec.split('-').collect();
        
        if parts.len() != 2 {
            return Err(WebTorrentError::InvalidTorrent("Invalid range format".to_string()));
        }

        let start_str = parts[0];
        let end_str = parts[1];

        let start = if start_str.is_empty() {
            0
        } else {
            start_str.parse::<u64>().map_err(|_| {
                WebTorrentError::InvalidTorrent("Invalid range start".to_string())
            })?
        };

        let end = if end_str.is_empty() {
            file_length
        } else {
            end_str.parse::<u64>().map_err(|_| {
                WebTorrentError::InvalidTorrent("Invalid range end".to_string())
            })?
        };

        Ok((start, end.min(file_length)))
    }

    /// Get MIME type from file extension
    fn get_mime_type(file_name: &str) -> &'static str {
        let ext = file_name.split('.').last().unwrap_or("").to_lowercase();
        match ext.as_str() {
            "html" | "htm" => "text/html",
            "css" => "text/css",
            "js" => "application/javascript",
            "json" => "application/json",
            "png" => "image/png",
            "jpg" | "jpeg" => "image/jpeg",
            "gif" => "image/gif",
            "svg" => "image/svg+xml",
            "webp" => "image/webp",
            "mp4" => "video/mp4",
            "webm" => "video/webm",
            "mp3" => "audio/mpeg",
            "ogg" => "audio/ogg",
            "wav" => "audio/wav",
            "pdf" => "application/pdf",
            "zip" => "application/zip",
            "txt" => "text/plain",
            _ => "application/octet-stream",
        }
    }

    /// Stream file data from the store
    async fn stream_file_data(
        stream: &mut TcpStream,
        store: &Arc<dyn crate::store::ChunkStore>,
        torrent: Arc<Torrent>,
        file_index: usize,
        file_offset: u64,
        piece_length: u64,
        range_start: u64,
        content_length: u64,
    ) -> Result<()> {
        let files = torrent.files();
        let file = &files[file_index];
        let file_start_piece = file.start_piece(piece_length);
        let file_end_piece = file.end_piece(piece_length);

        // Calculate which pieces we need
        let range_end = range_start + content_length;
        let start_piece = ((file_offset + range_start) / piece_length) as usize;
        let end_piece = ((file_offset + range_end - 1) / piece_length) as usize;

        let start_piece = start_piece.max(file_start_piece);
        let end_piece = end_piece.min(file_end_piece);

        // Stream data piece by piece
        let mut bytes_written = 0u64;
        for piece_index in start_piece..=end_piece {
            let piece_start_in_file = (piece_index as u64 * piece_length).saturating_sub(file_offset);
            let piece_end_in_file = ((piece_index as u64 + 1) * piece_length).saturating_sub(file_offset).min(file.length());

            // Calculate what part of this piece we need
            let piece_range_start = range_start.max(piece_start_in_file) - piece_start_in_file;
            let piece_range_end = range_end.min(piece_end_in_file) - piece_start_in_file;

            if piece_range_start < piece_range_end {
                // Get piece data from store
                let piece_offset = piece_range_start as usize;
                let piece_length_to_read = (piece_range_end - piece_range_start) as usize;

                match store.get(piece_index, piece_offset, piece_length_to_read).await {
                    Ok(data) => {
                        // Write data to stream
                        stream.write_all(&data).await.map_err(|e| {
                            WebTorrentError::Network(format!("Failed to write file data: {}", e))
                        })?;
                        bytes_written += data.len() as u64;
                    }
                    Err(e) => {
                        warn!("Failed to get piece {} from store: {}", piece_index, e);
                        // Continue with next piece
                    }
                }
            }
        }

        debug!("Streamed {} bytes for file {}", bytes_written, file_index);
        Ok(())
    }

    async fn send_cors_preflight_response(stream: &mut TcpStream) -> Result<()> {
        let response = "HTTP/1.1 204 No Content\r\n\
             Access-Control-Allow-Origin: *\r\n\
             Access-Control-Allow-Methods: GET, HEAD, OPTIONS\r\n\
             Access-Control-Allow-Headers: Range\r\n\
             Access-Control-Max-Age: 86400\r\n\
             \r\n";

        stream.write_all(response.as_bytes()).await.map_err(|e| {
            WebTorrentError::Network(format!("Failed to write CORS preflight response: {}", e))
        })?;

        Ok(())
    }

    async fn send_error_response(
        stream: &mut TcpStream,
        status_code: u16,
        message: &str,
    ) -> Result<()> {
        let response = format!(
            "HTTP/1.1 {} {}\r\n\
             Content-Type: text/plain\r\n\
             Content-Length: {}\r\n\
             Access-Control-Allow-Origin: *\r\n\
             \r\n\
             {}",
            status_code,
            message,
            message.len(),
            message
        );

        stream.write_all(response.as_bytes()).await.map_err(|e| {
            WebTorrentError::Network(format!("Failed to write error response: {}", e))
        })?;

        Ok(())
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub async fn destroy(&self) -> Result<()> {
        if *self.destroyed.read().await {
            return Ok(());
        }

        *self.destroyed.write().await = true;

        // Wait for server to finish
        // The server will stop when destroyed is set to true
        // We can't easily await the handle here since it's behind &self
        // In production, we might use Arc<Mutex<JoinHandle>> or similar

        Ok(())
    }
}

// Simple HTML escaping helper
mod html_escape {
    pub fn encode_text(text: &str) -> String {
        text.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&#x27;")
    }
}

