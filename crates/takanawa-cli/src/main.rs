use std::path::PathBuf;

use takanawa_core::{HashConfig, HashKind, Result, TakanawaError};
use takanawa_http::{
    DEFAULT_MAX_IO, DownloadConfig, DownloadEngine, RetryConfig, TimeoutConfig,
    download_to_completion,
};

#[tokio::main]
async fn main() {
    if let Err(err) = run().await {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if !(2..=4).contains(&args.len()) {
        return Err(TakanawaError::InvalidConfig(
            "usage: takanawa-cli <url> <target-path> [hash-kind] [hash-hex]\n       takanawa-cli <url> <target-path> [sha256-hex]"
                .to_owned(),
        ));
    }

    let hash = parse_hash_args(&args[2..])?;

    let engine = DownloadEngine::new(DEFAULT_MAX_IO)?;
    let snapshot = download_to_completion(
        engine,
        DownloadConfig {
            url: args[0].clone(),
            target_path: PathBuf::from(&args[1]),
            chunk_size: 0,
            parallelism: 0,
            max_parallel_chunks: 0,
            retry: RetryConfig::default(),
            timeout: TimeoutConfig::default(),
            bytes_per_second_limit: 0,
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

fn parse_hash_args(args: &[String]) -> Result<HashConfig> {
    match args {
        [] => Ok(HashConfig::None),
        [sha256_hex] => parse_hash(HashKind::Sha256, sha256_hex),
        [kind, hash_hex] => parse_hash(parse_hash_kind(kind)?, hash_hex),
        _ => Err(TakanawaError::InvalidConfig(
            "expected [hash-kind] [hash-hex]".to_owned(),
        )),
    }
}

fn parse_hash_kind(value: &str) -> Result<HashKind> {
    match value.to_ascii_lowercase().replace('-', "").as_str() {
        "sha1" => Ok(HashKind::Sha1),
        "sha256" => Ok(HashKind::Sha256),
        "sha512" => Ok(HashKind::Sha512),
        "md5" => Ok(HashKind::Md5),
        "crc32" => Ok(HashKind::Crc32),
        _ => Err(TakanawaError::InvalidConfig(format!(
            "unsupported hash kind {value}; expected sha1, sha256, sha512, md5, or crc32"
        ))),
    }
}

fn parse_hash(kind: HashKind, value: &str) -> Result<HashConfig> {
    let bytes = hex::decode(value).map_err(|err| {
        TakanawaError::InvalidConfig(format!("invalid {} hex: {err}", kind.name()))
    })?;
    HashConfig::from_expected_bytes(kind, &bytes).ok_or_else(|| {
        TakanawaError::InvalidConfig(format!(
            "{} must be {} bytes",
            kind.name(),
            kind.expected_len()
        ))
    })
}
