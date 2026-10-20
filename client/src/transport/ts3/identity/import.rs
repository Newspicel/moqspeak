//! Reads a TeamSpeak identity from text. The TeamSpeak client exports one as an INI file, where
//! the identity is a counter, the letter `V` and an obfuscated key pair. The plain
//! `<key>:<offset>` form is read as well.

use std::path::PathBuf;

use anyhow::{Context, Result, anyhow, bail, ensure};
use base64::Engine as _;
use base64::engine::general_purpose::{STANDARD, STANDARD_NO_PAD};
use sha1::{Digest, Sha1};

/// The first 100 bytes of the key the TeamSpeak client obfuscates an identity with. Obfuscation
/// covers 100 bytes at most, so the rest of that key is never read.
const OBFUSCATION_KEY: &[u8; 100] = b"b9dfaa7bee6ac57ac7b65f1094a1c155e747327bc2fe5d51c512023fe54a280201004e90ad1daaae1075d53b7d571c30e063";

/// How many bytes the hash of the tail covers.
const HASH_LEN: usize = 20;

/// The size of a P-256 scalar, and of each coordinate of the point it gives.
const SCALAR: usize = 32;

/// An identity read from text, with what the file says about it.
pub struct Imported {
    pub identity: ts3proto::Identity,
    /// The name the file gives the identity.
    pub label: Option<String>,
    /// The nickname the file carries.
    pub nickname: Option<String>,
}

/// Reads an identity from what a person pasted: a path to an exported file, the text of that
/// file, or the `<key>:<offset>` string.
pub fn from_input(input: &str) -> Result<Imported> {
    let input = input.trim();
    if !input.contains('\n') {
        let path = as_path(input.trim_matches(['"', '\'']));
        if path.is_file() {
            let text = std::fs::read_to_string(&path)
                .with_context(|| format!("reading {}", path.display()))?;
            return parse(&text);
        }
    }
    parse(input)
}

/// The file a pasted path names. A leading `~` stands for the home directory.
fn as_path(input: &str) -> PathBuf {
    if let Some(rest) = input.strip_prefix("~/")
        && let Some(home) = std::env::var_os("HOME")
    {
        return PathBuf::from(home).join(rest);
    }
    PathBuf::from(input)
}

/// Reads an identity from text: an exported INI file, or the `<key>:<offset>` string.
pub fn parse(text: &str) -> Result<Imported> {
    let text = text.trim();
    ensure!(!text.is_empty(), "there is nothing to import");
    if let Some(value) = field(text, "identity") {
        return Ok(Imported {
            identity: from_export(&value)?,
            label: field(text, "id"),
            nickname: field(text, "nickname"),
        });
    }
    let identity = ts3proto::identity_from_string(text)
        .map_err(|_| anyhow!("this is neither an exported identity nor a <key>:<offset> string"))?;
    usable(&identity)?;
    Ok(Imported {
        identity,
        label: None,
        nickname: None,
    })
}

/// The value `name` carries in an exported file.
fn field(text: &str, name: &str) -> Option<String> {
    text.lines()
        .filter_map(|line| line.trim().strip_prefix(name)?.strip_prefix('='))
        .map(|value| value.trim().trim_matches('"').trim().to_owned())
        .find(|value| !value.is_empty())
}

/// Reads the `<counter>V<obfuscated key pair>` an exported file carries.
fn from_export(value: &str) -> Result<ts3proto::Identity> {
    let digits = value.bytes().take_while(u8::is_ascii_digit).count();
    ensure!(digits > 0, "the identity does not start with a counter");
    let (counter, rest) = value.split_at(digits);
    counter
        .parse::<i64>()
        .context("the identity's counter is not a number")?;
    let obfuscated = rest
        .strip_prefix('V')
        .context("the identity has no V after its counter")?;
    let mut data = decode(obfuscated).context("the identity is not base64")?;
    ensure!(
        data.len() > HASH_LEN,
        "the identity is too short to hold a key"
    );
    deobfuscate(&mut data);
    let inner = std::str::from_utf8(&data)
        .ok()
        .context("the identity does not hold a key")?;
    let der = decode(inner).context("the key inside the identity is not base64")?;
    let (point, key) = point_and_key(&der)?;
    let identity = ts3proto::identity_from_string(&format!("{}:{counter}", STANDARD.encode(key)))
        .map_err(|_| anyhow!("the key inside the identity is not a key"))?;
    usable(&identity)?;
    ensure!(
        identity.public_key_bytes()[1..] == point[..],
        "the key does not match the public key beside it"
    );
    Ok(identity)
}

/// Lifts the obfuscation the TeamSpeak client puts over an exported identity.
fn deobfuscate(data: &mut [u8]) {
    let tail = &data[HASH_LEN..];
    let end = tail.iter().position(|&b| b == 0).unwrap_or(tail.len());
    let hash = Sha1::digest(&tail[..end]);
    for (i, byte) in data.iter_mut().take(HASH_LEN).enumerate() {
        *byte ^= hash[i];
    }
    for (i, byte) in data.iter_mut().take(OBFUSCATION_KEY.len()).enumerate() {
        *byte ^= OBFUSCATION_KEY[i];
    }
}

/// Base64 the TeamSpeak client writes with padding in one place and without it in another.
fn decode(text: &str) -> Option<Vec<u8>> {
    STANDARD_NO_PAD.decode(text.trim_end_matches('=')).ok()
}

/// The point and the private key a TeamSpeak key pair holds, as the last three numbers of its DER.
fn point_and_key(der: &[u8]) -> Result<([u8; SCALAR * 2], [u8; SCALAR])> {
    let numbers = numbers(der)?;
    ensure!(numbers.len() >= 4, "this is not a TeamSpeak key pair");
    let [x, y, key] = [3, 2, 1].map(|back: usize| numbers[numbers.len() - back]);
    let mut point = [0u8; SCALAR * 2];
    point[..SCALAR].copy_from_slice(&scalar(x)?);
    point[SCALAR..].copy_from_slice(&scalar(y)?);
    Ok((point, scalar(key)?))
}

/// Every number in a DER sequence, in the order the sequence holds them.
fn numbers(der: &[u8]) -> Result<Vec<&[u8]>> {
    ensure!(der.first() == Some(&0x30), "the key is not a DER sequence");
    let mut at = 1;
    let len = length(der, &mut at)?;
    let end = at
        .checked_add(len)
        .filter(|end| *end <= der.len())
        .context("the key is truncated")?;
    let mut found = Vec::new();
    while at < end {
        let tag = der[at];
        at += 1;
        let len = length(der, &mut at)?;
        let body = der
            .get(at..at + len)
            .context("the key ends inside a number")?;
        at += len;
        if tag == 0x02 {
            found.push(body);
        }
    }
    Ok(found)
}

/// The length a DER item declares, reading on from `at`.
fn length(der: &[u8], at: &mut usize) -> Result<usize> {
    let first = *der.get(*at).context("the key is truncated")?;
    *at += 1;
    let len = match first {
        0x81 => {
            let len = *der.get(*at).context("the key is truncated")? as usize;
            *at += 1;
            len
        }
        0x82 => {
            let high = *der.get(*at).context("the key is truncated")? as usize;
            let low = *der.get(*at + 1).context("the key is truncated")? as usize;
            *at += 2;
            (high << 8) | low
        }
        short if short < 0x80 => short as usize,
        _ => bail!("the key uses a length this does not read"),
    };
    Ok(len)
}

/// A DER number as the fixed-width number the curve works with.
fn scalar(value: &[u8]) -> Result<[u8; SCALAR]> {
    let trimmed = match value.iter().position(|&b| b != 0) {
        Some(at) => &value[at..],
        None => &[][..],
    };
    ensure!(
        trimmed.len() <= SCALAR,
        "a number in the key is too long for the curve"
    );
    let mut out = [0u8; SCALAR];
    out[SCALAR - trimmed.len()..].copy_from_slice(trimmed);
    Ok(out)
}

/// Whether the key is one the curve accepts. Deriving a public key from one it does not would end
/// the program.
fn usable(identity: &ts3proto::Identity) -> Result<()> {
    p256::SecretKey::from_slice(&identity.private_key)
        .map(|_| ())
        .map_err(|_| anyhow!("the private key is not a number of the curve"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn der_len(len: usize) -> Vec<u8> {
        if len < 0x80 {
            vec![len as u8]
        } else if len < 0x100 {
            vec![0x81, len as u8]
        } else {
            vec![0x82, (len >> 8) as u8, (len & 0xff) as u8]
        }
    }

    fn der_int(value: &[u8]) -> Vec<u8> {
        let trimmed = match value.iter().position(|&b| b != 0) {
            Some(at) => &value[at..],
            None => &[0][..],
        };
        let mut body = Vec::new();
        if trimmed[0] & 0x80 != 0 {
            body.push(0);
        }
        body.extend_from_slice(trimmed);
        let mut der = vec![0x02];
        der.extend_from_slice(&der_len(body.len()));
        der.extend_from_slice(&body);
        der
    }

    /// The key pair as the TeamSpeak client writes it before obfuscation.
    fn keypair_der(identity: &ts3proto::Identity) -> Vec<u8> {
        let point = identity.public_key_bytes();
        let mut body = vec![0x03, 0x02, 0x07, 0x80];
        body.extend_from_slice(&der_int(&[32]));
        body.extend_from_slice(&der_int(&point[1..33]));
        body.extend_from_slice(&der_int(&point[33..65]));
        body.extend_from_slice(&der_int(&identity.private_key));
        let mut der = vec![0x30];
        der.extend_from_slice(&der_len(body.len()));
        der.extend_from_slice(&body);
        der
    }

    /// The inverse of what [`deobfuscate`] does, so a test can write what the client would.
    fn obfuscate(plain: &[u8]) -> String {
        let mut data = plain.to_vec();
        for (i, byte) in data.iter_mut().take(OBFUSCATION_KEY.len()).enumerate() {
            *byte ^= OBFUSCATION_KEY[i];
        }
        let tail = &data[HASH_LEN..];
        let end = tail.iter().position(|&b| b == 0).unwrap_or(tail.len());
        let hash = Sha1::digest(&tail[..end]);
        for (i, byte) in data.iter_mut().take(HASH_LEN).enumerate() {
            *byte ^= hash[i];
        }
        STANDARD.encode(data)
    }

    /// An exported file, written the way the TeamSpeak client writes one.
    fn export_with(der: &[u8], offset: i64, label: &str, nickname: &str) -> String {
        let inner = STANDARD_NO_PAD.encode(der);
        format!(
            "[Identity]\nid={label}\nidentity=\"{offset}V{}\"\nnickname={nickname}\nphonetic_nickname=\n",
            obfuscate(inner.as_bytes())
        )
    }

    fn export(identity: &ts3proto::Identity, label: &str, nickname: &str) -> String {
        export_with(&keypair_der(identity), identity.offset, label, nickname)
    }

    #[test]
    fn reads_the_plain_identity_string() {
        let identity = ts3proto::generate_identity(0);
        let read = parse(&identity.to_string()).expect("the string is an identity");
        assert_eq!(
            read.identity.public_key_base64(),
            identity.public_key_base64()
        );
        assert_eq!(read.identity.offset, identity.offset);
        assert!(read.label.is_none());
        assert!(read.nickname.is_none());
    }

    #[test]
    fn reads_an_exported_file() {
        let identity = ts3proto::generate_identity(8);
        let read =
            parse(&export(&identity, "Main", "Julian | Newspicel")).expect("the file is an export");
        assert_eq!(
            read.identity.public_key_base64(),
            identity.public_key_base64()
        );
        assert_eq!(read.identity.security_level(), identity.security_level());
        assert_eq!(read.label.as_deref(), Some("Main"));
        assert_eq!(read.nickname.as_deref(), Some("Julian | Newspicel"));
    }

    #[test]
    fn reads_a_file_the_path_points_at() {
        let identity = ts3proto::generate_identity(0);
        let path = std::env::temp_dir().join(format!("moqspeak-import-{}.ini", std::process::id()));
        std::fs::write(&path, export(&identity, "Main", "Julian")).expect("write the export");
        let read =
            from_input(&format!(" {} ", path.display())).expect("the path leads to an export");
        let _ = std::fs::remove_file(&path);
        assert_eq!(
            read.identity.public_key_base64(),
            identity.public_key_base64()
        );
    }

    #[test]
    fn rejects_text_that_is_not_an_identity() {
        assert!(parse("hello world").is_err());
        assert!(parse("").is_err());
        assert!(parse("[Identity]\nid=Main\n").is_err());
    }

    #[test]
    fn rejects_an_export_whose_key_does_not_match_its_public_key() {
        let identity = ts3proto::generate_identity(0);
        let mut der = keypair_der(&identity);
        let last = der.len() - 1;
        der[last] ^= 0x01;
        assert!(parse(&export_with(&der, identity.offset, "Main", "Julian")).is_err());
    }

    #[test]
    fn rejects_a_key_that_is_not_a_scalar_of_the_curve() {
        let zero = format!("{}:0", STANDARD.encode([0u8; SCALAR]));
        assert!(parse(&zero).is_err());
    }
}
