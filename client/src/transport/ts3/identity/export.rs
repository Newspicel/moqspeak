//! Writes an identity as the INI file the TeamSpeak client imports.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use base64::Engine as _;
use base64::engine::general_purpose::{STANDARD, STANDARD_NO_PAD};

use super::{obfuscate, store};

/// The name an export carries when there is no nickname to put in it.
const FALLBACK: &str = "moqspeak";

/// What an export is called.
const FILE: &str = "moqspeak-identity.ini";

/// The identity as the INI file the TeamSpeak client imports. `nickname` names it there.
fn ini(identity: &ts3proto::Identity, nickname: &str) -> String {
    let name = match nickname.trim() {
        "" => FALLBACK,
        given => given,
    };
    let mut data = STANDARD_NO_PAD.encode(keypair(identity)).into_bytes();
    obfuscate::apply(&mut data);
    format!(
        "[Identity]\nid={name}\nidentity=\"{}V{}\"\nnickname={name}\nphonetic_nickname=\n",
        identity.offset,
        STANDARD.encode(&data)
    )
}

/// The key pair as the DER sequence an exported identity carries.
pub(super) fn keypair(identity: &ts3proto::Identity) -> Vec<u8> {
    let point = identity.public_key_bytes();
    let mut body = vec![0x03, 0x02, 0x07, 0x80];
    body.extend_from_slice(&number(&[32]));
    body.extend_from_slice(&number(&point[1..33]));
    body.extend_from_slice(&number(&point[33..65]));
    body.extend_from_slice(&number(&identity.private_key));
    let mut der = vec![0x30];
    der.extend_from_slice(&length(body.len()));
    der.extend_from_slice(&body);
    der
}

/// A DER length, in the shortest form that holds it.
fn length(len: usize) -> Vec<u8> {
    match len {
        short if short < 0x80 => vec![short as u8],
        one if one < 0x100 => vec![0x81, one as u8],
        two => vec![0x82, (two >> 8) as u8, (two & 0xff) as u8],
    }
}

/// A DER number. A leading zero keeps a high first bit from making it negative.
fn number(value: &[u8]) -> Vec<u8> {
    let trimmed = match value.iter().position(|&b| b != 0) {
        Some(at) => &value[at..],
        None => &[0][..],
    };
    let mut body = Vec::with_capacity(trimmed.len() + 1);
    if trimmed[0] & 0x80 != 0 {
        body.push(0);
    }
    body.extend_from_slice(trimmed);
    let mut der = vec![0x02];
    der.extend_from_slice(&length(body.len()));
    der.extend_from_slice(&body);
    der
}

/// Writes the export into the download directory. Gives back where it went.
pub fn to_downloads(identity: &ts3proto::Identity, nickname: &str) -> Result<PathBuf> {
    let dir = directories::UserDirs::new()
        .and_then(|dirs| dirs.download_dir().map(Path::to_path_buf))
        .context("this system has no download directory")?;
    let path = dir.join(FILE);
    write(&path, identity, nickname)?;
    Ok(path)
}

/// Writes the export to `path`, readable by this account alone.
fn write(path: &Path, identity: &ts3proto::Identity, nickname: &str) -> Result<()> {
    std::fs::write(path, ini(identity, nickname))
        .with_context(|| format!("writing {}", path.display()))?;
    store::seal(path)
}

#[cfg(test)]
mod tests {
    use super::super::import;
    use super::*;

    #[test]
    fn writes_a_file_the_importer_reads_back() {
        let identity = ts3proto::generate_identity(0);
        let read = import::parse(&ini(&identity, "Julian")).expect("the export is an identity");
        assert_eq!(
            read.identity.public_key_base64(),
            identity.public_key_base64()
        );
        assert_eq!(read.identity.offset, identity.offset);
        assert_eq!(read.label.as_deref(), Some("Julian"));
        assert_eq!(read.nickname.as_deref(), Some("Julian"));
    }

    #[test]
    fn keeps_the_level_the_search_reached() {
        let identity = ts3proto::generate_identity(12);
        let read = import::parse(&ini(&identity, "Julian")).expect("the export is an identity");
        assert_eq!(read.identity.offset, identity.offset);
        assert_eq!(read.identity.security_level(), identity.security_level());
        assert!(read.identity.security_level() >= 12);
    }

    #[test]
    fn names_an_export_that_has_no_nickname() {
        let identity = ts3proto::generate_identity(0);
        let read = import::parse(&ini(&identity, "   ")).expect("the export is an identity");
        assert_eq!(read.label.as_deref(), Some(FALLBACK));
        assert_eq!(read.nickname.as_deref(), Some(FALLBACK));
    }

    #[test]
    fn writes_a_file_only_this_account_reads() {
        let dir = std::env::temp_dir().join(format!("moqspeak-export-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("a scratch directory");
        let path = dir.join(FILE);
        let identity = ts3proto::generate_identity(0);
        write(&path, &identity, "Julian").expect("write the export");
        let read = import::from_input(&path.display().to_string()).expect("read the export");
        assert_eq!(
            read.identity.public_key_base64(),
            identity.public_key_base64()
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&path).expect("the export").permissions();
            assert_eq!(mode.mode() & 0o777, 0o600);
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
}
