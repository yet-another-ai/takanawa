use crc32fast::Hasher;

use crate::bitmap::{ChunkBitmap, bitmap_len};
use crate::chunk::{chunk_count_for, normalize_chunk_size};
use crate::{HashConfig, Result, TakanawaError};

/// Current on-disk part metadata version.
pub const METADATA_VERSION: u16 = 2;

const MAGIC: &[u8; 8] = b"TKNWPRT1";
const HEADER_LEN: usize = 256;
const ALIGNMENT: u64 = 4096;
const ETAG_CAPACITY: usize = 512;
const LAST_MODIFIED_CAPACITY: usize = 128;

const VERSION_OFFSET: usize = 8;
const HEADER_LEN_OFFSET: usize = 10;
const CRC_OFFSET: usize = 12;
const GENERATION_OFFSET: usize = 16;
const CONTENT_LEN_OFFSET: usize = 24;
const CHUNK_SIZE_OFFSET: usize = 32;
const CHUNK_COUNT_OFFSET: usize = 40;
const BITMAP_LEN_OFFSET: usize = 48;
const URL_HASH_OFFSET: usize = 56;
const HASH_KIND_OFFSET: usize = 88;
const HASH_LEN_OFFSET: usize = 89;
const EXPECTED_HASH_OFFSET: usize = 92;
const EXPECTED_HASH_CAPACITY: usize = 64;
const ETAG_LEN_OFFSET: usize = 156;
const LAST_MODIFIED_LEN_OFFSET: usize = 158;
const SLOT_SIZE_OFFSET: usize = 160;

const V1_ETAG_LEN_OFFSET: usize = 124;
const V1_LAST_MODIFIED_LEN_OFFSET: usize = 126;
const V1_SLOT_SIZE_OFFSET: usize = 128;

/// Remote resource properties captured before a download starts or resumes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteInfo {
    /// Total length of the remote resource in bytes.
    pub content_len: u64,
    /// Remote `ETag` validator, when supplied by the server.
    pub etag: Option<String>,
    /// Remote `Last-Modified` validator, when supplied by the server.
    pub last_modified: Option<String>,
}

/// Persisted state for a resumable part file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartMetadata {
    /// Monotonic generation used to choose the newest metadata slot.
    pub generation: u64,
    /// SHA-256 hash of the source URL.
    pub url_hash: [u8; 32],
    /// Total content length in bytes.
    pub content_len: u64,
    /// Normalized chunk size in bytes.
    pub chunk_size: u64,
    /// Number of chunks in the resource.
    pub chunk_count: u64,
    /// Completion bitmap for all chunks.
    pub bitmap: ChunkBitmap,
    /// Stored `ETag` validator, when available.
    pub etag: Option<String>,
    /// Stored `Last-Modified` validator, when available.
    pub last_modified: Option<String>,
    /// Optional final-file hash verification configuration.
    pub hash: HashConfig,
}

impl PartMetadata {
    /// Creates metadata for a new part file.
    ///
    /// Passing `0` for `chunk_size` selects the default chunk size.
    ///
    /// # Errors
    ///
    /// Returns an error if the chunk size is invalid, validator headers are too
    /// large for the metadata slot, or the completion bitmap cannot be sized.
    pub fn new(
        url_hash: [u8; 32],
        remote: &RemoteInfo,
        chunk_size: u64,
        hash: HashConfig,
    ) -> Result<Self> {
        validate_header_len(remote.etag.as_deref(), ETAG_CAPACITY, "ETag")?;
        validate_header_len(
            remote.last_modified.as_deref(),
            LAST_MODIFIED_CAPACITY,
            "Last-Modified",
        )?;

        let chunk_size = normalize_chunk_size(chunk_size)?;
        let chunk_count = chunk_count_for(remote.content_len, chunk_size);
        Ok(Self {
            generation: 0,
            url_hash,
            content_len: remote.content_len,
            chunk_size,
            chunk_count,
            bitmap: ChunkBitmap::new(chunk_count)?,
            etag: remote.etag.clone(),
            last_modified: remote.last_modified.clone(),
            hash,
        })
    }

    #[must_use]
    /// Returns the number of completed chunks.
    pub fn completed_chunks(&self) -> u64 {
        self.bitmap.complete_count()
    }

    #[must_use]
    /// Returns the number of bytes represented by completed chunks.
    pub fn completed_bytes(&self) -> u64 {
        if self.chunk_count == 0 {
            return 0;
        }

        let full_chunks_before_last = self.chunk_count.saturating_sub(1);
        let mut bytes = 0_u64;
        for index in 0..full_chunks_before_last {
            if self.bitmap.is_complete(index).unwrap_or(false) {
                bytes += self.chunk_size;
            }
        }

        if self
            .bitmap
            .is_complete(self.chunk_count - 1)
            .unwrap_or(false)
        {
            let last_start = full_chunks_before_last * self.chunk_size;
            bytes += self.content_len - last_start;
        }

        bytes
    }

    #[must_use]
    /// Returns whether every chunk is complete.
    pub fn all_complete(&self) -> bool {
        self.bitmap.all_complete()
    }

    /// Verifies that existing metadata still matches a remote resource and caller configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if the URL hash, content length, available validators,
    /// chunk size, or hash configuration differ from the stored metadata.
    pub fn ensure_compatible(
        &self,
        url_hash: [u8; 32],
        remote: &RemoteInfo,
        chunk_size: u64,
        hash: HashConfig,
    ) -> Result<()> {
        let chunk_size = normalize_chunk_size(chunk_size)?;
        if self.url_hash != url_hash {
            return Err(TakanawaError::RemoteChanged(
                "URL does not match part metadata".to_owned(),
            ));
        }
        if self.content_len != remote.content_len {
            return Err(TakanawaError::RemoteChanged(format!(
                "content length changed from {} to {}",
                self.content_len, remote.content_len
            )));
        }
        if let (Some(stored), Some(current)) = (&self.etag, &remote.etag) {
            if stored != current {
                return Err(TakanawaError::RemoteChanged(format!(
                    "ETag changed from {stored} to {current}"
                )));
            }
        }
        if let (Some(stored), Some(current)) = (&self.last_modified, &remote.last_modified) {
            if stored != current {
                return Err(TakanawaError::RemoteChanged(format!(
                    "Last-Modified changed from {stored} to {current}"
                )));
            }
        }
        if self.chunk_size != chunk_size {
            return Err(TakanawaError::RemoteChanged(format!(
                "chunk size changed from {} to {chunk_size}",
                self.chunk_size
            )));
        }
        if self.hash != hash {
            return Err(TakanawaError::RemoteChanged(
                "hash configuration changed".to_owned(),
            ));
        }
        Ok(())
    }

    /// Encodes this metadata into a fixed-size metadata slot.
    ///
    /// # Errors
    ///
    /// Returns an error if `slot_size` is invalid for this metadata, if header
    /// values exceed their fixed capacities, or if supporting lengths overflow.
    ///
    /// # Panics
    ///
    /// Panics only if fixed metadata constants no longer fit their encoded
    /// integer fields.
    pub fn encode_slot(&self, slot_size: u64) -> Result<Vec<u8>> {
        let slot_len = usize::try_from(slot_size)
            .map_err(|_| TakanawaError::InvalidConfig("slot size overflow".to_owned()))?;
        if slot_len < HEADER_LEN {
            return Err(TakanawaError::InvalidConfig(
                "slot size is smaller than metadata header".to_owned(),
            ));
        }

        let bitmap_len = bitmap_len(self.chunk_count)?;
        let expected_size = slot_size_for(self.content_len, self.chunk_size)?;
        if expected_size != slot_size {
            return Err(TakanawaError::InvalidConfig(format!(
                "slot size mismatch: expected {expected_size}, got {slot_size}"
            )));
        }

        validate_header_len(self.etag.as_deref(), ETAG_CAPACITY, "ETag")?;
        validate_header_len(
            self.last_modified.as_deref(),
            LAST_MODIFIED_CAPACITY,
            "Last-Modified",
        )?;

        let mut slot = vec![0; slot_len];
        slot[0..8].copy_from_slice(MAGIC);
        write_u16(&mut slot, VERSION_OFFSET, METADATA_VERSION);
        write_u16(
            &mut slot,
            HEADER_LEN_OFFSET,
            u16::try_from(HEADER_LEN)
                .expect("metadata header length is a fixed value that fits u16"),
        );
        write_u64(&mut slot, GENERATION_OFFSET, self.generation);
        write_u64(&mut slot, CONTENT_LEN_OFFSET, self.content_len);
        write_u64(&mut slot, CHUNK_SIZE_OFFSET, self.chunk_size);
        write_u64(&mut slot, CHUNK_COUNT_OFFSET, self.chunk_count);
        write_u64(
            &mut slot,
            BITMAP_LEN_OFFSET,
            u64::try_from(bitmap_len).expect("bitmap length fits in u64"),
        );
        slot[URL_HASH_OFFSET..URL_HASH_OFFSET + 32].copy_from_slice(&self.url_hash);
        slot[HASH_KIND_OFFSET] = u8::from(self.hash.kind());
        if let Some(hash) = self.hash.expected_bytes() {
            slot[HASH_LEN_OFFSET] =
                u8::try_from(hash.len()).expect("supported hash lengths fit in one byte");
            slot[EXPECTED_HASH_OFFSET..EXPECTED_HASH_OFFSET + hash.len()].copy_from_slice(&hash);
        }
        write_u16(
            &mut slot,
            ETAG_LEN_OFFSET,
            u16::try_from(self.etag.as_deref().map_or(0, str::len))
                .expect("ETag length was validated to fit u16"),
        );
        write_u16(
            &mut slot,
            LAST_MODIFIED_LEN_OFFSET,
            u16::try_from(self.last_modified.as_deref().map_or(0, str::len))
                .expect("Last-Modified length was validated to fit u16"),
        );
        write_u64(&mut slot, SLOT_SIZE_OFFSET, slot_size);

        let mut cursor = HEADER_LEN;
        slot[cursor..cursor + bitmap_len].copy_from_slice(self.bitmap.as_bytes());
        cursor += bitmap_len;
        write_fixed_string(
            &mut slot[cursor..cursor + ETAG_CAPACITY],
            self.etag.as_deref(),
        );
        cursor += ETAG_CAPACITY;
        write_fixed_string(
            &mut slot[cursor..cursor + LAST_MODIFIED_CAPACITY],
            self.last_modified.as_deref(),
        );

        let crc = checksum_slot(&slot);
        write_u32(&mut slot, CRC_OFFSET, crc);
        Ok(slot)
    }

    /// Decodes metadata from a fixed-size metadata slot.
    ///
    /// # Errors
    ///
    /// Returns an error if the slot is too short, has an unsupported version,
    /// fails CRC validation, has inconsistent lengths, or contains invalid
    /// UTF-8/header/hash data.
    pub fn decode_slot(slot: &[u8]) -> Result<Self> {
        let decoded = decode_slot_header(slot)?;

        let mut url_hash = [0; 32];
        url_hash.copy_from_slice(&slot[URL_HASH_OFFSET..URL_HASH_OFFSET + 32]);

        let hash = decode_hash(slot, decoded.offsets)?;

        let etag_len = usize::from(read_u16(slot, decoded.offsets.etag_len)?);
        let last_modified_len = usize::from(read_u16(slot, decoded.offsets.last_modified_len)?);
        if etag_len > ETAG_CAPACITY || last_modified_len > LAST_MODIFIED_CAPACITY {
            return Err(TakanawaError::PartCorrupt(
                "stored header length exceeds fixed capacity".to_owned(),
            ));
        }

        let mut cursor = HEADER_LEN;
        let bitmap = ChunkBitmap::from_bytes(
            decoded.chunk_count,
            slot[cursor..cursor + decoded.bitmap_len].to_vec(),
        )?;
        cursor += decoded.bitmap_len;
        let etag = decode_optional_string(&slot[cursor..cursor + ETAG_CAPACITY], etag_len)?;
        cursor += ETAG_CAPACITY;
        let last_modified = decode_optional_string(
            &slot[cursor..cursor + LAST_MODIFIED_CAPACITY],
            last_modified_len,
        )?;

        Ok(Self {
            generation: decoded.generation,
            url_hash,
            content_len: decoded.content_len,
            chunk_size: decoded.chunk_size,
            chunk_count: decoded.chunk_count,
            bitmap,
            etag,
            last_modified,
            hash,
        })
    }
}

#[derive(Debug, Clone, Copy)]
struct MetadataOffsets {
    etag_len: usize,
    last_modified_len: usize,
    slot_size: usize,
    expected_hash_capacity: usize,
}

impl MetadataOffsets {
    const fn for_version(version: u16) -> Self {
        if version == 1 {
            Self {
                etag_len: V1_ETAG_LEN_OFFSET,
                last_modified_len: V1_LAST_MODIFIED_LEN_OFFSET,
                slot_size: V1_SLOT_SIZE_OFFSET,
                expected_hash_capacity: 32,
            }
        } else {
            Self {
                etag_len: ETAG_LEN_OFFSET,
                last_modified_len: LAST_MODIFIED_LEN_OFFSET,
                slot_size: SLOT_SIZE_OFFSET,
                expected_hash_capacity: EXPECTED_HASH_CAPACITY,
            }
        }
    }
}

struct DecodedSlotHeader {
    offsets: MetadataOffsets,
    generation: u64,
    content_len: u64,
    chunk_size: u64,
    chunk_count: u64,
    bitmap_len: usize,
}

fn decode_slot_header(slot: &[u8]) -> Result<DecodedSlotHeader> {
    if slot.len() < HEADER_LEN {
        return Err(TakanawaError::PartCorrupt(
            "metadata slot is shorter than header".to_owned(),
        ));
    }
    if &slot[0..8] != MAGIC {
        return Err(TakanawaError::PartCorrupt(
            "metadata magic mismatch".to_owned(),
        ));
    }

    let version = read_u16(slot, VERSION_OFFSET)?;
    if version != 1 && version != METADATA_VERSION {
        return Err(TakanawaError::PartCorrupt(format!(
            "unsupported metadata version {version}"
        )));
    }
    let offsets = MetadataOffsets::for_version(version);
    let header_len = usize::from(read_u16(slot, HEADER_LEN_OFFSET)?);
    if header_len != HEADER_LEN {
        return Err(TakanawaError::PartCorrupt(format!(
            "unexpected metadata header length {header_len}"
        )));
    }

    verify_slot_crc(slot)?;

    let generation = read_u64(slot, GENERATION_OFFSET)?;
    let content_len = read_u64(slot, CONTENT_LEN_OFFSET)?;
    let chunk_size = read_u64(slot, CHUNK_SIZE_OFFSET)?;
    let chunk_count = read_u64(slot, CHUNK_COUNT_OFFSET)?;
    let bitmap_len = decode_bitmap_len(slot, chunk_count)?;
    verify_slot_size(slot, offsets, content_len, chunk_size)?;

    Ok(DecodedSlotHeader {
        offsets,
        generation,
        content_len,
        chunk_size,
        chunk_count,
        bitmap_len,
    })
}

fn verify_slot_crc(slot: &[u8]) -> Result<()> {
    let stored_crc = read_u32(slot, CRC_OFFSET)?;
    let actual_crc = checksum_slot(slot);
    if stored_crc != actual_crc {
        return Err(TakanawaError::PartCorrupt(format!(
            "metadata CRC mismatch: expected {stored_crc:#010x}, got {actual_crc:#010x}"
        )));
    }
    Ok(())
}

fn decode_bitmap_len(slot: &[u8], chunk_count: u64) -> Result<usize> {
    let bitmap_len = usize::try_from(read_u64(slot, BITMAP_LEN_OFFSET)?)
        .map_err(|_| TakanawaError::PartCorrupt("bitmap length overflow".to_owned()))?;
    let expected_bitmap_len = bitmap_len_for_decode(chunk_count)?;
    if bitmap_len != expected_bitmap_len {
        return Err(TakanawaError::PartCorrupt(format!(
            "bitmap length mismatch: expected {expected_bitmap_len}, got {bitmap_len}"
        )));
    }
    Ok(bitmap_len)
}

fn verify_slot_size(
    slot: &[u8],
    offsets: MetadataOffsets,
    content_len: u64,
    chunk_size: u64,
) -> Result<()> {
    let slot_size = read_u64(slot, offsets.slot_size)?;
    let expected_slot_size = slot_size_for(content_len, chunk_size)?;
    if expected_slot_size != slot_size {
        return Err(TakanawaError::PartCorrupt(format!(
            "slot size mismatch: expected {expected_slot_size}, got {slot_size}"
        )));
    }
    if usize::try_from(slot_size).ok() != Some(slot.len()) {
        return Err(TakanawaError::PartCorrupt(format!(
            "slot buffer length mismatch: header says {slot_size}, buffer has {}",
            slot.len()
        )));
    }
    Ok(())
}

fn decode_hash(slot: &[u8], offsets: MetadataOffsets) -> Result<HashConfig> {
    let hash_kind =
        crate::HashKind::from_u32(u32::from(slot[HASH_KIND_OFFSET])).ok_or_else(|| {
            TakanawaError::PartCorrupt(format!("unsupported hash kind: {}", slot[HASH_KIND_OFFSET]))
        })?;
    let hash_len = usize::from(slot[HASH_LEN_OFFSET]);
    if hash_len > offsets.expected_hash_capacity {
        return Err(TakanawaError::PartCorrupt(format!(
            "hash length {hash_len} exceeds capacity {}",
            offsets.expected_hash_capacity
        )));
    }

    HashConfig::from_expected_bytes(
        hash_kind,
        &slot[EXPECTED_HASH_OFFSET..EXPECTED_HASH_OFFSET + hash_len],
    )
    .ok_or_else(|| {
        TakanawaError::PartCorrupt(format!(
            "unsupported hash kind/length: {}/{hash_len}",
            slot[HASH_KIND_OFFSET]
        ))
    })
}

/// Returns the aligned metadata slot size for a content length and chunk size.
///
/// Passing `0` for `chunk_size` selects the default chunk size.
///
/// # Errors
///
/// Returns an error if the chunk size or bitmap length is invalid.
pub fn slot_size_for(content_len: u64, chunk_size: u64) -> Result<u64> {
    let chunk_size = normalize_chunk_size(chunk_size)?;
    let chunk_count = chunk_count_for(content_len, chunk_size);
    let bitmap_len = u64::try_from(bitmap_len(chunk_count)?)
        .map_err(|_| TakanawaError::InvalidConfig("bitmap length overflow".to_owned()))?;
    let raw = HEADER_LEN as u64 + bitmap_len + ETAG_CAPACITY as u64 + LAST_MODIFIED_CAPACITY as u64;
    Ok(align_up(raw, ALIGNMENT))
}

fn bitmap_len_for_decode(chunk_count: u64) -> Result<usize> {
    bitmap_len(chunk_count)
}

fn align_up(value: u64, alignment: u64) -> u64 {
    value.div_ceil(alignment) * alignment
}

fn validate_header_len(value: Option<&str>, cap: usize, name: &str) -> Result<()> {
    if let Some(value) = value {
        if value.len() > cap {
            return Err(TakanawaError::InvalidConfig(format!(
                "{name} header is longer than {cap} bytes"
            )));
        }
    }
    Ok(())
}

fn write_fixed_string(dst: &mut [u8], value: Option<&str>) {
    if let Some(value) = value {
        dst[..value.len()].copy_from_slice(value.as_bytes());
    }
}

fn decode_optional_string(bytes: &[u8], len: usize) -> Result<Option<String>> {
    if len == 0 {
        return Ok(None);
    }
    let value = std::str::from_utf8(&bytes[..len])
        .map_err(|err| TakanawaError::PartCorrupt(format!("invalid stored UTF-8: {err}")))?;
    Ok(Some(value.to_owned()))
}

fn checksum_slot(slot: &[u8]) -> u32 {
    let mut hasher = Hasher::new();
    hasher.update(&slot[..CRC_OFFSET]);
    hasher.update(&[0, 0, 0, 0]);
    hasher.update(&slot[CRC_OFFSET + 4..]);
    hasher.finalize()
}

fn read_u16(bytes: &[u8], offset: usize) -> Result<u16> {
    let mut data = [0; 2];
    data.copy_from_slice(
        bytes
            .get(offset..offset + 2)
            .ok_or_else(|| TakanawaError::PartCorrupt("short metadata read".to_owned()))?,
    );
    Ok(u16::from_le_bytes(data))
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32> {
    let mut data = [0; 4];
    data.copy_from_slice(
        bytes
            .get(offset..offset + 4)
            .ok_or_else(|| TakanawaError::PartCorrupt("short metadata read".to_owned()))?,
    );
    Ok(u32::from_le_bytes(data))
}

fn read_u64(bytes: &[u8], offset: usize) -> Result<u64> {
    let mut data = [0; 8];
    data.copy_from_slice(
        bytes
            .get(offset..offset + 8)
            .ok_or_else(|| TakanawaError::PartCorrupt("short metadata read".to_owned()))?,
    );
    Ok(u64::from_le_bytes(data))
}

fn write_u16(bytes: &mut [u8], offset: usize, value: u16) {
    bytes[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
}

fn write_u32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn write_u64(bytes: &mut [u8], offset: usize, value: u64) {
    bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hash_url;

    #[test]
    fn slot_round_trips() {
        let remote = RemoteInfo {
            content_len: 10,
            etag: Some("abc".to_owned()),
            last_modified: Some("today".to_owned()),
        };
        let mut meta = PartMetadata::new(
            hash_url("https://example.test/file"),
            &remote,
            4,
            HashConfig::None,
        )
        .unwrap();
        meta.bitmap.mark_complete(1).unwrap();
        meta.generation = 42;

        let slot_size = slot_size_for(meta.content_len, meta.chunk_size).unwrap();
        let slot = meta.encode_slot(slot_size).unwrap();
        let decoded = PartMetadata::decode_slot(&slot).unwrap();

        assert_eq!(decoded, meta);
    }

    #[test]
    fn slot_round_trips_wide_hashes() {
        let remote = RemoteInfo {
            content_len: 10,
            etag: None,
            last_modified: None,
        };
        let hashes = [
            HashConfig::Sha1([1; 20]),
            HashConfig::Sha256([2; 32]),
            HashConfig::Sha512([3; 64]),
            HashConfig::Md5([4; 16]),
            HashConfig::Crc32([5; 4]),
        ];

        for hash in hashes {
            let meta =
                PartMetadata::new(hash_url("https://example.test/file"), &remote, 4, hash).unwrap();
            let slot_size = slot_size_for(meta.content_len, meta.chunk_size).unwrap();
            let slot = meta.encode_slot(slot_size).unwrap();
            let decoded = PartMetadata::decode_slot(&slot).unwrap();

            assert_eq!(decoded.hash, hash);
        }
    }

    #[test]
    fn picks_up_crc_corruption() {
        let remote = RemoteInfo {
            content_len: 10,
            etag: None,
            last_modified: None,
        };
        let meta = PartMetadata::new(
            hash_url("https://example.test/file"),
            &remote,
            4,
            HashConfig::None,
        )
        .unwrap();
        let slot_size = slot_size_for(meta.content_len, meta.chunk_size).unwrap();
        let mut slot = meta.encode_slot(slot_size).unwrap();
        let last = slot.len() - 1;
        slot[last] ^= 1;

        assert!(PartMetadata::decode_slot(&slot).is_err());
    }

    #[test]
    fn rejects_changed_remote_validators() {
        let remote = RemoteInfo {
            content_len: 10,
            etag: Some("etag-a".to_owned()),
            last_modified: Some("date-a".to_owned()),
        };
        let meta = PartMetadata::new(
            hash_url("https://example.test/file"),
            &remote,
            4,
            HashConfig::None,
        )
        .unwrap();

        let changed_etag = RemoteInfo {
            etag: Some("etag-b".to_owned()),
            ..remote.clone()
        };
        assert!(matches!(
            meta.ensure_compatible(
                hash_url("https://example.test/file"),
                &changed_etag,
                4,
                HashConfig::None,
            ),
            Err(TakanawaError::RemoteChanged(_))
        ));

        let changed_last_modified = RemoteInfo {
            last_modified: Some("date-b".to_owned()),
            ..remote.clone()
        };
        assert!(matches!(
            meta.ensure_compatible(
                hash_url("https://example.test/file"),
                &changed_last_modified,
                4,
                HashConfig::None,
            ),
            Err(TakanawaError::RemoteChanged(_))
        ));
    }

    #[test]
    fn skips_missing_remote_validator_checks() {
        let remote = RemoteInfo {
            content_len: 10,
            etag: Some("etag-a".to_owned()),
            last_modified: Some("date-a".to_owned()),
        };
        let meta = PartMetadata::new(
            hash_url("https://example.test/file"),
            &remote,
            4,
            HashConfig::None,
        )
        .unwrap();
        let missing_validators = RemoteInfo {
            content_len: 10,
            etag: None,
            last_modified: None,
        };

        meta.ensure_compatible(
            hash_url("https://example.test/file"),
            &missing_validators,
            4,
            HashConfig::None,
        )
        .unwrap();
    }
}
