//! What survives a restart: the nickname, the servers, the audio choices and the look.

use serde::{Deserialize, Serialize};

use crate::audio::VoiceMode;
use crate::ui::theme::{Scheme, Variant};

/// The server a new installation offers.
pub const DEFAULT_ADDRESS: &str = "moq.newspicel.dev/public";

/// How wide the chat column opens at first, and the range a drag keeps it in, in CSS pixels.
pub const CHAT_WIDTH: f32 = 360.0;
pub const CHAT_MIN: f32 = 260.0;
pub const CHAT_MAX: f32 = 720.0;

/// A saved server.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Bookmark {
    pub label: String,
    pub address: String,
    pub nickname: String,
}

/// The preferences file.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub nickname: String,
    pub address: String,
    pub bookmarks: Vec<Bookmark>,
    pub voice_mode: String,
    pub threshold_db: f32,
    pub input_gain: f32,
    pub output_volume: f32,
    /// Whether the interface plays sounds.
    pub sounds: bool,
    /// Linear volume of the interface sounds.
    pub sound_volume: f32,
    pub input_device: Option<String>,
    pub output_device: Option<String>,
    /// The surface: "system", "dark" or "light".
    pub theme_mode: String,
    /// The theme name, as `Variant::name` writes it.
    pub theme: String,
    pub noise_suppression: bool,
    pub smart_vad: bool,
    pub echo_cancellation: bool,
    /// How wide the chat column stands open, in CSS pixels.
    pub chat_width: f32,
}

impl Default for Settings {
    fn default() -> Self {
        let user = std::env::var("USER").unwrap_or_else(|_| "Guest".into());
        let mut nickname: String = user.chars().take(1).flat_map(char::to_uppercase).collect();
        nickname.extend(user.chars().skip(1));
        Self {
            nickname,
            address: DEFAULT_ADDRESS.into(),
            bookmarks: vec![Bookmark {
                label: "moqspeak Public".into(),
                address: DEFAULT_ADDRESS.into(),
                nickname: String::new(),
            }],
            voice_mode: "activation".into(),
            threshold_db: -50.0,
            input_gain: 1.0,
            output_volume: 1.0,
            sounds: true,
            sound_volume: 0.6,
            input_device: None,
            output_device: None,
            theme_mode: Scheme::Dark.name().into(),
            theme: Variant::default().name().into(),
            noise_suppression: true,
            smart_vad: true,
            echo_cancellation: true,
            chat_width: CHAT_WIDTH,
        }
    }
}

impl Settings {
    fn path() -> Option<std::path::PathBuf> {
        directories::ProjectDirs::from("dev", "moqspeak", "moqspeak")
            .map(|d| d.config_dir().join("settings.json"))
    }

    pub fn load() -> Self {
        Self::path()
            .and_then(|p| std::fs::read(p).ok())
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) {
        let Some(path) = Self::path() else { return };
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(json) = serde_json::to_vec_pretty(self) {
            let _ = std::fs::write(path, json);
        }
    }

    /// The label a bookmark for `address` carries, if one exists.
    pub fn bookmark_of(&self, address: &str) -> Option<&Bookmark> {
        self.bookmarks.iter().find(|b| b.address == address)
    }
}

pub fn mode_from_str(s: &str) -> VoiceMode {
    match s {
        "ptt" => VoiceMode::PushToTalk,
        "continuous" => VoiceMode::Continuous,
        _ => VoiceMode::Activation,
    }
}

pub fn mode_to_str(m: VoiceMode) -> &'static str {
    match m {
        VoiceMode::Activation => "activation",
        VoiceMode::PushToTalk => "ptt",
        VoiceMode::Continuous => "continuous",
    }
}
