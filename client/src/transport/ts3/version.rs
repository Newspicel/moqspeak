//! The client version `clientinit` presents. TeamSpeak signs one version string for a release,
//! and every platform's client of that release carries that same signature, so only the platform
//! name differs between them. The version and the signature here are read from the official 3.6.2
//! clients, whose macOS, Linux and Windows builds all hold the same one.

use ts3proto::VersionSign;

/// The release every official 3.6.2 client reports.
const VERSION: &str = "3.6.2 [Build: 1695203293]";

/// What TeamSpeak wrote over that release.
const SIGN: &str =
    "JEhuVodiWr2/F9mixBcaAZTtjx4Rs9cJDLbpEG8i7hPKswcFdsn6MWwINP+Nwmw4AEPpVJevUEvRQbqVMVoLlw==";

/// The names TeamSpeak gives the systems it builds for.
const WINDOWS: &str = "Windows";
const LINUX: &str = "Linux";
const MACOS: &str = "OS X";
const IOS: &str = "iOS";
const ANDROID: &str = "Android";

/// The version to present on the system this client runs on.
pub fn for_this_system() -> VersionSign {
    VersionSign {
        version: VERSION.to_owned(),
        platform: platform().to_owned(),
        sign: SIGN.to_owned(),
    }
}

/// What TeamSpeak calls the system this client runs on.
fn platform() -> &'static str {
    // A development aid for checking what a server makes of each one:
    // MOQSPEAK_TS3_PLATFORM=Linux.
    if let Ok(forced) = std::env::var("MOQSPEAK_TS3_PLATFORM")
        && let Some(named) = named(&forced)
    {
        return named;
    }
    match std::env::consts::OS {
        "windows" => WINDOWS,
        "macos" => MACOS,
        "ios" => IOS,
        "android" => ANDROID,
        // Every other system this client builds for is a Unix that TeamSpeak would call Linux.
        _ => LINUX,
    }
}

/// The platform TeamSpeak names `wanted`, whatever case it is written in.
fn named(wanted: &str) -> Option<&'static str> {
    [WINDOWS, LINUX, MACOS, IOS, ANDROID]
        .into_iter()
        .find(|name| name.eq_ignore_ascii_case(wanted.trim()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;

    #[test]
    fn the_signature_is_a_whole_ed25519_signature() {
        let raw = base64::engine::general_purpose::STANDARD
            .decode(SIGN)
            .expect("the signature is base64");
        assert_eq!(raw.len(), 64);
    }

    #[test]
    fn this_system_gets_a_name_teamspeak_knows() {
        let version = for_this_system();
        assert_eq!(version.version, VERSION);
        assert!(named(&version.platform).is_some(), "{}", version.platform);
    }

    #[test]
    fn macos_is_the_name_teamspeak_uses_for_it() {
        // TeamSpeak's own clients call it `OS X`, and a server takes no other name for it.
        assert_eq!(named("os x"), Some(MACOS));
        assert_eq!(named("macOS"), None);
    }

    #[test]
    fn a_forced_platform_is_matched_loosely() {
        assert_eq!(named("linux"), Some(LINUX));
        assert_eq!(named(" Windows "), Some(WINDOWS));
        assert_eq!(named("Haiku"), None);
    }
}
