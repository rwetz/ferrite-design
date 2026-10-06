//! Window chrome: square, self-drawn on Windows and Linux, native traffic
//! lights on macOS. The same platform split as Nexis, a different look.
//!
//! [`TitleBar`] ports gpui-component's platform handling (that part is
//! hard-won; see docs/PITFALLS.md) but draws text-mode controls — `_` `□` `x` in the
//! display face — and a dithered drag strip instead of a gradient.

use gpui::{
    AnyElement, App, Bounds, Decorations, InteractiveElement, IntoElement, MouseButton,
    ParentElement, Pixels, RenderOnce, SharedString, Size, StatefulInteractiveElement, Styled,
    TitlebarOptions, Window, WindowBackgroundAppearance, WindowBounds, WindowControlArea,
    WindowDecorations, WindowOptions, div, point, prelude::FluentBuilder as _, px, size,
};


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
            // Double-click to maximize (Windows gets this from the OS via the
            // caption hit-test).
            .when(IS_LINUX || IS_MAC, |this| {
                this.on_click(|ev, window, _| {
                    if ev.click_count() == 2 {
                        if IS_MAC { window.titlebar_double_click() } else { window.zoom_window() }
                    }
                })
            })
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

// ── Window frame ──────────────────────────────────────────────────────────

/// How far in from each window edge a press starts a resize.
const RESIZE_ZONE: Pixels = px(6.);

/// Wraps a window's whole content. Where the app draws its own decorations
/// (Linux with client-side decorations), it adds what the window manager
/// would have: a square 1px `line_strong` frame and resize edges with the
/// right cursors, both dropped on sides the window is tiled against.
/// Everywhere else (Windows, macOS, Linux with server decorations) it's a
/// plain full-size container.
///
/// ```ignore
/// window_frame().child(title_bar("App")).child(body)
/// ```
#[derive(IntoElement)]
pub struct WindowFrame {
    children: Vec<AnyElement>,
}

pub fn window_frame() -> WindowFrame {
    WindowFrame { children: Vec::new() }
}

impl ParentElement for WindowFrame {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

/// Which edge (if any) a window-relative point is on. Pure.
pub fn resize_edge(pos: gpui::Point<Pixels>, zone: Pixels, size: Size<Pixels>, tiling: gpui::Tiling) -> Option<gpui::ResizeEdge> {
    use gpui::ResizeEdge as E;
    let top = !tiling.top && pos.y < zone;
    let bottom = !tiling.bottom && pos.y > size.height - zone;
    let left = !tiling.left && pos.x < zone;
    let right = !tiling.right && pos.x > size.width - zone;
    Some(match (top, bottom, left, right) {
        (true, _, true, _) => E::TopLeft,
        (true, _, _, true) => E::TopRight,
        (_, true, true, _) => E::BottomLeft,
        (_, true, _, true) => E::BottomRight,
        (true, ..) => E::Top,
        (_, true, ..) => E::Bottom,
        (_, _, true, _) => E::Left,
        (_, _, _, true) => E::Right,
        _ => return None,
    })
}

impl RenderOnce for WindowFrame {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = palette(cx);
        let root = div().size_full().flex().flex_col();
        let Decorations::Client { tiling } = window.window_decorations() else {
            return root.children(self.children);
        };
        // Opaque, square, no shadow: nothing to inset.
        window.set_client_inset(px(0.));
        let line = hsla(p.line_strong);
        root.relative()
            .when(!tiling.top, |el| el.border_t_1())
            .when(!tiling.bottom, |el| el.border_b_1())
            .when(!tiling.left, |el| el.border_l_1())
            .when(!tiling.right, |el| el.border_r_1())
            .border_color(line)
            .children(self.children)
            .child(
                gpui::canvas(
                    |_, window, _| {
                        let size = window.window_bounds().get_bounds().size;
                        window.insert_hitbox(Bounds::new(point(px(0.), px(0.)), size), gpui::HitboxBehavior::Normal)
                    },
                    move |_, hitbox, window, _| {
                        let size = window.window_bounds().get_bounds().size;
                        if let Some(edge) = resize_edge(window.mouse_position(), RESIZE_ZONE, size, tiling) {
                            use gpui::{CursorStyle as C, ResizeEdge as E};
                            let cursor = match edge {
                                E::Top | E::Bottom => C::ResizeUpDown,
                                E::Left | E::Right => C::ResizeLeftRight,
                                E::TopLeft | E::BottomRight => C::ResizeUpLeftDownRight,
                                E::TopRight | E::BottomLeft => C::ResizeUpRightDownLeft,
                            };
                            window.set_cursor_style(cursor, &hitbox);
                        }
                        // Capture phase: an edge press resizes even over content.
                        window.on_mouse_event(move |ev: &gpui::MouseDownEvent, phase, window, cx| {
                            if !phase.capture() || ev.button != MouseButton::Left {
                                return;
                            }
                            let size = window.window_bounds().get_bounds().size;
                            if let Some(edge) = resize_edge(ev.position, RESIZE_ZONE, size, tiling) {
                                cx.stop_propagation();
                                window.start_window_resize(edge);
                            }
                        });
                        // Cursor follows the pointer between edges.
                        window.on_mouse_event(|_: &gpui::MouseMoveEvent, phase, window, _| {
                            if phase.bubble() {
                                window.refresh();
                            }
                        });
                    },
                )
                .absolute()
                .inset_0(),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{ResizeEdge as E, Tiling};

    #[test]
    fn edges_and_corners() {
        let size = gpui::size(px(800.), px(600.));
        let free = Tiling::default();
        let at = |x: f32, y: f32| resize_edge(point(px(x), px(y)), RESIZE_ZONE, size, free);
        assert_eq!(at(2., 2.), Some(E::TopLeft));
        assert_eq!(at(798., 598.), Some(E::BottomRight));
        assert_eq!(at(400., 2.), Some(E::Top));
        assert_eq!(at(2., 300.), Some(E::Left));
        assert_eq!(at(400., 300.), None);
    }

    #[test]
    fn tiled_sides_dont_resize() {
        let size = gpui::size(px(800.), px(600.));
        let tiled = Tiling { left: true, ..Tiling::default() };
        assert_eq!(resize_edge(point(px(2.), px(300.)), RESIZE_ZONE, size, tiled), None);
        assert_eq!(resize_edge(point(px(2.), px(2.)), RESIZE_ZONE, size, tiled), Some(E::Top));
    }
}
