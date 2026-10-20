//! The client version `clientinit` presents. TeamSpeak signs the version string and the platform
//! together, so a version, a platform and a signature only ever work as one whole triple. Each one
//! here is read from an official client, and a server accepts no other.

use ts3proto::VersionSign;

/// One triple, as an official client sends it.
struct Known {
    version: &'static str,
    platform: &'static str,
    sign: &'static str,
}

/// TeamSpeak 3.6.2 for macOS, built 2023-09-20. TeamSpeak calls this platform `OS X`.
const MACOS: Known = Known {
    version: "3.6.2 [Build: 1695203293]",
    platform: "OS X",
    sign: "JEhuVodiWr2/F9mixBcaAZTtjx4Rs9cJDLbpEG8i7hPKswcFdsn6MWwINP+Nwmw4AEPpVJevUEvRQbqVMVoLlw==",
};

/// The triple third-party clients have long presented on Windows.
const WINDOWS: Known = Known {
    version: "3.?.? [Build: 5680278000]",
    platform: "Windows",
    sign: "DX5NIYLvfJEUjuIbCidnoeozxIDRRkpq3I9vVMBmE9L2qnekOoBzSenkzsg2lC9CMv8K5hkEzhr2TYUYSwUXCg==",
};

/// The version to present on the system this client runs on. A system with no triple of its own
/// borrows the Windows one, which every server accepts.
pub fn for_this_system() -> VersionSign {
    let known = match std::env::consts::OS {
        "macos" => MACOS,
        _ => WINDOWS,
    };
    VersionSign {
        version: known.version.to_owned(),
        platform: known.platform.to_owned(),
        sign: known.sign.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;

    /// Every signature is 64 bytes, which is what Ed25519 writes.
    #[test]
    fn every_signature_is_a_whole_ed25519_signature() {
        for known in [MACOS, WINDOWS] {
            let raw = base64::engine::general_purpose::STANDARD
                .decode(known.sign)
                .expect("the signature is base64");
            assert_eq!(raw.len(), 64, "{}", known.version);
        }
    }

    /// TeamSpeak names its platforms from a fixed set.
    #[test]
    fn every_platform_is_one_teamspeak_names() {
        const NAMED: [&str; 5] = ["Windows", "Linux", "OS X", "iOS", "Android"];
        for known in [MACOS, WINDOWS] {
            assert!(NAMED.contains(&known.platform), "{}", known.platform);
        }
    }

    #[test]
    fn this_system_gets_a_whole_triple() {
        let version = for_this_system();
        assert!(!version.version.is_empty());
        assert!(!version.platform.is_empty());
        assert!(!version.sign.is_empty());
        if std::env::consts::OS == "macos" {
            assert_eq!(version.platform, "OS X");
        }
    }
}
