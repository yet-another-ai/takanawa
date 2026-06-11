use crc32fast::Hasher as Crc32Hasher;
use sha2::{Digest, Sha256, Sha512};

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HashKind {
    None = 0,
    Sha256 = 1,
    Sha1 = 2,
    Sha512 = 3,
    Md5 = 4,
    Crc32 = 5,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HashConfig {
    None,
    Sha1([u8; 20]),
    Sha256([u8; 32]),
    Sha512([u8; 64]),
    Md5([u8; 16]),
    Crc32([u8; 4]),
}

pub(crate) enum HashVerifier {
    Sha1 {
        hasher: Box<Sha1Hasher>,
        expected: [u8; 20],
    },
    Sha256 {
        hasher: Box<Sha256>,
        expected: [u8; 32],
    },
    Sha512 {
        hasher: Box<Sha512>,
        expected: [u8; 64],
    },
    Md5 {
        hasher: Box<Md5Hasher>,
        expected: [u8; 16],
    },
    Crc32 {
        hasher: Box<Crc32Hasher>,
        expected: [u8; 4],
    },
}

impl HashKind {
    #[must_use]
    pub const fn expected_len(self) -> usize {
        match self {
            Self::None => 0,
            Self::Sha1 => 20,
            Self::Sha256 => 32,
            Self::Sha512 => 64,
            Self::Md5 => 16,
            Self::Crc32 => 4,
        }
    }

    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Sha1 => "SHA-1",
            Self::Sha256 => "SHA-256",
            Self::Sha512 => "SHA-512",
            Self::Md5 => "MD5",
            Self::Crc32 => "CRC32",
        }
    }

    #[must_use]
    pub const fn from_u32(value: u32) -> Option<Self> {
        match value {
            0 => Some(Self::None),
            1 => Some(Self::Sha256),
            2 => Some(Self::Sha1),
            3 => Some(Self::Sha512),
            4 => Some(Self::Md5),
            5 => Some(Self::Crc32),
            _ => None,
        }
    }
}

impl From<HashKind> for u8 {
    fn from(kind: HashKind) -> Self {
        match kind {
            HashKind::None => 0,
            HashKind::Sha256 => 1,
            HashKind::Sha1 => 2,
            HashKind::Sha512 => 3,
            HashKind::Md5 => 4,
            HashKind::Crc32 => 5,
        }
    }
}

impl From<HashKind> for u32 {
    fn from(kind: HashKind) -> Self {
        u8::from(kind).into()
    }
}

impl HashConfig {
    #[must_use]
    pub const fn kind(self) -> HashKind {
        match self {
            Self::None => HashKind::None,
            Self::Sha1(_) => HashKind::Sha1,
            Self::Sha256(_) => HashKind::Sha256,
            Self::Sha512(_) => HashKind::Sha512,
            Self::Md5(_) => HashKind::Md5,
            Self::Crc32(_) => HashKind::Crc32,
        }
    }

    #[must_use]
    pub fn expected_bytes(self) -> Option<Vec<u8>> {
        match self {
            Self::None => None,
            Self::Sha1(hash) => Some(hash.to_vec()),
            Self::Sha256(hash) => Some(hash.to_vec()),
            Self::Sha512(hash) => Some(hash.to_vec()),
            Self::Md5(hash) => Some(hash.to_vec()),
            Self::Crc32(hash) => Some(hash.to_vec()),
        }
    }

    #[must_use]
    pub fn from_expected_bytes(kind: HashKind, bytes: &[u8]) -> Option<Self> {
        match kind {
            HashKind::None if bytes.is_empty() => Some(Self::None),
            HashKind::None => None,
            HashKind::Sha1 if bytes.len() == 20 => Some(Self::Sha1(bytes.try_into().ok()?)),
            HashKind::Sha256 if bytes.len() == 32 => Some(Self::Sha256(bytes.try_into().ok()?)),
            HashKind::Sha512 if bytes.len() == 64 => Some(Self::Sha512(bytes.try_into().ok()?)),
            HashKind::Md5 if bytes.len() == 16 => Some(Self::Md5(bytes.try_into().ok()?)),
            HashKind::Crc32 if bytes.len() == 4 => Some(Self::Crc32(bytes.try_into().ok()?)),
            _ => None,
        }
    }
}

impl HashVerifier {
    #[must_use]
    pub(crate) fn new(config: HashConfig) -> Option<Self> {
        match config {
            HashConfig::None => None,
            HashConfig::Sha1(expected) => Some(Self::Sha1 {
                hasher: Box::new(Sha1Hasher::new()),
                expected,
            }),
            HashConfig::Sha256(expected) => Some(Self::Sha256 {
                hasher: Box::new(Sha256::new()),
                expected,
            }),
            HashConfig::Sha512(expected) => Some(Self::Sha512 {
                hasher: Box::new(Sha512::new()),
                expected,
            }),
            HashConfig::Md5(expected) => Some(Self::Md5 {
                hasher: Box::new(Md5Hasher::new()),
                expected,
            }),
            HashConfig::Crc32(expected) => Some(Self::Crc32 {
                hasher: Box::new(Crc32Hasher::new()),
                expected,
            }),
        }
    }

    pub(crate) fn update(&mut self, bytes: &[u8]) {
        match self {
            Self::Sha1 { hasher, .. } => hasher.update(bytes),
            Self::Sha256 { hasher, .. } => hasher.update(bytes),
            Self::Sha512 { hasher, .. } => hasher.update(bytes),
            Self::Md5 { hasher, .. } => hasher.update(bytes),
            Self::Crc32 { hasher, .. } => hasher.update(bytes),
        }
    }

    #[must_use]
    pub(crate) fn finish(self) -> bool {
        match self {
            Self::Sha1 { hasher, expected } => hasher.finalize() == expected,
            Self::Sha256 { hasher, expected } => {
                let actual: [u8; 32] = hasher.finalize().into();
                actual == expected
            }
            Self::Sha512 { hasher, expected } => {
                let actual: [u8; 64] = hasher.finalize().into();
                actual == expected
            }
            Self::Md5 { hasher, expected } => hasher.finalize() == expected,
            Self::Crc32 { hasher, expected } => hasher.finalize().to_be_bytes() == expected,
        }
    }
}

#[must_use]
pub fn hash_url(url: &str) -> [u8; 32] {
    let digest = Sha256::digest(url.as_bytes());
    digest.into()
}

struct Sha1Hasher {
    state: [u32; 5],
    len: u64,
    buffer: [u8; 64],
    buffer_len: usize,
}

impl Sha1Hasher {
    #[must_use]
    const fn new() -> Self {
        Self {
            state: [
                0x6745_2301,
                0xefcd_ab89,
                0x98ba_dcfe,
                0x1032_5476,
                0xc3d2_e1f0,
            ],
            len: 0,
            buffer: [0; 64],
            buffer_len: 0,
        }
    }

    fn update(&mut self, mut bytes: &[u8]) {
        self.len = self
            .len
            .wrapping_add(u64::try_from(bytes.len()).expect("slice length fits in u64"));
        if self.buffer_len > 0 {
            let take = (64 - self.buffer_len).min(bytes.len());
            self.buffer[self.buffer_len..self.buffer_len + take].copy_from_slice(&bytes[..take]);
            self.buffer_len += take;
            bytes = &bytes[take..];
            if self.buffer_len == 64 {
                sha1_compress(&mut self.state, &self.buffer);
                self.buffer_len = 0;
            }
        }
        let mut chunks = bytes.chunks_exact(64);
        for chunk in chunks.by_ref() {
            sha1_compress(
                &mut self.state,
                chunk.try_into().expect("chunk is 64 bytes"),
            );
        }
        let remainder = chunks.remainder();
        if !remainder.is_empty() {
            self.buffer[..remainder.len()].copy_from_slice(remainder);
            self.buffer_len = remainder.len();
        }
    }

    #[must_use]
    fn finalize(mut self) -> [u8; 20] {
        let bit_len = self.len.wrapping_mul(8);
        self.update_padding_be(bit_len);
        let mut out = [0; 20];
        for (chunk, value) in out.chunks_exact_mut(4).zip(self.state) {
            chunk.copy_from_slice(&value.to_be_bytes());
        }
        out
    }

    fn update_padding_be(&mut self, bit_len: u64) {
        let mut padding = [0; 128];
        padding[0] = 0x80;
        let pad_len = if self.buffer_len < 56 {
            56 - self.buffer_len
        } else {
            120 - self.buffer_len
        };
        self.update(&padding[..pad_len]);
        self.update(&bit_len.to_be_bytes());
    }
}

impl Default for Sha1Hasher {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(clippy::many_single_char_names)]
fn sha1_compress(state: &mut [u32; 5], block: &[u8; 64]) {
    let mut w = [0_u32; 80];
    for (i, word) in w.iter_mut().take(16).enumerate() {
        let start = i * 4;
        *word = u32::from_be_bytes(block[start..start + 4].try_into().expect("word length"));
    }
    for i in 16..80 {
        w[i] = (w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]).rotate_left(1);
    }
    let [mut a, mut b, mut c, mut d, mut e] = *state;
    for (i, &wi) in w.iter().enumerate() {
        let (f, k) = match i {
            0..=19 => ((b & c) | ((!b) & d), 0x5a82_7999),
            20..=39 => (b ^ c ^ d, 0x6ed9_eba1),
            40..=59 => ((b & c) | (b & d) | (c & d), 0x8f1b_bcdc),
            _ => (b ^ c ^ d, 0xca62_c1d6),
        };
        let temp = a
            .rotate_left(5)
            .wrapping_add(f)
            .wrapping_add(e)
            .wrapping_add(k)
            .wrapping_add(wi);
        e = d;
        d = c;
        c = b.rotate_left(30);
        b = a;
        a = temp;
    }
    state[0] = state[0].wrapping_add(a);
    state[1] = state[1].wrapping_add(b);
    state[2] = state[2].wrapping_add(c);
    state[3] = state[3].wrapping_add(d);
    state[4] = state[4].wrapping_add(e);
}

struct Md5Hasher {
    state: [u32; 4],
    len: u64,
    buffer: [u8; 64],
    buffer_len: usize,
}

impl Md5Hasher {
    #[must_use]
    const fn new() -> Self {
        Self {
            state: [0x6745_2301, 0xefcd_ab89, 0x98ba_dcfe, 0x1032_5476],
            len: 0,
            buffer: [0; 64],
            buffer_len: 0,
        }
    }

    fn update(&mut self, mut bytes: &[u8]) {
        self.len = self
            .len
            .wrapping_add(u64::try_from(bytes.len()).expect("slice length fits in u64"));
        if self.buffer_len > 0 {
            let take = (64 - self.buffer_len).min(bytes.len());
            self.buffer[self.buffer_len..self.buffer_len + take].copy_from_slice(&bytes[..take]);
            self.buffer_len += take;
            bytes = &bytes[take..];
            if self.buffer_len == 64 {
                md5_compress(&mut self.state, &self.buffer);
                self.buffer_len = 0;
            }
        }
        let mut chunks = bytes.chunks_exact(64);
        for chunk in chunks.by_ref() {
            md5_compress(
                &mut self.state,
                chunk.try_into().expect("chunk is 64 bytes"),
            );
        }
        let remainder = chunks.remainder();
        if !remainder.is_empty() {
            self.buffer[..remainder.len()].copy_from_slice(remainder);
            self.buffer_len = remainder.len();
        }
    }

    #[must_use]
    fn finalize(mut self) -> [u8; 16] {
        let bit_len = self.len.wrapping_mul(8);
        let mut padding = [0; 128];
        padding[0] = 0x80;
        let pad_len = if self.buffer_len < 56 {
            56 - self.buffer_len
        } else {
            120 - self.buffer_len
        };
        self.update(&padding[..pad_len]);
        self.update(&bit_len.to_le_bytes());
        let mut out = [0; 16];
        for (chunk, value) in out.chunks_exact_mut(4).zip(self.state) {
            chunk.copy_from_slice(&value.to_le_bytes());
        }
        out
    }
}

impl Default for Md5Hasher {
    fn default() -> Self {
        Self::new()
    }
}

const MD5_S: [u32; 64] = [
    7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 5, 9, 14, 20, 5, 9, 14, 20, 5, 9,
    14, 20, 5, 9, 14, 20, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 6, 10, 15,
    21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21,
];

const MD5_K: [u32; 64] = [
    0xd76a_a478,
    0xe8c7_b756,
    0x2420_70db,
    0xc1bd_ceee,
    0xf57c_0faf,
    0x4787_c62a,
    0xa830_4613,
    0xfd46_9501,
    0x6980_98d8,
    0x8b44_f7af,
    0xffff_5bb1,
    0x895c_d7be,
    0x6b90_1122,
    0xfd98_7193,
    0xa679_438e,
    0x49b4_0821,
    0xf61e_2562,
    0xc040_b340,
    0x265e_5a51,
    0xe9b6_c7aa,
    0xd62f_105d,
    0x0244_1453,
    0xd8a1_e681,
    0xe7d3_fbc8,
    0x21e1_cde6,
    0xc337_07d6,
    0xf4d5_0d87,
    0x455a_14ed,
    0xa9e3_e905,
    0xfcef_a3f8,
    0x676f_02d9,
    0x8d2a_4c8a,
    0xfffa_3942,
    0x8771_f681,
    0x6d9d_6122,
    0xfde5_380c,
    0xa4be_ea44,
    0x4bde_cfa9,
    0xf6bb_4b60,
    0xbebf_bc70,
    0x289b_7ec6,
    0xeaa1_27fa,
    0xd4ef_3085,
    0x0488_1d05,
    0xd9d4_d039,
    0xe6db_99e5,
    0x1fa2_7cf8,
    0xc4ac_5665,
    0xf429_2244,
    0x432a_ff97,
    0xab94_23a7,
    0xfc93_a039,
    0x655b_59c3,
    0x8f0c_cc92,
    0xffef_f47d,
    0x8584_5dd1,
    0x6fa8_7e4f,
    0xfe2c_e6e0,
    0xa301_4314,
    0x4e08_11a1,
    0xf753_7e82,
    0xbd3a_f235,
    0x2ad7_d2bb,
    0xeb86_d391,
];

#[allow(clippy::many_single_char_names)]
fn md5_compress(state: &mut [u32; 4], block: &[u8; 64]) {
    let mut m = [0_u32; 16];
    for (i, word) in m.iter_mut().enumerate() {
        let start = i * 4;
        *word = u32::from_le_bytes(block[start..start + 4].try_into().expect("word length"));
    }
    let [mut a, mut b, mut c, mut d] = *state;
    for i in 0..64 {
        let (f, g) = match i {
            0..=15 => ((b & c) | ((!b) & d), i),
            16..=31 => ((d & b) | ((!d) & c), (5 * i + 1) % 16),
            32..=47 => (b ^ c ^ d, (3 * i + 5) % 16),
            _ => (c ^ (b | !d), (7 * i) % 16),
        };
        let temp = d;
        d = c;
        c = b;
        b = b.wrapping_add(
            a.wrapping_add(f)
                .wrapping_add(MD5_K[i])
                .wrapping_add(m[g])
                .rotate_left(MD5_S[i]),
        );
        a = temp;
    }
    state[0] = state[0].wrapping_add(a);
    state[1] = state[1].wrapping_add(b);
    state[2] = state[2].wrapping_add(c);
    state[3] = state[3].wrapping_add(d);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha1_matches_known_vector() {
        let mut hasher = Sha1Hasher::new();
        hasher.update(b"abc");
        assert_eq!(
            hex::encode(hasher.finalize()),
            "a9993e364706816aba3e25717850c26c9cd0d89d"
        );
    }

    #[test]
    fn md5_matches_known_vector() {
        let mut hasher = Md5Hasher::new();
        hasher.update(b"abc");
        assert_eq!(
            hex::encode(hasher.finalize()),
            "900150983cd24fb0d6963f7d28e17f72"
        );
    }
}
