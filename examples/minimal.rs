//! The smallest correct Ferrite app — the code in docs/SCAFFOLDING.md, kept
//! compiling so the guide can't rot.
//!
//!     cargo run --example minimal

// Release builds are GUI-subsystem on Windows: no console window behind the app.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use ferrite_design::{Appearance, chrome, components::status_bar, palette, tokens::hsla};
use gpui::{App, AppContext as _, Context, IntoElement, ParentElement, Render, Styled,
           Subscription, Window, div, px, size};

struct MyApp {
    _appearance: Subscription,
}

impl Render for MyApp {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette(cx);
        div()
            .flex().flex_col().size_full()
            .bg(hsla(p.bg)).text_color(hsla(p.fg))
            .child(chrome::title_bar("My App"))
            .child(div().flex_1().min_h_0() /* your content */)
            .child(status_bar().left("READY"))
    }
}

fn main() {
    gpui_platform::application().run(|cx: &mut App| {
        // 1. Theme + fonts BEFORE any window (no first-frame flash).
        ferrite_design::init(Appearance::Dark, cx);

        // 2. Every window starts from chrome::window_options.
        let options = chrome::window_options("My App", size(px(1200.), px(800.)), cx);
        cx.open_window(options, |window, cx| {
            // 3. Square corners on Windows 11.
            chrome::square_corners(window);
            // 4. Your view is the window's root — no wrapper needed.
            cx.new(|_| MyApp { _appearance: ferrite_design::theme::follow_system(window) })
        })
        .unwrap();
        cx.activate(true);
    });
}
