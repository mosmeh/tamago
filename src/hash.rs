use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum HashFunc {
    XxHash,
    Fnv,
    MurmurHash,
    Crc,
}

impl FromStr for HashFunc {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "xxhash" => Ok(Self::XxHash),
            "fnv" => Ok(Self::Fnv),
            "murmur" | "murmurhash" => Ok(Self::MurmurHash),
            "crc" => Ok(Self::Crc),
            _ => Err(format!("Unknown hash function '{}'", s)),
        }
    }
}

impl HashFunc {
    #[inline]
    pub fn hash(&self, x: &[u8]) -> u32 {
        match self {
            HashFunc::XxHash => xxhash_rust::xxh32::xxh32(x, 0),
            HashFunc::Fnv => {
                use hash32::Hasher;
                let mut hasher = hash32::FnvHasher::default();
                hasher.write(x);
                hasher.finish()
            }
            HashFunc::MurmurHash => {
                use hash32::Hasher;
                let mut hasher = hash32::Murmur3Hasher::default();
                hasher.write(x);
                hasher.finish()
            }
            HashFunc::Crc => {
                let mut hasher = crc32fast::Hasher::new();
                hasher.update(x);
                hasher.finalize()
            }
        }
    }
}
