use std::ffi::OsString;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use fs2::FileExt;

use crate::chunk::{ChunkPlan, normalize_chunk_size};
use crate::metadata::{PartMetadata, RemoteInfo, slot_size_for};
use crate::{HashConfig, HashVerifier, Result, TakanawaError, hash_url};

/// Resumable on-disk part file with metadata slots and an exclusive lock.
#[derive(Debug)]
pub struct PartFile {
    file: File,
    lock_file: File,
    lock_path: PathBuf,
    part_path: PathBuf,
    slot_size: u64,
    active_slot: u8,
    metadata: PartMetadata,
}

impl PartFile {
    /// Opens an existing compatible part file or creates a new one.
    ///
    /// The target file must not already exist. The companion `.part.lock` file
    /// is locked for the lifetime of the returned value.
    ///
    /// # Errors
    ///
    /// Returns an error if the target exists, the part file is locked by
    /// another process, existing metadata is corrupt or incompatible, the part
    /// file size is unexpected, or filesystem operations fail.
    pub fn open_or_create(
        target_path: &Path,
        url: &str,
        remote: &RemoteInfo,
        chunk_size: u64,
        hash: HashConfig,
    ) -> Result<Self> {
        if target_path.exists() {
            return Err(TakanawaError::TargetExists(target_path.to_owned()));
        }

        let chunk_size = normalize_chunk_size(chunk_size)?;
        let part_path = part_path_for(target_path);
        let lock_path = part_lock_path_for(target_path);
        let lock_file = acquire_lock(&lock_path)?;
        let slot_size = slot_size_for(remote.content_len, chunk_size)?;
        let expected_len = remote
            .content_len
            .checked_add(slot_size.checked_mul(2).ok_or_else(|| {
                TakanawaError::InvalidConfig("part file length overflow".to_owned())
            })?)
            .ok_or_else(|| TakanawaError::InvalidConfig("part file length overflow".to_owned()))?;
        let url_hash = hash_url(url);

        if part_path.exists() {
            let mut file = OpenOptions::new().read(true).write(true).open(&part_path)?;
            let actual_len = file.metadata()?.len();
            if actual_len != expected_len {
                return Err(TakanawaError::PartSizeMismatch {
                    expected: expected_len,
                    actual: actual_len,
                });
            }

            let (metadata, active_slot) =
                read_best_metadata(&mut file, remote.content_len, slot_size)?;
            metadata.ensure_compatible(url_hash, remote, chunk_size, hash)?;
            return Ok(Self {
                file,
                lock_file,
                lock_path,
                part_path,
                slot_size,
                active_slot,
                metadata,
            });
        }

        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(&part_path)?;
        file.set_len(expected_len)?;

        let metadata = PartMetadata::new(url_hash, remote, chunk_size, hash)?;
        let slot = metadata.encode_slot(slot_size)?;
        file.seek(SeekFrom::Start(remote.content_len))?;
        file.write_all(&slot)?;
        file.sync_all()?;

        Ok(Self {
            file,
            lock_file,
            lock_path,
            part_path,
            slot_size,
            active_slot: 0,
            metadata,
        })
    }

    #[must_use]
    /// Returns the current part metadata.
    pub const fn metadata(&self) -> &PartMetadata {
        &self.metadata
    }

    #[must_use]
    /// Returns indexes of chunks that still need to be downloaded.
    pub fn incomplete_chunks(&self) -> Vec<u64> {
        self.metadata.bitmap.incomplete_indices()
    }

    /// Writes and commits a complete chunk.
    ///
    /// Already completed chunks are ignored.
    ///
    /// # Errors
    ///
    /// Returns an error if `index` is outside the chunk plan, `bytes` does not
    /// exactly match the chunk length, metadata cannot be updated, or I/O fails.
    pub fn write_chunk(&mut self, index: u64, bytes: &[u8]) -> Result<()> {
        let plan = ChunkPlan::new(self.metadata.content_len, self.metadata.chunk_size)?;
        let chunk = plan.chunk(index)?;
        if bytes.len() != usize::try_from(chunk.len).unwrap_or(usize::MAX) {
            return Err(TakanawaError::HttpProtocol(format!(
                "chunk {index} length mismatch: expected {}, got {}",
                chunk.len,
                bytes.len()
            )));
        }
        if self.metadata.bitmap.is_complete(index)? {
            return Ok(());
        }

        self.write_chunk_bytes(index, 0, bytes)?;
        self.commit_chunk(index)
    }

    /// Writes bytes into a chunk without marking the chunk complete.
    ///
    /// This supports streaming partial responses. Call [`Self::commit_chunk`]
    /// only after the full chunk has been written.
    ///
    /// # Errors
    ///
    /// Returns an error if the chunk index or write range is invalid, the byte
    /// length cannot fit in file offsets, or I/O fails.
    pub fn write_chunk_bytes(&mut self, index: u64, chunk_offset: u64, bytes: &[u8]) -> Result<()> {
        let plan = ChunkPlan::new(self.metadata.content_len, self.metadata.chunk_size)?;
        let chunk = plan.chunk(index)?;
        let len = u64::try_from(bytes.len()).map_err(|_| {
            TakanawaError::InvalidConfig(format!(
                "chunk {index} write length does not fit in file offsets"
            ))
        })?;
        let end = chunk_offset.checked_add(len).ok_or_else(|| {
            TakanawaError::InvalidConfig(format!("chunk {index} write offset overflow"))
        })?;
        if end > chunk.len {
            return Err(TakanawaError::InvalidConfig(format!(
                "chunk {index} write range {chunk_offset}..{end} exceeds chunk length {}",
                chunk.len
            )));
        }
        if bytes.is_empty() || self.metadata.bitmap.is_complete(index)? {
            return Ok(());
        }

        self.file
            .seek(SeekFrom::Start(chunk.start + chunk_offset))?;
        self.file.write_all(bytes)?;
        Ok(())
    }

    /// Marks a previously written chunk complete and persists metadata.
    ///
    /// Already completed chunks are ignored.
    ///
    /// # Errors
    ///
    /// Returns an error if `index` is outside the chunk plan, metadata
    /// generation overflows, metadata cannot be encoded, or I/O fails.
    pub fn commit_chunk(&mut self, index: u64) -> Result<()> {
        let plan = ChunkPlan::new(self.metadata.content_len, self.metadata.chunk_size)?;
        let _chunk = plan.chunk(index)?;
        if self.metadata.bitmap.is_complete(index)? {
            return Ok(());
        }

        self.file.sync_data()?;

        self.metadata.bitmap.mark_complete(index)?;
        self.commit_metadata()
    }

    /// Verifies and promotes the part file to the final target path.
    ///
    /// This consumes the part file, truncates away metadata slots, renames the
    /// `.part` file to `target_path`, and releases the lock.
    ///
    /// # Errors
    ///
    /// Returns an error if the target exists, not all chunks are complete, hash
    /// verification fails, or filesystem operations fail.
    pub fn finalize(mut self, target_path: &Path) -> Result<()> {
        if target_path.exists() {
            return Err(TakanawaError::TargetExists(target_path.to_owned()));
        }
        if !self.metadata.all_complete() {
            return Err(TakanawaError::InvalidConfig(
                "cannot finalize an incomplete part file".to_owned(),
            ));
        }

        if !self.verify_hash()? {
            return Err(TakanawaError::HashMismatch);
        }

        let PartFile {
            file,
            lock_file,
            lock_path,
            part_path,
            metadata,
            ..
        } = self;
        file.set_len(metadata.content_len)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&part_path, target_path)?;
        sync_parent_dir(target_path);
        drop(lock_file);
        let _ = fs::remove_file(lock_path);
        Ok(())
    }

    fn commit_metadata(&mut self) -> Result<()> {
        self.metadata.generation = self.metadata.generation.checked_add(1).ok_or_else(|| {
            TakanawaError::InvalidConfig("metadata generation overflow".to_owned())
        })?;
        self.active_slot = (self.metadata.generation % 2) as u8;
        let slot = self.metadata.encode_slot(self.slot_size)?;
        let offset = self.metadata.content_len + u64::from(self.active_slot) * self.slot_size;
        self.file.seek(SeekFrom::Start(offset))?;
        self.file.write_all(&slot)?;
        self.file.sync_all()?;
        Ok(())
    }

    fn verify_hash(&mut self) -> Result<bool> {
        let Some(mut verifier) = HashVerifier::new(self.metadata.hash) else {
            return Ok(true);
        };

        let mut remaining = self.metadata.content_len;
        let mut buffer = vec![0; 1024 * 1024];
        self.file.seek(SeekFrom::Start(0))?;
        while remaining > 0 {
            let read_len = usize::try_from(remaining.min(buffer.len() as u64))
                .expect("bounded by buffer length");
            self.file.read_exact(&mut buffer[..read_len])?;
            verifier.update(&buffer[..read_len]);
            remaining -= read_len as u64;
        }
        Ok(verifier.finish())
    }
}

#[must_use]
/// Returns the companion `.part` path for a target file.
pub fn part_path_for(target_path: &Path) -> PathBuf {
    let mut value: OsString = target_path.as_os_str().to_owned();
    value.push(".part");
    PathBuf::from(value)
}

#[must_use]
/// Returns the companion `.part.lock` path for a target file.
pub fn part_lock_path_for(target_path: &Path) -> PathBuf {
    let mut value: OsString = target_path.as_os_str().to_owned();
    value.push(".part.lock");
    PathBuf::from(value)
}

fn acquire_lock(lock_path: &Path) -> Result<File> {
    let lock_file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(lock_path)?;
    match lock_file.try_lock_exclusive() {
        Ok(()) => Ok(lock_file),
        Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
            Err(TakanawaError::PartBusy(lock_path.to_owned()))
        }
        Err(err) => Err(TakanawaError::Io(err)),
    }
}

fn read_best_metadata(
    file: &mut File,
    content_len: u64,
    slot_size: u64,
) -> Result<(PartMetadata, u8)> {
    let slot_len = usize::try_from(slot_size)
        .map_err(|_| TakanawaError::PartCorrupt("slot size overflow".to_owned()))?;
    let mut slots = Vec::new();

    for slot_index in 0..2_u8 {
        let offset = content_len + u64::from(slot_index) * slot_size;
        let mut buffer = vec![0; slot_len];
        file.seek(SeekFrom::Start(offset))?;
        file.read_exact(&mut buffer)?;
        if let Ok(metadata) = PartMetadata::decode_slot(&buffer) {
            slots.push((metadata, slot_index));
        }
    }

    slots
        .into_iter()
        .max_by_key(|(metadata, _)| metadata.generation)
        .ok_or_else(|| TakanawaError::PartCorrupt("no valid metadata slot found".to_owned()))
}

#[cfg(unix)]
fn sync_parent_dir(target_path: &Path) {
    if let Some(parent) = target_path.parent() {
        if let Ok(dir) = File::open(parent) {
            let _ = dir.sync_all();
        }
    }
}

#[cfg(not(unix))]
fn sync_parent_dir(_target_path: &Path) {}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::*;

    fn hex_array<const N: usize>(value: impl AsRef<str>) -> [u8; N] {
        hex::decode(value.as_ref()).unwrap().try_into().unwrap()
    }

    fn remote(content_len: u64) -> RemoteInfo {
        RemoteInfo {
            content_len,
            etag: Some("etag".to_owned()),
            last_modified: Some("now".to_owned()),
        }
    }

    #[test]
    fn resumes_valid_part() {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("file.bin");
        {
            let mut part = PartFile::open_or_create(
                &target,
                "https://example.test/file",
                &remote(6),
                3,
                HashConfig::None,
            )
            .unwrap();
            part.write_chunk(0, b"abc").unwrap();
        }

        let part = PartFile::open_or_create(
            &target,
            "https://example.test/file",
            &remote(6),
            3,
            HashConfig::None,
        )
        .unwrap();

        assert_eq!(part.metadata().completed_chunks(), 1);
        assert_eq!(part.incomplete_chunks(), vec![1]);
    }

    #[test]
    fn partial_chunk_write_is_not_committed_on_reopen() {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("file.bin");
        {
            let mut part = PartFile::open_or_create(
                &target,
                "https://example.test/file",
                &remote(6),
                3,
                HashConfig::None,
            )
            .unwrap();
            part.write_chunk_bytes(0, 0, b"ab").unwrap();
        }

        let part = PartFile::open_or_create(
            &target,
            "https://example.test/file",
            &remote(6),
            3,
            HashConfig::None,
        )
        .unwrap();

        assert_eq!(part.metadata().completed_chunks(), 0);
        assert_eq!(part.incomplete_chunks(), vec![0, 1]);
    }

    #[test]
    fn partial_chunk_can_be_overwritten_and_committed() {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("file.bin");
        let mut part = PartFile::open_or_create(
            &target,
            "https://example.test/file",
            &remote(6),
            3,
            HashConfig::None,
        )
        .unwrap();
        part.write_chunk_bytes(0, 0, b"xx").unwrap();
        part.write_chunk_bytes(0, 0, b"abc").unwrap();
        part.commit_chunk(0).unwrap();
        part.write_chunk(1, b"def").unwrap();
        part.finalize(&target).unwrap();

        assert_eq!(fs::read(&target).unwrap(), b"abcdef");
    }

    #[test]
    fn partial_chunk_write_rejects_out_of_bounds_ranges() {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("file.bin");
        let mut part = PartFile::open_or_create(
            &target,
            "https://example.test/file",
            &remote(6),
            3,
            HashConfig::None,
        )
        .unwrap();

        let err = part.write_chunk_bytes(0, 2, b"bc").unwrap_err();

        assert!(matches!(err, TakanawaError::InvalidConfig(_)));
    }

    #[test]
    fn rejects_part_size_mismatch() {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("file.bin");
        let part_path = part_path_for(&target);
        fs::write(&part_path, b"too short").unwrap();

        let err = PartFile::open_or_create(
            &target,
            "https://example.test/file",
            &remote(6),
            3,
            HashConfig::None,
        )
        .unwrap_err();

        assert!(matches!(err, TakanawaError::PartSizeMismatch { .. }));
    }

    #[test]
    fn finalizes_with_supported_hashes() {
        let cases = [
            HashConfig::Sha1(hex_array::<20>("a9993e364706816aba3e25717850c26c9cd0d89d")),
            HashConfig::Sha256(hex_array::<32>(
                "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
            )),
            HashConfig::Sha512(hex_array::<64>(concat!(
                "ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a",
                "2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f",
            ))),
            HashConfig::Md5(hex_array::<16>("900150983cd24fb0d6963f7d28e17f72")),
            HashConfig::Crc32(hex_array::<4>("352441c2")),
        ];

        for hash in cases {
            let dir = TempDir::new().unwrap();
            let target = dir.path().join("file.bin");
            let mut part =
                PartFile::open_or_create(&target, "https://example.test/file", &remote(3), 3, hash)
                    .unwrap();
            part.write_chunk(0, b"abc").unwrap();
            part.finalize(&target).unwrap();

            assert_eq!(fs::read(&target).unwrap(), b"abc");
        }
    }

    #[test]
    fn finalizes_and_strips_metadata() {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("file.bin");
        let mut part = PartFile::open_or_create(
            &target,
            "https://example.test/file",
            &remote(6),
            3,
            HashConfig::None,
        )
        .unwrap();
        part.write_chunk(1, b"def").unwrap();
        part.write_chunk(0, b"abc").unwrap();
        part.finalize(&target).unwrap();

        assert_eq!(fs::read(&target).unwrap(), b"abcdef");
        assert!(!part_path_for(&target).exists());
    }
}
