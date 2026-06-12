use crate::{Result, TakanawaError};

/// Compact completion bitmap for a fixed number of chunks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkBitmap {
    chunk_count: u64,
    bytes: Vec<u8>,
}

impl ChunkBitmap {
    /// Creates an empty bitmap for `chunk_count` chunks.
    ///
    /// # Errors
    ///
    /// Returns an error if the bitmap length cannot fit in memory indexes.
    pub fn new(chunk_count: u64) -> Result<Self> {
        let len = bitmap_len(chunk_count)?;
        Ok(Self {
            chunk_count,
            bytes: vec![0; len],
        })
    }

    /// Creates a bitmap from serialized bytes.
    ///
    /// # Errors
    ///
    /// Returns an error if the byte length does not match `chunk_count` or if
    /// unused bits are set in the final byte.
    pub fn from_bytes(chunk_count: u64, bytes: Vec<u8>) -> Result<Self> {
        let expected = bitmap_len(chunk_count)?;
        if bytes.len() != expected {
            return Err(TakanawaError::PartCorrupt(format!(
                "bitmap length mismatch: expected {expected}, got {}",
                bytes.len()
            )));
        }

        let bitmap = Self { chunk_count, bytes };
        bitmap.validate_unused_bits()?;
        Ok(bitmap)
    }

    #[must_use]
    /// Returns the number of chunks tracked by the bitmap.
    pub const fn chunk_count(&self) -> u64 {
        self.chunk_count
    }

    #[must_use]
    /// Returns the serialized bitmap bytes.
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    #[must_use]
    /// Consumes the bitmap and returns its serialized bytes.
    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }

    /// Marks a chunk complete.
    ///
    /// # Errors
    ///
    /// Returns an error if `index` is outside the bitmap.
    pub fn mark_complete(&mut self, index: u64) -> Result<()> {
        let (byte, mask) = self.bit_position(index)?;
        self.bytes[byte] |= mask;
        Ok(())
    }

    /// Returns whether a chunk is complete.
    ///
    /// # Errors
    ///
    /// Returns an error if `index` is outside the bitmap.
    pub fn is_complete(&self, index: u64) -> Result<bool> {
        let (byte, mask) = self.bit_position(index)?;
        Ok((self.bytes[byte] & mask) != 0)
    }

    #[must_use]
    /// Returns whether all chunks are complete.
    pub fn all_complete(&self) -> bool {
        self.complete_count() == self.chunk_count
    }

    #[must_use]
    /// Counts chunks marked complete.
    ///
    /// # Panics
    ///
    /// Panics only if the count of completed chunks does not fit in `u64`.
    pub fn complete_count(&self) -> u64 {
        (0..self.chunk_count)
            .filter(|index| self.is_complete(*index).unwrap_or(false))
            .count()
            .try_into()
            .expect("complete count fits u64")
    }

    #[must_use]
    /// Returns indexes of chunks that are not yet complete.
    pub fn incomplete_indices(&self) -> Vec<u64> {
        (0..self.chunk_count)
            .filter(|index| !self.is_complete(*index).unwrap_or(false))
            .collect()
    }

    fn bit_position(&self, index: u64) -> Result<(usize, u8)> {
        if index >= self.chunk_count {
            return Err(TakanawaError::InvalidConfig(format!(
                "chunk index {index} is outside chunk count {}",
                self.chunk_count
            )));
        }

        let byte = usize::try_from(index / 8)
            .map_err(|_| TakanawaError::InvalidConfig("bitmap index overflow".to_owned()))?;
        let bit = u8::try_from(index % 8).expect("bit offset is in 0..8");
        Ok((byte, 1u8 << bit))
    }

    fn validate_unused_bits(&self) -> Result<()> {
        if self.chunk_count == 0 {
            return Ok(());
        }

        let used_bits = self.chunk_count % 8;
        if used_bits == 0 {
            return Ok(());
        }

        let last = *self
            .bytes
            .last()
            .ok_or_else(|| TakanawaError::PartCorrupt("empty bitmap".to_owned()))?;
        let valid_mask = (1u8 << used_bits) - 1;
        if (last & !valid_mask) != 0 {
            return Err(TakanawaError::PartCorrupt(
                "bitmap has unused completion bits set".to_owned(),
            ));
        }
        Ok(())
    }
}

/// Returns the number of bytes needed to store `chunk_count` completion bits.
///
/// # Errors
///
/// Returns an error if the byte length cannot fit in memory indexes.
pub fn bitmap_len(chunk_count: u64) -> Result<usize> {
    usize::try_from(chunk_count.div_ceil(8))
        .map_err(|_| TakanawaError::InvalidConfig("bitmap length overflow".to_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tracks_completed_chunks() {
        let mut bitmap = ChunkBitmap::new(10).unwrap();

        bitmap.mark_complete(0).unwrap();
        bitmap.mark_complete(9).unwrap();

        assert!(bitmap.is_complete(0).unwrap());
        assert!(!bitmap.is_complete(1).unwrap());
        assert!(bitmap.is_complete(9).unwrap());
        assert_eq!(bitmap.complete_count(), 2);
        assert_eq!(bitmap.incomplete_indices().len(), 8);
    }

    #[test]
    fn rejects_unused_bits() {
        assert!(ChunkBitmap::from_bytes(9, vec![0, 0b1111_1110]).is_err());
    }
}
