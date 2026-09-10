//! Standard SHA-256 wrapper for content/request digests.

use sha2::{Digest as ShaDigest, Sha256 as InnerSha256};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Digest(pub(crate) [u8; 32]);

impl Digest {
    pub(crate) fn bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl std::fmt::LowerHex for Digest {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for byte in self.0 {
            write!(formatter, "{byte:02x}")?;
        }
        Ok(())
    }
}

#[derive(Clone)]
pub(crate) struct Sha256 {
    inner: InnerSha256,
}

impl Sha256 {
    pub(crate) fn new() -> Self {
        Self {
            inner: InnerSha256::new(),
        }
    }

    pub(crate) fn update(&mut self, input: &[u8]) {
        self.inner.update(input);
    }

    pub(crate) fn finalize(self) -> Digest {
        Digest(self.inner.finalize().into())
    }

    pub(crate) fn digest(input: &[u8]) -> Digest {
        let mut hasher = Self::new();
        hasher.update(input);
        hasher.finalize()
    }
}

#[cfg(test)]
mod tests {
    use super::Sha256;

    #[test]
    fn sha256_known_vectors_and_chunking() {
        let expected = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
        assert_eq!(format!("{:x}", Sha256::digest(b"abc")), expected);
        let mut digest = Sha256::new();
        for byte in b"abc" {
            digest.update(std::slice::from_ref(byte));
        }
        assert_eq!(format!("{:x}", digest.finalize()), expected);
    }
}
