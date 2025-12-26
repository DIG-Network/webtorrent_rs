// Comprehensive test runner for all API permutation tests
// Run with: cargo run --example test_runner

use std::process::Command;
use std::path::Path;
use std::fs;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== WebTorrent-RS Comprehensive API Permutation Test Runner ===\n");

    // Find all test example files
    let examples_dir = Path::new("examples");
    let mut test_files = Vec::new();

    if examples_dir.exists() {
        for entry in fs::read_dir(examples_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("rs") {
                if let Some(file_name) = path.file_stem().and_then(|s| s.to_str()) {
                    if file_name.starts_with("test_") {
                        test_files.push(file_name.to_string());
                    }
                }
            }
        }
    }

    test_files.sort();

    println!("Found {} test examples\n", test_files.len());

    let mut passed = 0;
    let mut failed = 0;
    let mut skipped = 0;

    for test_file in &test_files {
        print!("Running {}... ", test_file);
        std::io::Write::flush(&mut std::io::stdout()).unwrap();

        let output = Command::new("cargo")
            .args(&["run", "--example", test_file])
            .output();

        match output {
            Ok(output) => {
                if output.status.success() {
                    println!("✓ PASSED");
                    passed += 1;
                } else {
                    println!("✗ FAILED");
                    failed += 1;
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    if !stderr.is_empty() {
                        println!("  Error: {}", stderr.lines().next().unwrap_or("Unknown error"));
                    }
                }
            },
            Err(e) => {
                println!("⚠ SKIPPED (error: {})", e);
                skipped += 1;
            }
        }
    }

    println!("\n=== Test Summary ===");
    println!("Total tests: {}", test_files.len());
    println!("Passed: {}", passed);
    println!("Failed: {}", failed);
    println!("Skipped: {}", skipped);
    println!("\nSuccess rate: {:.1}%", (passed as f64 / test_files.len() as f64) * 100.0);

    if failed > 0 {
        std::process::exit(1);
    }

    Ok(())
}

