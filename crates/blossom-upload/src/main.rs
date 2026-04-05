use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use base64::Engine;
use clap::Parser;
use nostr_sdk::prelude::*;
use reqwest::Client;
use sha2::{Digest, Sha256};

const DEFAULT_SERVERS: &[&str] = &[
    "https://blossom.primal.net",
    "https://cdn.satellite.earth",
    "https://blossom.oxtr.dev",
];

#[derive(Parser)]
#[command(name = "blossom-upload", about = "Upload files to Blossom servers")]
struct Cli {
    /// Files to upload
    #[arg(required = true)]
    files: Vec<PathBuf>,

    /// Blossom server URL (repeatable; defaults to public servers)
    #[arg(long)]
    server: Vec<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let servers: Vec<String> = if cli.server.is_empty() {
        DEFAULT_SERVERS.iter().map(|s| s.to_string()).collect()
    } else {
        cli.server
    };

    let keys = Keys::generate();
    let http = Client::new();
    let mut failed = Vec::new();

    for path in &cli.files {
        let data = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
        let content_type = mime_from_path(path);
        let sha256_hex = hex::encode(Sha256::digest(&data));

        match try_upload(&http, &servers, &data, &content_type, &sha256_hex, &keys).await {
            Ok(url) => println!("{url}"),
            Err(e) => {
                eprintln!("failed to upload {}: {e}", path.display());
                failed.push(path.display().to_string());
            }
        }
    }

    if !failed.is_empty() {
        anyhow::bail!("{} upload(s) failed: {}", failed.len(), failed.join(", "));
    }

    Ok(())
}

async fn try_upload(
    http: &Client,
    servers: &[String],
    data: &[u8],
    content_type: &str,
    sha256_hex: &str,
    keys: &Keys,
) -> Result<String, String> {
    let mut last_error: Option<String> = None;

    for server in servers {
        let upload_url = format!("{}/upload", server.trim_end_matches('/'));

        // Build NIP-98 authorization event (kind 24242 for Blossom)
        let auth_header = match build_auth_header(keys, &upload_url, sha256_hex, data.len()).await {
            Ok(h) => h,
            Err(e) => {
                last_error = Some(format!("{server}: auth build failed: {e}"));
                continue;
            }
        };

        let resp = match http
            .put(&upload_url)
            .header("Authorization", &auth_header)
            .header("Content-Type", content_type)
            .body(data.to_vec())
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                last_error = Some(format!("{server}: request failed: {e}"));
                continue;
            }
        };

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            last_error = Some(format!("{server}: HTTP {status}: {body}"));
            continue;
        }

        let body = match resp.text().await {
            Ok(b) => b,
            Err(e) => {
                last_error = Some(format!("{server}: failed to read response: {e}"));
                continue;
            }
        };

        // Parse Blossom response — expects JSON with url and sha256 fields
        let descriptor: BlobDescriptor = match serde_json::from_str(&body) {
            Ok(d) => d,
            Err(e) => {
                last_error = Some(format!("{server}: bad response JSON: {e} (body: {body})"));
                continue;
            }
        };

        if !descriptor.sha256.eq_ignore_ascii_case(sha256_hex) {
            last_error = Some(format!(
                "{server}: hash mismatch (expected {sha256_hex}, got {})",
                descriptor.sha256
            ));
            continue;
        }

        return Ok(descriptor.url);
    }

    Err(last_error.unwrap_or_else(|| "no servers configured".into()))
}

/// Build a Blossom authorization header (NIP-98 style, kind 24242).
///
/// The event includes:
/// - kind 24242
/// - "t" tag with value "upload"
/// - "x" tag with the file's SHA-256 hex
/// - "size" tag with the file size
/// - content is empty
async fn build_auth_header(
    keys: &Keys,
    _url: &str,
    sha256_hex: &str,
    size: usize,
) -> Result<String> {
    let event = EventBuilder::new(Kind::Custom(24242), "")
        .tag(Tag::custom(TagKind::Custom("t".into()), ["upload"]))
        .tag(Tag::custom(TagKind::Custom("x".into()), [sha256_hex]))
        .tag(Tag::custom(
            TagKind::Custom("size".into()),
            [&size.to_string()],
        ))
        .tag(Tag::custom(
            TagKind::Custom("expiration".into()),
            [&(Timestamp::now().as_u64() + 300).to_string()],
        ))
        .sign_with_keys(keys)?;

    let event_json = event.as_json();
    let encoded = base64::engine::general_purpose::STANDARD.encode(event_json.as_bytes());

    Ok(format!("Nostr {encoded}"))
}

#[derive(serde::Deserialize)]
struct BlobDescriptor {
    url: String,
    sha256: String,
    #[allow(dead_code)]
    size: Option<u64>,
    #[allow(dead_code)]
    #[serde(rename = "type")]
    content_type: Option<String>,
}

fn mime_from_path(path: &Path) -> String {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        "txt" => "text/plain",
        "html" | "htm" => "text/html",
        "css" => "text/css",
        "js" => "application/javascript",
        "json" => "application/json",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        "mp3" => "audio/mpeg",
        "ogg" => "audio/ogg",
        "wav" => "audio/wav",
        "pdf" => "application/pdf",
        "zip" => "application/zip",
        _ => "application/octet-stream",
    }
    .to_string()
}
