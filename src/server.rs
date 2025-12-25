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

            let _file = &files[file_index];
            Self::serve_file(&mut stream, torrent.clone(), file_index, method == "HEAD").await?;
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
    ) -> Result<()> {
        let files = torrent.files();
        if file_index >= files.len() {
            return Self::send_error_response(stream, 404, "File not found").await;
        }

        let file = &files[file_index];
        let file_name = file.path();
        let _file_length = file.length();
        let file_length = file.length();

        // For now, send a simple response
        // In a full implementation, we would:
        // 1. Read the file data from the store
        // 2. Handle range requests (HTTP 206 Partial Content)
        // 3. Stream the data efficiently

        let response = format!(
            "HTTP/1.1 200 OK\r\n\
             Content-Type: application/octet-stream\r\n\
             Content-Length: {}\r\n\
             Content-Disposition: attachment; filename=\"{}\"\r\n\
             \r\n",
            file_length,
            file_name
        );

        stream.write_all(response.as_bytes()).await.map_err(|e| {
            WebTorrentError::Network(format!("Failed to write HTTP response: {}", e))
        })?;

        if !is_head {
            // TODO: Stream file data from store
            // For now, we just send the headers
            warn!("File serving not fully implemented - would stream {} bytes", file_length);
        }

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

