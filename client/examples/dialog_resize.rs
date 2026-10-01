//! Repro: a centred dialog whose content changes height is painted off-centre, while hit-testing
//! still uses the centred position.
//!
//! Click "Grow" and "Shrink": the dialog box moves toward the bottom right, and the buttons only
//! respond where they would be if the dialog were still centred.

use zgui::prelude::*;
use zgui::reactive::LocalStorage;
use zgui_ui::prelude::*;
use zgui_ui_tokens::prelude::*;

#[component]
fn Repro() -> impl IntoView {
    let open: RwSignal<bool, LocalStorage> = RwSignal::new_local(true);
    let clicks = RwSignal::new_local(0);
    view! {
        Dialog(open = open) {
            DialogContent(style:height = "520px") {
                DialogHeader { DialogTitle {"Resize me"} }
                Tabs(default_value = "tall", label = "Pages") {
                    TabsList {
                        TabsTrigger(value = "tall") {"Tall"}
                        TabsTrigger(value = "short") {"Short"}
                    }
                    TabsContent(value = "tall") {
                        box(style:height = "360px", style:background-color = "#ddd") {}
                    }
                    TabsContent(value = "short") {
                        Button(on:click = move |_| clicks.update(|n| *n += 1)) {{move || format!("clicked {}", clicks.get())}}
                    }
                }
            }
        }
    }
}

fn main() -> Result<(), zgui::Error> {
    app()
        .with_title("dialog-resize")
        .with_size(900.0, 700.0)
        .run(|| view! { ThemeProvider(scheme = ColorScheme::Light) { Repro() } })
}
