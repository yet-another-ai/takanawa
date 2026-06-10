use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use reqwest::header::{
    ACCEPT_ENCODING, CONTENT_ENCODING, CONTENT_LENGTH, CONTENT_RANGE, ETAG, HeaderMap,
    LAST_MODIFIED, RANGE,
};
use reqwest::{Client, StatusCode};
use takanawa_core::{
    Chunk, ChunkPlan, DEFAULT_CHUNK_SIZE, HashConfig, PartFile, RemoteInfo, Result, TakanawaError,
};
use tokio::runtime::Runtime;
use tokio::task::JoinSet;

use crate::content_range::{parse_content_range, parse_unsatisfied_total};
use crate::limiter::IoLimiter;
use crate::state::{DownloadSnapshot, SharedState};

const MAX_ATTEMPTS: usize = 5;

#[derive(Debug, Clone)]
pub struct DownloadConfig {
    pub url: String,
    pub target_path: PathBuf,
    pub chunk_size: u64,
    pub parallelism: usize,
    pub hash: HashConfig,
}

impl DownloadConfig {
    #[must_use]
    pub fn normalized(mut self) -> Self {
        if self.chunk_size == 0 {
            self.chunk_size = DEFAULT_CHUNK_SIZE;
        }
        self
    }
}

#[derive(Debug, Clone)]
pub struct DownloadEngine {
    client: Client,
    limiter: IoLimiter,
}

impl DownloadEngine {
    pub fn new(max_io: usize) -> Result<Self> {
        let client = client_builder()
            .user_agent("takanawa/0.1")
            .connect_timeout(Duration::from_secs(30))
            .build()
            .map_err(|err| {
                TakanawaError::InvalidConfig(format!("failed to build HTTP client: {err}"))
            })?;
        Ok(Self {
            client,
            limiter: IoLimiter::new(max_io.max(1)),
        })
    }

    #[must_use]
    pub fn max_io(&self) -> usize {
        self.limiter.max()
    }

    pub fn set_max_io(&self, max_io: usize) {
        self.limiter.set_max(max_io);
    }

    fn default_parallelism(&self) -> usize {
        self.max_io().clamp(1, 4)
    }
}

fn client_builder() -> reqwest::ClientBuilder {
    let builder = Client::builder();
    #[cfg(feature = "tls-rustls")]
    {
        let roots = rustls::RootCertStore {
            roots: webpki_roots::TLS_SERVER_ROOTS.to_vec(),
        };
        let tls_config = rustls::ClientConfig::builder()
            .with_root_certificates(roots)
            .with_no_client_auth();
        return builder.tls_backend_preconfigured(tls_config);
    }
    #[allow(unreachable_code)]
    builder
}

#[derive(Debug)]
pub struct DownloadHandle {
    engine: DownloadEngine,
    config: DownloadConfig,
    state: SharedState,
    control: Arc<Control>,
    join: Mutex<Option<tokio::task::JoinHandle<()>>>,
}

#[derive(Debug, Default)]
struct Control {
    pause: AtomicBool,
    cancel: AtomicBool,
}

impl DownloadHandle {
    #[must_use]
    pub fn new(engine: DownloadEngine, config: DownloadConfig) -> Self {
        Self {
            engine,
            config: config.normalized(),
            state: SharedState::new(),
            control: Arc::new(Control::default()),
            join: Mutex::new(None),
        }
    }

    pub fn start_on(&self, runtime: &Runtime) -> Result<()> {
        let mut join = self.join.lock().expect("download join mutex poisoned");
        if join.as_ref().is_some_and(|handle| !handle.is_finished()) {
            return Err(TakanawaError::AlreadyStarted);
        }
        if join
            .as_ref()
            .is_some_and(tokio::task::JoinHandle::is_finished)
        {
            let _ = join.take();
        }

        self.control.pause.store(false, Ordering::Relaxed);
        self.control.cancel.store(false, Ordering::Relaxed);
        self.state.clear_error();
        self.state.mark_running();

        let engine = self.engine.clone();
        let config = self.config.clone();
        let state = self.state.clone();
        let control = Arc::clone(&self.control);
        *join = Some(runtime.spawn(async move {
            if let Err(err) = run_download(engine, config, state.clone(), control).await {
                match err {
                    TakanawaError::Cancelled => state.mark_cancelled(),
                    err => state.mark_failed(err.to_string()),
                }
            }
        }));
        Ok(())
    }

    pub fn pause(&self) -> Result<()> {
        self.control.pause.store(true, Ordering::Relaxed);
        self.state.request_pause();
        Ok(())
    }

    pub fn cancel(&self) -> Result<()> {
        self.control.cancel.store(true, Ordering::Relaxed);
        self.state.request_cancel();
        if !self
            .join
            .lock()
            .expect("download join mutex poisoned")
            .as_ref()
            .is_some_and(|handle| !handle.is_finished())
        {
            self.state.mark_cancelled();
        }
        Ok(())
    }

    #[must_use]
    pub fn snapshot(&self) -> DownloadSnapshot {
        self.state.snapshot()
    }

    #[must_use]
    pub fn bitmap(&self) -> Vec<u8> {
        self.state.bitmap()
    }
}

impl Drop for DownloadHandle {
    fn drop(&mut self) {
        self.control.cancel.store(true, Ordering::Relaxed);
        if let Some(join) = self
            .join
            .lock()
            .expect("download join mutex poisoned")
            .take()
        {
            join.abort();
        }
    }
}

pub async fn download_to_completion(
    engine: DownloadEngine,
    config: DownloadConfig,
) -> Result<DownloadSnapshot> {
    let state = SharedState::new();
    let control = Arc::new(Control::default());
    run_download(engine, config.normalized(), state.clone(), control).await?;
    Ok(state.snapshot())
}

async fn run_download(
    engine: DownloadEngine,
    config: DownloadConfig,
    state: SharedState,
    control: Arc<Control>,
) -> Result<()> {
    state.mark_running();
    let remote = probe_with_retry(&engine, &config, &state, &control).await?;
    let chunk_plan = ChunkPlan::new(remote.content_len, config.chunk_size)?;
    let target_path = config.target_path.clone();
    let url = config.url.clone();
    let hash = config.hash;
    let chunk_size = config.chunk_size;

    let mut part = tokio::task::spawn_blocking(move || {
        PartFile::open_or_create(&target_path, &url, &remote, chunk_size, hash)
    })
    .await
    .map_err(|err| TakanawaError::Ffi(format!("part open task failed: {err}")))??;
    state.update_from_metadata(part.metadata());

    if part.metadata().all_complete() {
        finalize_part(part, &config, &state).await?;
        return Ok(());
    }

    let mut pending: VecDeque<u64> = part.incomplete_chunks().into();
    let parallelism = if config.parallelism == 0 {
        engine.default_parallelism()
    } else {
        config.parallelism.max(1)
    };
    let mut tasks = JoinSet::new();

    loop {
        if control.cancel.load(Ordering::Relaxed) {
            state.mark_cancelled();
            return Err(TakanawaError::Cancelled);
        }
        if control.pause.load(Ordering::Relaxed) {
            tasks.abort_all();
            state.set_phase(DownloadPhase::Paused);
            return Ok(());
        }

        while !control.pause.load(Ordering::Relaxed)
            && !control.cancel.load(Ordering::Relaxed)
            && tasks.len() < parallelism
        {
            let Some(index) = pending.pop_front() else {
                break;
            };
            let chunk = chunk_plan.chunk(index)?;
            let engine = engine.clone();
            let config = config.clone();
            let state = state.clone();
            let control = Arc::clone(&control);
            tasks.spawn(async move {
                let data =
                    fetch_chunk_with_retry(&engine, &config.url, chunk, &state, &control).await?;
                Ok::<_, TakanawaError>((index, data))
            });
        }

        if tasks.is_empty() {
            break;
        }

        let Some(result) = tasks.join_next().await else {
            break;
        };
        if control.pause.load(Ordering::Relaxed) {
            tasks.abort_all();
            state.set_phase(DownloadPhase::Paused);
            return Ok(());
        }
        let (index, data) =
            result.map_err(|err| TakanawaError::Ffi(format!("download task failed: {err}")))??;
        part = tokio::task::spawn_blocking(move || {
            part.write_chunk(index, &data)?;
            Ok::<_, TakanawaError>(part)
        })
        .await
        .map_err(|err| TakanawaError::Ffi(format!("writer task failed: {err}")))??;
        state.update_from_metadata(part.metadata());

        if control.pause.load(Ordering::Relaxed) && tasks.is_empty() {
            state.mark_paused();
            return Ok(());
        }
    }

    if control.pause.load(Ordering::Relaxed) && !part.metadata().all_complete() {
        state.mark_paused();
        return Ok(());
    }

    finalize_part(part, &config, &state).await
}

async fn finalize_part(part: PartFile, config: &DownloadConfig, state: &SharedState) -> Result<()> {
    let target_path = config.target_path.clone();
    tokio::task::spawn_blocking(move || part.finalize(&target_path))
        .await
        .map_err(|err| TakanawaError::Ffi(format!("finalize task failed: {err}")))??;
    state.mark_completed();
    Ok(())
}

async fn probe_with_retry(
    engine: &DownloadEngine,
    config: &DownloadConfig,
    state: &SharedState,
    control: &Control,
) -> Result<RemoteInfo> {
    let mut delay = Duration::from_millis(100);
    for attempt in 1..=MAX_ATTEMPTS {
        if control.cancel.load(Ordering::Relaxed) {
            return Err(TakanawaError::Cancelled);
        }
        match probe_once(engine, &config.url, state).await {
            Ok(remote) => return Ok(remote),
            Err(err) if err.is_retryable() && attempt < MAX_ATTEMPTS => {
                tokio::time::sleep(delay).await;
                delay = (delay * 2).min(Duration::from_secs(3));
            }
            Err(err) => return Err(err),
        }
    }
    Err(TakanawaError::Network(
        "probe exhausted retry attempts".to_owned(),
    ))
}

async fn fetch_chunk_with_retry(
    engine: &DownloadEngine,
    url: &str,
    chunk: Chunk,
    state: &SharedState,
    control: &Control,
) -> Result<Vec<u8>> {
    let mut delay = Duration::from_millis(100);
    for attempt in 1..=MAX_ATTEMPTS {
        if control.cancel.load(Ordering::Relaxed) {
            return Err(TakanawaError::Cancelled);
        }
        match fetch_chunk_once(engine, url, chunk, state).await {
            Ok(data) => return Ok(data),
            Err(err) if err.is_retryable() && attempt < MAX_ATTEMPTS => {
                tokio::time::sleep(delay).await;
                delay = (delay * 2).min(Duration::from_secs(3));
            }
            Err(err) => return Err(err),
        }
    }
    Err(TakanawaError::Network(format!(
        "chunk {} exhausted retry attempts",
        chunk.index
    )))
}

async fn probe_once(engine: &DownloadEngine, url: &str, state: &SharedState) -> Result<RemoteInfo> {
    let _permit = engine.limiter.acquire().await;
    let _active_io = ActiveIoGuard::new(state.clone());
    let response = engine
        .client
        .get(url)
        .header(RANGE, "bytes=0-0")
        .header(ACCEPT_ENCODING, "identity")
        .send()
        .await
        .map_err(map_reqwest_error)?;

    if response.status() == StatusCode::RANGE_NOT_SATISFIABLE {
        let total = response
            .headers()
            .get(CONTENT_RANGE)
            .ok_or_else(|| {
                TakanawaError::HttpProtocol("416 response missing Content-Range".to_owned())
            })?
            .to_str()
            .map_err(|err| {
                TakanawaError::HttpProtocol(format!("invalid Content-Range header: {err}"))
            })
            .and_then(parse_unsatisfied_total)?;
        if total == 0 {
            return Ok(RemoteInfo {
                content_len: 0,
                etag: header_to_string(response.headers(), ETAG)?,
                last_modified: header_to_string(response.headers(), LAST_MODIFIED)?,
            });
        }
        return Err(TakanawaError::HttpProtocol(format!(
            "probe range was unsatisfied for non-empty resource length {total}"
        )));
    }

    validate_status(response.status())?;
    validate_identity(response.headers())?;
    let range = response_content_range(&response, 0, 0)?;
    let content_len = response_content_length(&response)?;
    if content_len != 1 {
        return Err(TakanawaError::HttpProtocol(format!(
            "probe Content-Length mismatch: expected 1, got {content_len}"
        )));
    }
    let headers = response.headers().clone();
    let body = response.bytes().await.map_err(map_reqwest_error)?;
    if body.len() != 1 {
        return Err(TakanawaError::HttpProtocol(format!(
            "probe body length mismatch: expected 1, got {}",
            body.len()
        )));
    }

    Ok(RemoteInfo {
        content_len: range.total,
        etag: header_to_string(&headers, ETAG)?,
        last_modified: header_to_string(&headers, LAST_MODIFIED)?,
    })
}

async fn fetch_chunk_once(
    engine: &DownloadEngine,
    url: &str,
    chunk: Chunk,
    state: &SharedState,
) -> Result<Vec<u8>> {
    let _permit = engine.limiter.acquire().await;
    let _active_io = ActiveIoGuard::new(state.clone());
    let response = engine
        .client
        .get(url)
        .header(RANGE, format!("bytes={}-{}", chunk.start, chunk.end))
        .header(ACCEPT_ENCODING, "identity")
        .send()
        .await
        .map_err(map_reqwest_error)?;

    validate_status(response.status())?;
    validate_identity(response.headers())?;
    let _range = response_content_range(&response, chunk.start, chunk.end)?;
    let content_len = response_content_length(&response)?;
    if content_len != chunk.len {
        return Err(TakanawaError::HttpProtocol(format!(
            "chunk {} Content-Length mismatch: expected {}, got {content_len}",
            chunk.index, chunk.len
        )));
    }
    let body = response.bytes().await.map_err(map_reqwest_error)?;
    if body.len() != usize::try_from(chunk.len).unwrap_or(usize::MAX) {
        return Err(TakanawaError::HttpProtocol(format!(
            "chunk {} body length mismatch: expected {}, got {}",
            chunk.index,
            chunk.len,
            body.len()
        )));
    }
    Ok(body.to_vec())
}

fn validate_status(status: StatusCode) -> Result<()> {
    if status == StatusCode::PARTIAL_CONTENT {
        return Ok(());
    }
    if status == StatusCode::REQUEST_TIMEOUT
        || status == StatusCode::TOO_MANY_REQUESTS
        || status.is_server_error()
    {
        return Err(TakanawaError::RetryableHttpStatus(status.as_u16()));
    }
    Err(TakanawaError::HttpProtocol(format!(
        "expected 206 Partial Content, got {status}"
    )))
}

fn validate_identity(headers: &HeaderMap) -> Result<()> {
    if let Some(value) = headers.get(CONTENT_ENCODING) {
        let value = value.to_str().map_err(|err| {
            TakanawaError::HttpProtocol(format!("invalid Content-Encoding: {err}"))
        })?;
        if !value.eq_ignore_ascii_case("identity") {
            return Err(TakanawaError::HttpProtocol(format!(
                "unexpected Content-Encoding {value}"
            )));
        }
    }
    Ok(())
}

fn response_content_range(
    response: &reqwest::Response,
    start: u64,
    end: u64,
) -> Result<crate::content_range::ContentRange> {
    let value = response
        .headers()
        .get(CONTENT_RANGE)
        .ok_or_else(|| TakanawaError::HttpProtocol("missing Content-Range".to_owned()))?
        .to_str()
        .map_err(|err| {
            TakanawaError::HttpProtocol(format!("invalid Content-Range header: {err}"))
        })?;
    let range = parse_content_range(value)?;
    if range.start != start || range.end != end {
        return Err(TakanawaError::HttpProtocol(format!(
            "Content-Range mismatch: expected {start}-{end}, got {}-{}",
            range.start, range.end
        )));
    }
    Ok(range)
}

fn response_content_length(response: &reqwest::Response) -> Result<u64> {
    response
        .headers()
        .get(CONTENT_LENGTH)
        .ok_or_else(|| TakanawaError::HttpProtocol("missing Content-Length".to_owned()))?
        .to_str()
        .map_err(|err| {
            TakanawaError::HttpProtocol(format!("invalid Content-Length header: {err}"))
        })?
        .parse::<u64>()
        .map_err(|err| TakanawaError::HttpProtocol(format!("invalid Content-Length: {err}")))
}

fn header_to_string(
    headers: &HeaderMap,
    name: reqwest::header::HeaderName,
) -> Result<Option<String>> {
    headers
        .get(name)
        .map(|value| {
            value.to_str().map(str::to_owned).map_err(|err| {
                TakanawaError::HttpProtocol(format!("invalid response header: {err}"))
            })
        })
        .transpose()
}

#[allow(clippy::needless_pass_by_value)]
fn map_reqwest_error(err: reqwest::Error) -> TakanawaError {
    if err.is_timeout() || err.is_connect() || err.is_request() || err.is_body() {
        TakanawaError::Network(err.to_string())
    } else {
        TakanawaError::HttpProtocol(err.to_string())
    }
}

struct ActiveIoGuard {
    state: SharedState,
}

impl ActiveIoGuard {
    fn new(state: SharedState) -> Self {
        state.increment_active_io();
        Self { state }
    }
}

impl Drop for ActiveIoGuard {
    fn drop(&mut self) {
        self.state.decrement_active_io();
    }
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};
    use std::net::{SocketAddr, TcpListener};
    use std::sync::Arc;
    use std::thread;
    use std::time::Duration;

    use sha2::{Digest, Sha256};
    use tempfile::TempDir;

    use super::*;
    use crate::DownloadPhase;
    use crate::limiter::DEFAULT_MAX_IO;

    #[tokio::test]
    async fn downloads_file_with_ranges() {
        let data = Arc::new(b"abcdefghijklmnopqrstuvwxyz".to_vec());
        let addr = spawn_range_server(Arc::clone(&data), false);
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("out.bin");
        let engine = DownloadEngine::new(DEFAULT_MAX_IO).unwrap();
        let config = DownloadConfig {
            url: format!("http://{addr}/file"),
            target_path: target.clone(),
            chunk_size: 5,
            parallelism: 2,
            hash: HashConfig::None,
        };

        let snapshot = download_to_completion(engine, config).await.unwrap();

        assert_eq!(snapshot.phase, DownloadPhase::Completed);
        assert_eq!(std::fs::read(target).unwrap(), data.as_slice());
    }

    #[tokio::test]
    async fn rejects_ignored_range() {
        let data = Arc::new(b"abcdef".to_vec());
        let addr = spawn_range_server(Arc::clone(&data), true);
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("out.bin");
        let engine = DownloadEngine::new(DEFAULT_MAX_IO).unwrap();
        let config = DownloadConfig {
            url: format!("http://{addr}/file"),
            target_path: target,
            chunk_size: 3,
            parallelism: 1,
            hash: HashConfig::None,
        };

        let err = download_to_completion(engine, config).await.unwrap_err();

        assert!(matches!(err, TakanawaError::HttpProtocol(_)));
    }

    #[tokio::test]
    async fn resumes_from_existing_part() {
        let data = Arc::new(b"abcdefghijklmnopqrstuvwxyz".to_vec());
        let addr = spawn_range_server(Arc::clone(&data), false);
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("out.bin");
        let remote = RemoteInfo {
            content_len: data.len() as u64,
            etag: None,
            last_modified: None,
        };
        let mut part = PartFile::open_or_create(
            &target,
            &format!("http://{addr}/file"),
            &remote,
            5,
            HashConfig::None,
        )
        .unwrap();
        part.write_chunk(0, &data[..5]).unwrap();
        drop(part);

        let engine = DownloadEngine::new(DEFAULT_MAX_IO).unwrap();
        let config = DownloadConfig {
            url: format!("http://{addr}/file"),
            target_path: target.clone(),
            chunk_size: 5,
            parallelism: 2,
            hash: HashConfig::None,
        };

        let snapshot = download_to_completion(engine, config).await.unwrap();

        assert_eq!(snapshot.phase, DownloadPhase::Completed);
        assert_eq!(std::fs::read(target).unwrap(), data.as_slice());
    }

    #[test]
    fn pause_discards_in_flight_chunk() {
        let data = Arc::new(b"abcdefghij".to_vec());
        let addr = spawn_delayed_chunk_server(Arc::clone(&data), Duration::from_millis(300));
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("out.bin");
        let engine = DownloadEngine::new(DEFAULT_MAX_IO).unwrap();
        let runtime = Runtime::new().unwrap();
        let download = DownloadHandle::new(
            engine,
            DownloadConfig {
                url: format!("http://{addr}/file"),
                target_path: target,
                chunk_size: 5,
                parallelism: 1,
                hash: HashConfig::None,
            },
        );

        download.start_on(&runtime).unwrap();
        thread::sleep(Duration::from_millis(100));
        download.pause().unwrap();

        let snapshot = wait_for_phase(&download, DownloadPhase::Paused);

        assert_eq!(snapshot.completed_chunks, 0);
        assert_eq!(snapshot.downloaded_bytes, 0);
    }

    #[tokio::test]
    async fn rejects_existing_target() {
        let data = Arc::new(b"abcdef".to_vec());
        let addr = spawn_range_server(Arc::clone(&data), false);
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("out.bin");
        std::fs::write(&target, b"already here").unwrap();
        let engine = DownloadEngine::new(DEFAULT_MAX_IO).unwrap();
        let config = DownloadConfig {
            url: format!("http://{addr}/file"),
            target_path: target,
            chunk_size: 3,
            parallelism: 1,
            hash: HashConfig::None,
        };

        let err = download_to_completion(engine, config).await.unwrap_err();

        assert!(matches!(err, TakanawaError::TargetExists(_)));
    }

    #[tokio::test]
    async fn verifies_sha256_before_finalize() {
        let data = Arc::new(b"abcdef".to_vec());
        let addr = spawn_range_server(Arc::clone(&data), false);
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("out.bin");
        let expected: [u8; 32] = Sha256::digest(data.as_slice()).into();
        let engine = DownloadEngine::new(DEFAULT_MAX_IO).unwrap();
        let config = DownloadConfig {
            url: format!("http://{addr}/file"),
            target_path: target.clone(),
            chunk_size: 3,
            parallelism: 1,
            hash: HashConfig::Sha256(expected),
        };

        download_to_completion(engine, config).await.unwrap();

        assert_eq!(std::fs::read(target).unwrap(), data.as_slice());
    }

    fn spawn_range_server(data: Arc<Vec<u8>>, ignore_range: bool) -> SocketAddr {
        spawn_range_server_with_chunk_delay(data, ignore_range, None)
    }

    fn spawn_delayed_chunk_server(data: Arc<Vec<u8>>, delay: Duration) -> SocketAddr {
        spawn_range_server_with_chunk_delay(data, false, Some(delay))
    }

    fn spawn_range_server_with_chunk_delay(
        data: Arc<Vec<u8>>,
        ignore_range: bool,
        chunk_delay: Option<Duration>,
    ) -> SocketAddr {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let data = Arc::clone(&data);
                thread::spawn(move || handle_connection(stream, &data, ignore_range, chunk_delay));
            }
        });
        addr
    }

    fn handle_connection(
        mut stream: std::net::TcpStream,
        data: &[u8],
        ignore_range: bool,
        chunk_delay: Option<Duration>,
    ) {
        let mut buffer = [0; 4096];
        let read = stream.read(&mut buffer).unwrap_or(0);
        let request = String::from_utf8_lossy(&buffer[..read]);
        let range = request.lines().find_map(|line| {
            let (name, value) = line.split_once(':')?;
            if name.eq_ignore_ascii_case("range") {
                value.trim().strip_prefix("bytes=")
            } else {
                None
            }
        });

        if ignore_range {
            let response = format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n", data.len());
            stream.write_all(response.as_bytes()).unwrap();
            stream.write_all(data).unwrap();
            return;
        }

        let Some(range) = range else {
            stream
                .write_all(b"HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\n\r\n")
                .unwrap();
            return;
        };
        let Some((start, end)) = range.split_once('-') else {
            stream
                .write_all(b"HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\n\r\n")
                .unwrap();
            return;
        };
        let start = start.parse::<usize>().unwrap();
        let end = end.parse::<usize>().unwrap();
        if start >= data.len() {
            let response = format!(
                "HTTP/1.1 416 Range Not Satisfiable\r\nContent-Range: bytes */{}\r\nContent-Length: 0\r\n\r\n",
                data.len()
            );
            stream.write_all(response.as_bytes()).unwrap();
            return;
        }
        let end = end.min(data.len() - 1);
        let body = &data[start..=end];
        if let Some(delay) = chunk_delay {
            if !(start == 0 && end == 0) {
                thread::sleep(delay);
            }
        }
        let response = format!(
            "HTTP/1.1 206 Partial Content\r\nContent-Range: bytes {start}-{end}/{}\r\nContent-Length: {}\r\nAccept-Ranges: bytes\r\n\r\n",
            data.len(),
            body.len()
        );
        stream.write_all(response.as_bytes()).unwrap();
        stream.write_all(body).unwrap();
    }

    fn wait_for_phase(download: &DownloadHandle, phase: DownloadPhase) -> DownloadSnapshot {
        for _ in 0..50 {
            let snapshot = download.snapshot();
            if snapshot.phase == phase {
                return snapshot;
            }
            thread::sleep(Duration::from_millis(20));
        }
        download.snapshot()
    }
}
