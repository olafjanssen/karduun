use blake3;
use sha2::{Digest, Sha256};

/// Compute SHA-256 hash of content
pub fn sha256_hash(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}

/// Compute Blake3 hash of content (faster, modern alternative)
pub fn blake3_hash(data: &[u8]) -> String {
    let hash = blake3::hash(data);
    hex::encode(hash.as_bytes())
}

/// Compute content hash for a card's canonical serialization
pub fn card_content_hash(card_serialized: &[u8]) -> String {
    // Use Blake3 for speed, SHA256 available as alternative
    blake3_hash(card_serialized)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sha256_hash() {
        let hash = sha256_hash(b"test");
        assert_eq!(hash.len(), 64);
    }

    #[test]
    fn test_blake3_hash() {
        let hash = blake3_hash(b"test");
        assert_eq!(hash.len(), 64);
    }

    #[test]
    fn test_deterministic() {
        let data = b"test data";
        let h1 = blake3_hash(data);
        let h2 = blake3_hash(data);
        assert_eq!(h1, h2);
    }
}

