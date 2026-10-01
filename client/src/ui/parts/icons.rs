//! The icon set: Lucide (ISC licence, see `assets/icons/LICENSE`), and the component that draws
//! one.

use zgui::prelude::*;
use zgui::reactive::LocalStorage;

/// Declares one constant per icon. Each is a Lucide SVG drawn with `currentColor`.
macro_rules! icons {
    ($($(#[$meta:meta])* $name:ident => $file:literal,)*) => {
        $(
            #[doc = concat!("The `", $file, "` icon.")]
            $(#[$meta])*
            pub const $name: &str = include_str!(concat!("../../../assets/icons/", $file, ".svg"));
        )*
    };
}

icons! {
    ARROW_RIGHT_LEFT => "arrow-right-left",
    AUDIO_LINES => "audio-lines",
    CHEVRONS_DOWN_UP => "chevrons-down-up",
    CHEVRONS_UP_DOWN => "chevrons-up-down",
    CHEVRON_DOWN => "chevron-down",
    CHEVRON_RIGHT => "chevron-right",
    COPY => "copy",
    EAR => "ear",
    ERASER => "eraser",
    FOLDER_PLUS => "folder-plus",
    GLOBE => "globe",
    HAND => "hand",
    HASH => "hash",
    HEADPHONES => "headphones",
    HEADPHONE_OFF => "headphone-off",
    HOUSE => "house",
    INFO => "info",
    LOADER_CIRCLE => "loader-circle",
    LOG_IN => "log-in",
    MESSAGE_SQUARE => "message-square",
    MIC => "mic",
    MIC_OFF => "mic-off",
    MINUS => "minus",
    #[cfg(feature = "screen-share")]
    MONITOR => "monitor",
    #[cfg(feature = "screen-share")]
    MONITOR_PLAY => "monitor-play",
    MOON => "moon",
    PALETTE => "palette",
    PLUG => "plug",
    #[cfg(feature = "screen-share")]
    SCREEN_SHARE => "screen-share",
    #[cfg(feature = "screen-share")]
    SCREEN_SHARE_OFF => "screen-share-off",
    SCROLL_TEXT => "scroll-text",
    SEND_HORIZONTAL => "send-horizontal",
    SETTINGS => "settings",
    SHIELD_USER => "shield-user",
    SQUARE => "square",
    STAR => "star",
    TRASH_2 => "trash-2",
    UNPLUG => "unplug",
    USERS => "users",
    USER_X => "user-x",
    VOLUME_2 => "volume-2",
    VOLUME_X => "volume-x",
    X => "x",
}

/// How large an icon is drawn. Each step reads one `--zui-icon-*` variable.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum IconSize {
    Xs,
    Sm,
    #[default]
    Md,
}

impl IconSize {
    fn token(self) -> &'static str {
        match self {
            IconSize::Xs => "xs",
            IconSize::Sm => "sm",
            IconSize::Md => "md",
        }
    }
}

/// Draws one icon in the colour of the element around it.
///
/// Give a label to an icon that carries meaning on its own. Leave it out beside text that says
/// the same thing, and the icon stays out of the accessibility tree.
#[component]
pub fn Icon(
    /// The document to draw.
    #[prop(into)]
    svg: Signal<&'static str, LocalStorage>,
    /// How large to draw it.
    #[prop(default = IconSize::Md)]
    size: IconSize,
    /// Classes merged after the icon's own.
    #[prop(into, optional)]
    class: Classes,
) -> impl IntoView {
    view! {
        vector(
            class = "ms-icon",
            class = class,
            attr:data-size = size.token(),
            prop:svg = move || PropValue::from(svg.get()),
            a11y:hidden = true
        )
    }
}
