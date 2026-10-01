//! The sheet the whole window is drawn by.
//!
//! The rules live in `assets/css`, one file per region. They are joined in cascade order:
//! measurements first, then the window frame, then the regions inside it, then motion.

/// Builds one sheet constant per file and the list that joins them.
macro_rules! sheets {
    ($($name:ident => $file:literal,)*) => {
        $(
            #[doc = concat!("The rules of `", $file, ".css`.")]
            const $name: &str = include_str!(concat!("../../assets/css/", $file, ".css"));
        )*

        /// Every sheet, in cascade order.
        const ALL: &[&str] = &[$($name),*];
    };
}

sheets! {
    MEASURE => "measure",
    FRAME => "frame",
    HEAD => "head",
    HOME => "home",
    TREE => "tree",
    PILL => "pill",
    CHAT => "chat",
    LOG => "log",
    MENU => "menu",
    DIALOG => "dialog",
    SETTINGS => "settings",
    NOTICE => "notice",
    MOTION => "motion",
}

/// The application sheet.
pub fn sheet() -> String {
    ALL.join("\n")
}
