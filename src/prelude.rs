//! Everything a Ferrite view usually needs, in one import:
//!
//! ```ignore
//! use ferrite_design::prelude::*;
//! ```
//!
//! Brings in the components, the effects, the palette accessors and type
//! helpers, icons, dither, motion and the gpui traits component builders
//! rely on (`Styled`, `ParentElement`, `InteractiveElement`, `FluentBuilder`…).
//! It deliberately does *not* glob-import gpui itself: name gpui items
//! (`div`, `px`, `Context`, …) from gpui as usual.
//!
//! The app skeleton from AGENTS.md, compiled so it stays true:
//!
//! ```no_run
//! use ferrite_design::prelude::*;
//! use gpui::{App, AppContext as _, Context, Entity, IntoElement, Render, SharedString,
//!            Subscription, Window, div, px, size};
//!
//! struct MyApp { palette: Entity<CommandPalette>, toaster: Entity<Toaster>, _appearance: Subscription }
//!
//! impl MyApp {
//!     fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
//!         let palette = cx.new(|cx| {                  // in the constructor: it parks focus so shortcuts work
//!             let mut p = CommandPalette::new(window, cx);
//!             p.set_commands(vec![command("New file").group("File").icon(Icon::Plus).shortcut("Ctrl+N")
//!                 .on_run(|window, cx| { /* … */ })], cx);
//!             p
//!         });
//!         Self { palette, toaster: cx.new(|_| Toaster::new()), _appearance: theme::follow_system(window) }
//!     }
//! }
//!
//! impl Render for MyApp {
//!     fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
//!         let p = palette(cx);
//!         window_frame().child(power_on_in("power", div()
//!             .flex().flex_col().size_full().bg(hsla(p.bg)).text_color(hsla(p.fg)).body(text::BASE)
//!             .on_action(cx.listener(|this, _: &TogglePalette, window, cx| this.palette.update(cx, |p, cx| p.toggle(window, cx))))
//!             .child(self.palette.clone()).child(self.toaster.clone())
//!             .child(title_bar("My App"))
//!             .child(div().flex().flex_row().flex_1().min_h_0() /* sidebar + content */)
//!             .child(status_bar().left("READY").right_live(format!("{}FPS", motion::fps())))))
//!     }
//! }
//!
//! fn main() {
//!     gpui_platform::application().run(|cx: &mut App| {
//!         ferrite_design::init(Appearance::Dark, cx);
//!         theme::apply_env(cx);                 // FERRITE_SCHEME / _APPEARANCE / _FPS dev overrides
//!         // theme::set_scheme(schemes::by_key("harbor").unwrap(), cx);   // or pick a scheme in code
//!         cx.bind_keys([gpui::KeyBinding::new("ctrl-shift-p", TogglePalette, None)]);
//!         let options = chrome::window_options("My App", size(px(1200.), px(800.)), cx);
//!         cx.open_window(options, |window, cx| {
//!             chrome::square_corners(window);
//!             chrome::power_off_on_close(window, cx);
//!             cx.new(|cx| MyApp::new(window, cx))
//!         }).unwrap();
//!         cx.activate(true);
//!     });
//! }
//! ```

pub use crate::components::*;
pub use crate::components::calendar::Date;
pub use crate::components::tag::Tone;
pub use crate::animate::{self, Edge};
pub use crate::chrome::{self, title_bar, window_frame};
pub use crate::dither::{self, dither};
pub use crate::fonts::{FerriteText, Scale, display_size};
pub use crate::icon::{Icon, icon};
pub use crate::motion;
pub use crate::schemes::{self, SCHEMES, Scheme, SchemeKind};
pub use crate::theme::{self, Appearance, palette};
pub use crate::tokens::{hsla, hsla_a, space, text};

pub use gpui::prelude::FluentBuilder as _;
pub use gpui::{InteractiveElement as _, ParentElement as _, StatefulInteractiveElement as _, Styled as _};
