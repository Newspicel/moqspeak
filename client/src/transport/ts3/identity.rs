//! The TeamSpeak identity: a P-256 key pair whose public key is this client's unique id. A server
//! may ask for a minimum security level, which is proof of work over that key.

use std::path::PathBuf;

/// The level a stock TeamSpeak server asks for. Reaching it costs a moment once, on first use.
const SECURITY_LEVEL: i32 = 8;

fn path() -> Option<PathBuf> {
    directories::ProjectDirs::from("dev", "moqspeak", "moqspeak")
        .map(|d| d.config_dir().join("teamspeak.identity"))
}

/// The TeamSpeak identity saved on this machine, created on first use. Keeping it means a server
/// recognises this client again, and keeps the groups it granted.
pub fn load_or_create() -> ts3proto::Identity {
    let path = path();
    if let Some(identity) = path
        .as_ref()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|text| ts3proto::identity_from_string(text.trim()).ok())
    {
        return identity;
    }
    let identity = ts3proto::generate_identity(SECURITY_LEVEL);
    if let Some(path) = path {
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(&path, identity.to_string());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
        }
    }
    identity
}
