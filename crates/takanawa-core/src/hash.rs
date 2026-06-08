use sha2::{Digest, Sha256};

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HashKind {
    None = 0,
    Sha256 = 1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HashConfig {
    None,
    Sha256([u8; 32]),
}

impl HashConfig {
    #[must_use]
    pub const fn kind(self) -> HashKind {
        match self {
            Self::None => HashKind::None,
            Self::Sha256(_) => HashKind::Sha256,
        }
    }

    #[must_use]
    pub const fn expected_sha256(self) -> Option<[u8; 32]> {
        match self {
            Self::None => None,
            Self::Sha256(hash) => Some(hash),
        }
    }
}

#[must_use]
pub fn hash_url(url: &str) -> [u8; 32] {
    let digest = Sha256::digest(url.as_bytes());
    digest.into()
}
