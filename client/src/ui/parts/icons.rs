//! The icon set: Lucide (ISC licence, see `assets/icons/LICENSE`), and the component that draws
//! one.

use zgui::prelude::*;
use zgui::reactive::LocalStorage;

/// Declares one constant per icon. Each is a Lucide SVG drawn with `currentColor`.
macro_rules! icons {
    ($($name:ident => $file:literal,)*) => {
        $(
            #[doc = concat!("The `", $file, "` icon.")]
            pub const $name: &str = include_str!(concat!("../../../assets/icons/", $file, ".svg"));
        )*
    };
}

icons! {
    AUDIO_LINES => "audio-lines",
    CHEVRON_DOWN => "chevron-down",
    CHEVRON_RIGHT => "chevron-right",
    COPY => "copy",
    ERASER => "eraser",
    FOLDER_PLUS => "folder-plus",
    HASH => "hash",
    HEADPHONE_OFF => "headphone-off",
    HEADPHONES => "headphones",
    HOUSE => "house",
    INFO => "info",
    LOADER_CIRCLE => "loader-circle",
    MESSAGE_SQUARE => "message-square",
    MIC => "mic",
    MIC_OFF => "mic-off",
    MINUS => "minus",
    MONITOR => "monitor",
    MOON => "moon",
    PALETTE => "palette",
    SCREEN_SHARE => "screen-share",
    SCREEN_SHARE_OFF => "screen-share-off",
    SCROLL_TEXT => "scroll-text",
    SETTINGS => "settings",
    SQUARE => "square",
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
