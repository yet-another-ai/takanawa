use std::ffi::OsStr;
use std::fs;
use std::io::{ErrorKind, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::{env, thread, time};

pub(crate) type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

const TEST_HTTP_PAYLOAD: &[u8] = b"takanawa integration fixture payload\n";
const SUPPORTED_WINDOWS_TARGETS: &[&str] = &["x86_64-pc-windows-msvc", "aarch64-pc-windows-msvc"];

pub(crate) fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask manifest should live under repo root")
        .to_path_buf()
}

pub(crate) fn repo_command(program: impl AsRef<OsStr>) -> Command {
    let mut command = Command::new(program);
    command.current_dir(repo_root());
    command
}

pub(crate) struct TestHttpServer {
    addr: SocketAddr,
    shutdown: Arc<AtomicBool>,
    thread: Option<thread::JoinHandle<()>>,
}

impl TestHttpServer {
    pub(crate) fn start() -> Result<Self> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        listener.set_nonblocking(true)?;
        let addr = listener.local_addr()?;
        let payload = Arc::new(TEST_HTTP_PAYLOAD.to_vec());
        let shutdown = Arc::new(AtomicBool::new(false));
        let thread_shutdown = Arc::clone(&shutdown);
        let thread = thread::spawn(move || {
            while !thread_shutdown.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((stream, _)) => {
                        let payload = Arc::clone(&payload);
                        thread::spawn(move || handle_test_http_connection(stream, &payload));
                    }
                    Err(error) if error.kind() == ErrorKind::WouldBlock => {
                        thread::sleep(time::Duration::from_millis(10));
                    }
                    Err(_) => break,
                }
            }
        });

        Ok(Self {
            addr,
            shutdown,
            thread: Some(thread),
        })
    }

    fn url(&self) -> String {
        format!("http://{}/file", self.addr)
    }

    pub(crate) fn configure_command(&self, command: &mut Command) {
        command.env("TAKANAWA_TEST_URL", self.url());
        command.env(
            "TAKANAWA_TEST_EXPECTED_BYTES",
            String::from_utf8_lossy(TEST_HTTP_PAYLOAD).as_ref(),
        );
    }
}

impl Drop for TestHttpServer {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn handle_test_http_connection(mut stream: TcpStream, payload: &[u8]) {
    let mut buffer = [0; 4096];
    let read = stream.read(&mut buffer).unwrap_or(0);
    let request = String::from_utf8_lossy(&buffer[..read]);
    let Some((start, end)) = request_range(&request) else {
        let _ = stream.write_all(
            b"HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
        );
        return;
    };

    if start >= payload.len() {
        let response = format!(
            "HTTP/1.1 416 Range Not Satisfiable\r\nContent-Range: bytes */{}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            payload.len()
        );
        let _ = stream.write_all(response.as_bytes());
        return;
    }

    let end = end.min(payload.len() - 1);
    let body = &payload[start..=end];
    let response = format!(
        "HTTP/1.1 206 Partial Content\r\nContent-Range: bytes {start}-{end}/{}\r\nContent-Length: {}\r\nAccept-Ranges: bytes\r\nConnection: close\r\n\r\n",
        payload.len(),
        body.len()
    );
    let _ = stream.write_all(response.as_bytes());
    let _ = stream.write_all(body);
}

fn request_range(request: &str) -> Option<(usize, usize)> {
    let range = request.lines().find_map(|line| {
        let (name, value) = line.split_once(':')?;
        if name.eq_ignore_ascii_case("range") {
            value.trim().strip_prefix("bytes=")
        } else {
            None
        }
    })?;
    let (start, end) = range.split_once('-')?;
    Some((start.parse().ok()?, end.parse().ok()?))
}

pub(crate) fn ensure_supported_windows_target(target: &str) -> Result<()> {
    if SUPPORTED_WINDOWS_TARGETS.contains(&target) {
        return Ok(());
    }

    Err(format!(
        "{target} is not a supported Windows target; supported targets are {}",
        SUPPORTED_WINDOWS_TARGETS.join(", ")
    )
    .into())
}

pub(crate) fn run_command(command: &mut Command) -> Result<()> {
    let debug = format!("{command:?}");
    let status = command
        .status()
        .map_err(|error| format!("{debug} failed to start: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{debug} exited with {status}").into())
    }
}

pub(crate) fn output_text(command: &mut Command) -> Result<String> {
    let debug = format!("{command:?}");
    let output = command
        .output()
        .map_err(|error| format!("{debug} failed to start: {error}"))?;
    if !output.status.success() {
        return Err(format!("{debug} exited with {}", output.status).into());
    }
    Ok(String::from_utf8(output.stdout)?.trim().to_owned())
}

pub(crate) fn ensure_dir(path: impl AsRef<Path>) -> Result<()> {
    fs::create_dir_all(repo_root().join(path))?;
    Ok(())
}

pub(crate) fn remove_dir_if_exists(path: impl AsRef<Path>) -> Result<()> {
    let path = repo_root().join(path);
    if path.is_dir() {
        fs::remove_dir_all(path)?;
    }
    Ok(())
}

pub(crate) fn copy_file(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> Result<()> {
    let root = repo_root();
    let src = root.join(src);
    let dst = root.join(dst);
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::copy(src, dst)?;
    Ok(())
}

pub(crate) fn copy_file_if_exists(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> Result<()> {
    let root = repo_root();
    let src = root.join(src);
    if src.is_file() {
        let dst = root.join(dst);
        if let Some(parent) = dst.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(src, dst)?;
    }
    Ok(())
}

pub(crate) fn deployment_env(mut command: Command) -> Command {
    command.env(
        "IPHONEOS_DEPLOYMENT_TARGET",
        env::var("IPHONEOS_DEPLOYMENT_TARGET").unwrap_or_else(|_| "13.0".to_owned()),
    );
    command.env(
        "IPHONESIMULATOR_DEPLOYMENT_TARGET",
        env::var("IPHONESIMULATOR_DEPLOYMENT_TARGET").unwrap_or_else(|_| "13.0".to_owned()),
    );
    command.env(
        "MACOSX_DEPLOYMENT_TARGET",
        env::var("MACOSX_DEPLOYMENT_TARGET").unwrap_or_else(|_| "10.15".to_owned()),
    );
    command
}

pub(crate) fn prepend_dynamic_library_path(command: &mut Command, native_dir: &Path) {
    let path = env::var_os("PATH").unwrap_or_default();
    let mut paths = env::split_paths(&path).collect::<Vec<_>>();
    paths.insert(0, native_dir.to_path_buf());
    if let Ok(joined) = env::join_paths(paths) {
        command.env("PATH", joined);
    }

    if cfg!(target_os = "macos") {
        prepend_env_path(command, "DYLD_LIBRARY_PATH", native_dir);
    } else if cfg!(target_os = "linux") {
        prepend_env_path(command, "LD_LIBRARY_PATH", native_dir);
    }
}

fn prepend_env_path(command: &mut Command, name: &str, native_dir: &Path) {
    let mut paths = env::var_os(name)
        .map(|value| env::split_paths(&value).collect::<Vec<_>>())
        .unwrap_or_default();
    paths.insert(0, native_dir.to_path_buf());
    if let Ok(joined) = env::join_paths(paths) {
        command.env(name, joined);
    }
}

pub(crate) fn copy_dir(src: &Path, dst: &Path) -> Result<()> {
    if !src.is_dir() {
        return Err(format!("missing directory {}", src.display()).into());
    }
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let source_path = entry.path();
        let target_path = dst.join(entry.file_name());
        let metadata = fs::symlink_metadata(&source_path)?;
        if metadata.is_dir() {
            copy_dir(&source_path, &target_path)?;
        } else if metadata.is_file() {
            if let Some(parent) = target_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(&source_path, &target_path)?;
        } else if metadata.file_type().is_symlink() {
            copy_symlink(&source_path, &target_path)?;
        }
    }
    Ok(())
}

#[cfg(unix)]
fn copy_symlink(src: &Path, dst: &Path) -> Result<()> {
    use std::os::unix::fs::symlink;

    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent)?;
    }
    if dst.exists() {
        fs::remove_file(dst)?;
    }
    symlink(fs::read_link(src)?, dst)?;
    Ok(())
}

#[cfg(windows)]
fn copy_symlink(src: &Path, dst: &Path) -> Result<()> {
    if src.is_dir() {
        copy_dir(src, dst)
    } else {
        fs::copy(src, dst)?;
        Ok(())
    }
}
