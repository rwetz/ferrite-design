//! Window chrome: square, self-drawn on Windows and Linux, native traffic
//! lights on macOS. The same platform split as Nexis, a different look.
//!
//! [`TitleBar`] is Ferrite's own component, not gpui-component's. It follows
//! the library's platform handling closely (that part is hard-won; see
//! docs/PITFALLS.md) but draws text-mode controls — `_` `□` `x` in the
//! display face — and a dithered drag strip instead of a gradient.

use gpui::{
    AnyElement, App, Bounds, Decorations, InteractiveElement, IntoElement, MouseButton,
    ParentElement, Pixels, RenderOnce, SharedString, Size, StatefulInteractiveElement, Styled,
    TitlebarOptions, Window, WindowBackgroundAppearance, WindowBounds, WindowControlArea,
    WindowDecorations, WindowOptions, div, point, prelude::FluentBuilder as _, px, size,
};

use gpui_component::InteractiveElementExt as _;

use crate::dither::{self, dither};
use crate::fonts::{FerriteText, Scale};
use crate::theme::palette;
use crate::tokens::hsla;

pub const IS_MAC: bool = cfg!(target_os = "macos");
pub const IS_LINUX: bool = cfg!(target_os = "linux");
pub const IS_WINDOWS: bool = cfg!(target_os = "windows");

/// Two display-font rows.
pub const TITLE_BAR_HEIGHT: Pixels = px(32.);
/// Room for the macOS traffic lights.
const MAC_TRAFFIC_LIGHT_PAD: Pixels = px(80.);

/// The `WindowOptions` every Ferrite window starts from.
///
/// - Transparent titlebar, app-owned drag (so macOS doesn't delay clicks
///   disambiguating double-clicks; see PITFALLS).
/// - Client decorations requested on Linux (the WM may still refuse).
/// - Opaque background: Ferrite has no glass.
pub fn window_options(title: impl Into<SharedString>, initial: Size<Pixels>, cx: &App) -> WindowOptions {
    WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds::centered(None, initial, cx))),
        titlebar: Some(TitlebarOptions {
            title: Some(title.into()),
            appears_transparent: true,
            traffic_light_position: Some(point(px(10.), px(9.))),
        }),
        app_owns_titlebar_drag: true,
        window_min_size: Some(size(px(480.), px(320.))),
        window_decorations: Some(WindowDecorations::Client),
        window_background: WindowBackgroundAppearance::Opaque,
        ..Default::default()
    }
}

/// Opt out of Windows 11's DWM-forced rounded corners. Call once from the
/// `open_window` build closure. No-op elsewhere (macOS keeps its native
/// corner; Linux corners are whatever the compositor draws).
pub fn square_corners(window: &Window) {
    #[cfg(target_os = "windows")]
    {
        use raw_window_handle::{HasWindowHandle, RawWindowHandle};
        use windows::Win32::Foundation::HWND;
        use windows::Win32::Graphics::Dwm::{
            DWM_WINDOW_CORNER_PREFERENCE, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_DONOTROUND,
            DwmSetWindowAttribute,
        };
        // gpui's inherent `window_handle()` returns its own handle type; the
        // raw OS handle is behind the raw-window-handle trait.
        let Ok(handle) = HasWindowHandle::window_handle(window) else { return };
        if let RawWindowHandle::Win32(h) = handle.as_raw() {
            let pref: DWM_WINDOW_CORNER_PREFERENCE = DWMWCP_DONOTROUND;
            // Fails harmlessly on Windows 10, which has no rounding to undo.
            let _ = unsafe {
                DwmSetWindowAttribute(
                    HWND(h.hwnd.get() as *mut _),
                    DWMWA_WINDOW_CORNER_PREFERENCE,
                    &pref as *const _ as *const _,
                    std::mem::size_of::<DWM_WINDOW_CORNER_PREFERENCE>() as u32,
                )
            };
        }
    }
    #[cfg(not(target_os = "windows"))]
    let _ = window;
}

/// The Ferrite title bar. Put it first in the root view's column.
#[derive(IntoElement)]
pub struct TitleBar {
    title: SharedString,
    children: Vec<AnyElement>,
}

pub fn title_bar(title: impl Into<SharedString>) -> TitleBar {
    TitleBar { title: title.into(), children: Vec::new() }
}

impl ParentElement for TitleBar {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

struct DragState {
    should_move: bool,
}

impl RenderOnce for TitleBar {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = palette(cx);
        let state = window.use_state(cx, |_, _| DragState { should_move: false });
        let client_decorated = matches!(window.window_decorations(), Decorations::Client { .. });

        div()
            .id("ferrite-title-bar")
            .flex()
            .flex_row()
            .items_center()
            .flex_shrink_0()
            .h(TITLE_BAR_HEIGHT)
            .bg(hsla(p.surface))
            .border_b_1()
            .border_color(hsla(p.line))
            .when(IS_MAC, |this| this.pl(MAC_TRAFFIC_LIGHT_PAD))
            .when(IS_LINUX, |this| this.on_double_click(|_, window, _| window.zoom_window()))
            .when(IS_MAC, |this| this.on_double_click(|_, window, _| window.titlebar_double_click()))
            // Drag by starting a window move on the first mouse-move after a
            // press — not on press, or clicks on children would drag.
            .on_mouse_down_out(window.listener_for(&state, |s, _, _, _| s.should_move = false))
            .on_mouse_down(MouseButton::Left, window.listener_for(&state, |s, _, _, _| s.should_move = true))
            .on_mouse_up(MouseButton::Left, window.listener_for(&state, |s, _, _, _| s.should_move = false))
            .on_mouse_move(window.listener_for(&state, |s, _, window, _| {
                if s.should_move {
                    s.should_move = false;
                    window.start_window_move();
                }
            }))
            .child(
                div()
                    .id("ferrite-title-drag")
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_2()
                    .flex_1()
                    .h_full()
                    .px_3()
                    .window_control_area(WindowControlArea::Drag)
                    .when(IS_LINUX && client_decorated, |this| {
                        this.on_mouse_down(MouseButton::Right, |ev, window, _| window.show_window_menu(ev.position))
                    })
                    .child(
                        div()
                            .display(Scale::X1, window)
                            .text_color(hsla(p.accent))
                            .child("▓▒░"),
                    )
                    .child(
                        div()
                            .display(Scale::X1, window)
                            .text_color(hsla(p.fg))
                            .child(self.title.to_uppercase()),
                    )
                    .children(self.children)
                    // The rest of the bar is a quiet dither strip: the grip.
                    .child(
                        dither(dither::flat(dither::level::LIGHT))
                            .ink(hsla(p.line))
                            .cell(1)
                            .flex_1()
                            .h(px(8.)),
                    ),
            )
            .child(WindowControls { client_decorated })
    }
}

#[derive(IntoElement)]
struct WindowControls {
    client_decorated: bool,
}

#[derive(Clone, Copy)]
enum Control {
    Minimize,
    Maximize,
    Restore,
    Close,
}

impl RenderOnce for WindowControls {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        // macOS: native traffic lights. Linux under server-side decorations:
        // the WM already drew min/max/close — drawing ours too doubles them.
        if IS_MAC || (IS_LINUX && !self.client_decorated) {
            return div().id("ferrite-window-controls");
        }
        let supported = window.window_controls();
        let maximized = window.is_maximized();

        div()
            .id("ferrite-window-controls")
            .flex()
            .flex_row()
            .h_full()
            .flex_shrink_0()
            .when(supported.minimize, |this| this.child(control(Control::Minimize, window, cx)))
            .when(supported.maximize, |this| {
                this.child(control(if maximized { Control::Restore } else { Control::Maximize }, window, cx))
            })
            .child(control(Control::Close, window, cx))
    }
}

fn control(kind: Control, window: &mut Window, cx: &mut App) -> impl IntoElement {
    let p = palette(cx);
    let (id, glyph, area) = match kind {
        Control::Minimize => ("ferrite-min", "_", WindowControlArea::Min),
        Control::Maximize => ("ferrite-max", "□", WindowControlArea::Max),
        Control::Restore => ("ferrite-restore", "▫", WindowControlArea::Max),
        Control::Close => ("ferrite-close", "x", WindowControlArea::Close),
    };
    let is_close = matches!(kind, Control::Close);
    let (hover_bg, hover_fg) = if is_close { (p.danger, p.danger_fg) } else { (p.raised, p.fg) };

    div()
        .id(id)
        .flex()
        .items_center()
        .justify_center()
        .w(px(40.))
        .h_full()
        .display(Scale::X1, window)
        .text_color(hsla(p.fg_dim))
        .hover(move |s| s.bg(hsla(hover_bg)).text_color(hsla(hover_fg)))
        // Windows: the OS hit-tests these areas itself, which is what makes
        // Snap Layouts appear on hovering maximize. Do not add click handlers.
        .when(IS_WINDOWS, |this| this.window_control_area(area))
        .when(IS_LINUX, |this| {
            this.on_mouse_down(MouseButton::Left, |_, window, cx| {
                window.prevent_default();
                cx.stop_propagation();
            })
            .on_click(move |_, window, cx| {
                cx.stop_propagation();
                match kind {
                    Control::Minimize => window.minimize_window(),
                    Control::Maximize | Control::Restore => window.zoom_window(),
                    Control::Close => window.remove_window(),
                }
            })
        })
        .child(glyph)
}
