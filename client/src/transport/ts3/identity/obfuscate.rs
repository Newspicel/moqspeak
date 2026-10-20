//! The obfuscation the TeamSpeak client puts over an exported identity. It covers the first 100
//! bytes with a fixed key, and the first 20 with the hash of what follows them.

use sha1::{Digest, Sha1};

/// The first 100 bytes of the key the TeamSpeak client obfuscates an identity with. Obfuscation
/// covers 100 bytes at most, so the rest of that key is never read.
const KEY: &[u8; 100] = b"b9dfaa7bee6ac57ac7b65f1094a1c155e747327bc2fe5d51c512023fe54a280201004e90ad1daaae1075d53b7d571c30e063";

/// How many bytes the hash of the tail covers.
pub const HASH_LEN: usize = 20;

/// Puts the obfuscation over `data`.
pub fn apply(data: &mut [u8]) {
    xor_key(data);
    xor_hash(data);
}

/// Lifts the obfuscation off `data`.
pub fn lift(data: &mut [u8]) {
    xor_hash(data);
    xor_key(data);
}

/// Covers the first 100 bytes with the fixed key.
fn xor_key(data: &mut [u8]) {
    for (i, byte) in data.iter_mut().take(KEY.len()).enumerate() {
        *byte ^= KEY[i];
    }
}

/// Covers the first 20 bytes with the hash of the tail, up to the first zero byte.
fn xor_hash(data: &mut [u8]) {
    let Some(tail) = data.get(HASH_LEN..) else {
        return;
    };
    let end = tail.iter().position(|&b| b == 0).unwrap_or(tail.len());
    let hash = Sha1::digest(&tail[..end]);
    for (i, byte) in data.iter_mut().take(HASH_LEN).enumerate() {
        *byte ^= hash[i];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lifts_what_it_applied() {
        let plain = b"MIGbAgEAAiEA7ZmTq1N6rB8kXvQ2hLpYc3FdGs9TuWvAzE4nKmoCIQCmXn7b".to_vec();
        let mut data = plain.clone();
        apply(&mut data);
        assert_ne!(data, plain, "the obfuscation changes the bytes");
        lift(&mut data);
        assert_eq!(data, plain);
    }

    #[test]
    fn leaves_data_too_short_to_hold_a_tail() {
        let mut data = [1u8; HASH_LEN];
        let plain = data;
        apply(&mut data);
        lift(&mut data);
        assert_eq!(data, plain);
    }
}
