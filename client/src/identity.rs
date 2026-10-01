//! Who this client is: an Ed25519 key pair. The public key is the identity servers attach roles
//! to; signing their challenge proves it.

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use ed25519_dalek::{Signer, SigningKey};

pub struct Identity {
    key: SigningKey,
}

impl Identity {
    fn path() -> Option<std::path::PathBuf> {
        directories::ProjectDirs::from("dev", "moqspeak", "moqspeak")
            .map(|d| d.config_dir().join("identity.key"))
    }

    /// The identity saved on this machine, created on first use.
    pub fn load_or_create() -> Self {
        let path = Self::path();
        if let Some(seed) = path
            .as_ref()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|text| URL_SAFE_NO_PAD.decode(text.trim()).ok())
            .and_then(|bytes| <[u8; 32]>::try_from(bytes).ok())
        {
            return Self {
                key: SigningKey::from_bytes(&seed),
            };
        }
        let identity = Self::ephemeral();
        if let Some(path) = path {
            if let Some(dir) = path.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            let _ = std::fs::write(&path, URL_SAFE_NO_PAD.encode(identity.key.to_bytes()));
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
            }
        }
        identity
    }

    /// A fresh identity that is not saved, for the headless bot.
    pub fn ephemeral() -> Self {
        let mut seed = [0u8; 32];
        getrandom::fill(&mut seed).expect("the system random source");
        Self {
            key: SigningKey::from_bytes(&seed),
        }
    }

    /// The public key, base64url without padding: 43 characters.
    pub fn uid(&self) -> String {
        URL_SAFE_NO_PAD.encode(self.key.verifying_key().to_bytes())
    }

    /// Answers a server's challenge.
    pub fn answer(&self, server: &str, nonce: &str) -> String {
        let message = format!("moqspeak-auth:{server}:{nonce}");
        URL_SAFE_NO_PAD.encode(self.key.sign(message.as_bytes()).to_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signature, Verifier, VerifyingKey};

    #[test]
    fn answers_verify_against_the_uid() {
        let id = Identity::ephemeral();
        let uid = id.uid();
        assert_eq!(uid.len(), 43);
        let sig = id.answer("demo", "abc");
        let key =
            VerifyingKey::from_bytes(&URL_SAFE_NO_PAD.decode(&uid).unwrap().try_into().unwrap())
                .unwrap();
        let sig = Signature::from_slice(&URL_SAFE_NO_PAD.decode(sig).unwrap()).unwrap();
        assert!(key.verify(b"moqspeak-auth:demo:abc", &sig).is_ok());
    }
}
