//! How the connection is, as a status tone.

use crate::engine::{ConnStatus, MediaStatus};

/// The tone of the connection mark, and whether it pulses.
pub fn connection_tone(status: &ConnStatus, media: &MediaStatus) -> (&'static str, bool) {
    match status {
        ConnStatus::Connected => match media {
            MediaStatus::Connected { .. } | MediaStatus::Off => ("ok", false),
            MediaStatus::Connecting => ("ok", true),
            MediaStatus::Failed(_) => ("warn", false),
        },
        ConnStatus::Connecting(_) => ("warn", true),
        ConnStatus::Failed(_) => ("err", false),
        ConnStatus::Disconnected => ("idle", false),
    }
}

/// What the voice path is doing, in words.
pub fn voice_words(media: &MediaStatus) -> String {
    match media {
        MediaStatus::Off => "Voice off".into(),
        MediaStatus::Connecting => "Voice connecting".into(),
        MediaStatus::Connected { relay, .. } => format!("Voice via {relay}"),
        MediaStatus::Failed(e) => format!("Voice unavailable: {e}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_connection_reads_as_its_tone() {
        let off = MediaStatus::Off;
        assert_eq!(connection_tone(&ConnStatus::Connected, &off), ("ok", false));
        assert_eq!(
            connection_tone(&ConnStatus::Connecting("x".into()), &off),
            ("warn", true)
        );
        assert_eq!(
            connection_tone(&ConnStatus::Failed("x".into()), &off),
            ("err", false)
        );
        assert_eq!(
            connection_tone(&ConnStatus::Disconnected, &off),
            ("idle", false)
        );
        assert_eq!(
            connection_tone(&ConnStatus::Connected, &MediaStatus::Failed("x".into())),
            ("warn", false)
        );
    }
}
