//! Core resumable download primitives shared by the Takanawa front ends.

mod bitmap;
mod chunk;
mod error;
mod hash;
mod metadata;
mod part_file;

pub use bitmap::ChunkBitmap;
pub use chunk::{Chunk, ChunkPlan, DEFAULT_CHUNK_SIZE};
pub use error::{Result, TakanawaError};
pub(crate) use hash::HashVerifier;
pub use hash::{HashConfig, HashKind, hash_url};
pub use metadata::{METADATA_VERSION, PartMetadata, RemoteInfo, slot_size_for};
pub use part_file::{PartFile, part_lock_path_for, part_path_for};
