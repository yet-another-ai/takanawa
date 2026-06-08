use std::path::PathBuf;

use takanawa_core::{HashConfig, Result, TakanawaError};
use takanawa_http::{DEFAULT_MAX_IO, DownloadConfig, DownloadEngine, download_to_completion};

#[tokio::main]
async fn main() {
    if let Err(err) = run().await {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if !(2..=3).contains(&args.len()) {
        return Err(TakanawaError::InvalidConfig(
            "usage: takanawa-cli <url> <target-path> [sha256-hex]".to_owned(),
        ));
    }

    let hash = if let Some(value) = args.get(2) {
        let bytes = hex::decode(value)
            .map_err(|err| TakanawaError::InvalidConfig(format!("invalid SHA-256 hex: {err}")))?;
        let hash: [u8; 32] = bytes
            .try_into()
            .map_err(|_| TakanawaError::InvalidConfig("SHA-256 must be 32 bytes".to_owned()))?;
        HashConfig::Sha256(hash)
    } else {
        HashConfig::None
    };

    let engine = DownloadEngine::new(DEFAULT_MAX_IO)?;
    let snapshot = download_to_completion(
        engine,
        DownloadConfig {
            url: args[0].clone(),
            target_path: PathBuf::from(&args[1]),
            chunk_size: 0,
            parallelism: 0,
            hash,
        },
    )
    .await?;

    println!(
        "completed: {} bytes in {} chunks",
        snapshot.content_len, snapshot.chunk_count
    );
    Ok(())
}
