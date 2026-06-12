use crate::{Result, TakanawaError};

/// Default chunk size used when a caller passes `0` as the configured chunk size.
pub const DEFAULT_CHUNK_SIZE: u64 = 64 * 1024 * 1024;

/// Byte range assigned to one download chunk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Chunk {
    /// Zero-based chunk index within the download.
    pub index: u64,
    /// Inclusive byte offset where this chunk starts in the target file.
    pub start: u64,
    /// Inclusive byte offset where this chunk ends in the target file.
    pub end: u64,
    /// Number of bytes in this chunk.
    pub len: u64,
}

/// Precomputed chunk layout for a remote resource.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkPlan {
    content_len: u64,
    chunk_size: u64,
    chunk_count: u64,
}

impl ChunkPlan {
    /// Creates a chunk plan for a resource length and chunk size.
    ///
    /// Passing `0` for `chunk_size` selects [`DEFAULT_CHUNK_SIZE`].
    ///
    /// # Errors
    ///
    /// Returns an error if the normalized chunk size cannot be represented in
    /// signed file offsets.
    pub fn new(content_len: u64, chunk_size: u64) -> Result<Self> {
        let chunk_size = normalize_chunk_size(chunk_size)?;
        let chunk_count = chunk_count_for(content_len, chunk_size);
        Ok(Self {
            content_len,
            chunk_size,
            chunk_count,
        })
    }

    #[must_use]
    /// Returns the total content length covered by the plan.
    pub const fn content_len(&self) -> u64 {
        self.content_len
    }

    #[must_use]
    /// Returns the normalized chunk size used by the plan.
    pub const fn chunk_size(&self) -> u64 {
        self.chunk_size
    }

    #[must_use]
    /// Returns the number of chunks in the plan.
    pub const fn chunk_count(&self) -> u64 {
        self.chunk_count
    }

    /// Returns the byte range for a chunk index.
    ///
    /// # Errors
    ///
    /// Returns an error if `index` is outside the plan or if the chunk offset
    /// overflows.
    pub fn chunk(&self, index: u64) -> Result<Chunk> {
        if index >= self.chunk_count {
            return Err(TakanawaError::InvalidConfig(format!(
                "chunk index {index} is outside chunk count {}",
                self.chunk_count
            )));
        }

        let start = index
            .checked_mul(self.chunk_size)
            .ok_or_else(|| TakanawaError::InvalidConfig("chunk offset overflow".to_owned()))?;
        let remaining = self.content_len - start;
        let len = remaining.min(self.chunk_size);
        let end = start + len - 1;

        Ok(Chunk {
            index,
            start,
            end,
            len,
        })
    }

    #[must_use]
    /// Returns all chunks in index order.
    ///
    /// # Panics
    ///
    /// Panics only if an internally generated index is rejected by the plan.
    pub fn all_chunks(&self) -> Vec<Chunk> {
        (0..self.chunk_count)
            .map(|index| self.chunk(index).expect("valid generated chunk index"))
            .collect()
    }
}

/// Returns the effective chunk size for a caller-provided value.
///
/// Passing `0` selects [`DEFAULT_CHUNK_SIZE`].
///
/// # Errors
///
/// Returns an error if `chunk_size` cannot be represented in signed file
/// offsets.
pub fn normalize_chunk_size(chunk_size: u64) -> Result<u64> {
    match chunk_size {
        0 => Ok(DEFAULT_CHUNK_SIZE),
        value => {
            if value > i64::MAX as u64 {
                return Err(TakanawaError::InvalidConfig(
                    "chunk size must fit in signed file offsets".to_owned(),
                ));
            }
            Ok(value)
        }
    }
}

#[must_use]
/// Returns the number of chunks needed for a resource length.
pub const fn chunk_count_for(content_len: u64, chunk_size: u64) -> u64 {
    if content_len == 0 {
        0
    } else {
        content_len.div_ceil(chunk_size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plans_default_sized_chunks() {
        let plan = ChunkPlan::new(10, 4).unwrap();

        assert_eq!(plan.chunk_count(), 3);
        assert_eq!(
            plan.all_chunks(),
            vec![
                Chunk {
                    index: 0,
                    start: 0,
                    end: 3,
                    len: 4,
                },
                Chunk {
                    index: 1,
                    start: 4,
                    end: 7,
                    len: 4,
                },
                Chunk {
                    index: 2,
                    start: 8,
                    end: 9,
                    len: 2,
                },
            ]
        );
    }

    #[test]
    fn zero_length_has_no_chunks() {
        let plan = ChunkPlan::new(0, 4).unwrap();

        assert_eq!(plan.chunk_count(), 0);
    }
}
