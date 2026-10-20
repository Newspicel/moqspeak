//! The TeamSpeak identity saved on this machine: a P-256 key pair whose public key is this
//! client's unique id. A server may ask for a minimum security level, which is proof of work over
//! that key.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

/// The level a stock TeamSpeak server asks for. Reaching it costs a moment once, on first use.
const SECURITY_LEVEL: i32 = 8;

/// Where the identity is kept.
pub fn path() -> Option<PathBuf> {
    directories::ProjectDirs::from("dev", "moqspeak", "moqspeak")
        .map(|d| d.config_dir().join("teamspeak.identity"))
}

/// The TeamSpeak identity saved on this machine, created on first use. Keeping it means a server
/// recognises this client again, and keeps the groups it granted.
pub fn load_or_create() -> ts3proto::Identity {
    if let Some(identity) = saved() {
        return identity;
    }
    let path = path();
    let identity = ts3proto::generate_identity(SECURITY_LEVEL);
    if let Some(path) = path {
        let _ = write(&path, &identity);
    }
    identity
}

/// The identity this machine keeps, if there is one. Nothing is created.
pub fn saved() -> Option<ts3proto::Identity> {
    path().as_deref().and_then(read)
}

/// The identity at `path`, if one is there.
pub fn read(path: &Path) -> Option<ts3proto::Identity> {
    let text = std::fs::read_to_string(path).ok()?;
    ts3proto::identity_from_string(text.trim()).ok()
}

/// Writes `identity` to `path`, readable by this account alone.
pub fn write(path: &Path, identity: &ts3proto::Identity) -> Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
    }
    std::fs::write(path, identity.to_string())
        .with_context(|| format!("writing {}", path.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
            .with_context(|| format!("sealing {}", path.display()))?;
    }
    Ok(())
}

/// Puts `identity` in place of the saved one. The identity that was there is kept beside it,
/// because losing one loses the groups every server granted it.
pub fn replace(identity: &ts3proto::Identity) -> Result<()> {
    let path = path().context("this system has no configuration directory")?;
    replace_at(&path, identity)
}

/// Puts `identity` at `path`, keeping what was there as `<name>.bak`.
fn replace_at(path: &Path, identity: &ts3proto::Identity) -> Result<()> {
    if path.exists() {
        let kept = PathBuf::from(format!("{}.bak", path.display()));
        std::fs::copy(path, &kept).with_context(|| format!("keeping {}", kept.display()))?;
    }
    write(path, identity)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("moqspeak-store-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a scratch directory");
        dir
    }

    #[test]
    fn reads_back_what_it_wrote() {
        let dir = scratch("round-trip");
        let path = dir.join("teamspeak.identity");
        let identity = ts3proto::generate_identity(0);
        write(&path, &identity).expect("write the identity");
        let read = read(&path).expect("read the identity");
        assert_eq!(read.to_string(), identity.to_string());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn keeps_the_previous_identity_beside_the_new_one() {
        let dir = scratch("backup");
        let path = dir.join("teamspeak.identity");
        let old = ts3proto::generate_identity(0);
        let new = ts3proto::generate_identity(0);
        write(&path, &old).expect("write the first identity");
        replace_at(&path, &new).expect("replace it");
        assert_eq!(
            read(&path).expect("the new identity").to_string(),
            new.to_string()
        );
        let kept = read(&dir.join("teamspeak.identity.bak")).expect("the kept identity");
        assert_eq!(kept.to_string(), old.to_string());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn replaces_an_identity_that_is_not_there_yet() {
        let dir = scratch("fresh");
        let path = dir.join("teamspeak.identity");
        let identity = ts3proto::generate_identity(0);
        replace_at(&path, &identity).expect("write the identity");
        assert_eq!(
            read(&path).expect("the identity").to_string(),
            identity.to_string()
        );
        assert!(
            !dir.join("teamspeak.identity.bak").exists(),
            "nothing was there to keep"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
