//! Window chrome: square, self-drawn on Windows and Linux, native traffic
//! lights on macOS. The same platform split as Nexis, a different look.
//!
//! [`TitleBar`] ports gpui-component's platform handling (that part is
//! hard-won; see docs/PITFALLS.md) but draws text-mode controls — `_` `□` `x` in the
//! display face — and a dithered drag strip instead of a gradient.

use std::collections::HashSet;

use gpui::{
    AnyElement, App, Bounds, Decorations, InteractiveElement, IntoElement, MouseButton,
    ParentElement, Pixels, RenderOnce, SharedString, Size, StatefulInteractiveElement, Styled,
    TitlebarOptions, Window, WindowBackgroundAppearance, WindowBounds, WindowControlArea,
    WindowDecorations, WindowId, WindowOptions, div, point, prelude::FluentBuilder as _, px, size,
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
/// - The initial size is shrunk to fit the primary screen's usable area
///   (minus a margin), so a window designed at 1040×860 still opens whole
///   on a 1366×768 laptop or a 1080p screen at 150%. Lay the content out to
///   cope with that (see [`crate::components::responsive`]).
pub fn window_options(title: impl Into<SharedString>, initial: Size<Pixels>, cx: &App) -> WindowOptions {
    let initial = match cx.primary_display() {
        Some(d) => fit_size(initial, d.visible_bounds().size),
        None => initial,
    };
    WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds::centered(None, initial, cx))),
        titlebar: Some(TitlebarOptions {
            title: Some(title.into()),
            appears_transparent: true,
            traffic_light_position: Some(point(px(10.), px(9.))),
        }),
        app_owns_titlebar_drag: true,
        window_min_size: Some(MIN_WINDOW),
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
        // The mark types on and the title decrypts when the window opens.
        let boot = crate::animate::play("title-boot", 0u8, crate::motion::SLOW, window, cx);
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
            // Two drag areas with the caller's children between them. On
            // Windows a drag area is caption to the OS hit-test, which eats
            // clicks, so buttons must not sit inside one.
            .child(
                div()
                    .id("ferrite-title-drag")
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_2()
                    .h_full()
                    .pl_3()
                    .pr_2()
                    .window_control_area(WindowControlArea::Drag)
                    .when(IS_LINUX && client_decorated, |this| {
                        this.on_mouse_down(MouseButton::Right, |ev, window, _| window.show_window_menu(ev.position))
                    })
                    // The mark types on cell by cell as the window boots.
                    .child(crate::components::textmode::mark().shown(if boot.done { 3 } else { (boot.t * 3.).round() as usize + 1 }))
                    .child(
                        div()
                            .display(Scale::X1, window)
                            .text_color(hsla(p.fg))
                            .child(crate::animate::scramble(&self.title.to_uppercase(), boot)),
                    ),
            )
            .child(div().flex().flex_row().items_center().gap_2().children(self.children))
            // The rest of the bar is a quiet dither strip: the grip.
            .child(
                div()
                    .id("ferrite-title-grip")
                    .flex()
                    .items_center()
                    .flex_1()
                    .h_full()
                    .px_2()
                    .window_control_area(WindowControlArea::Drag)
                    .when(IS_LINUX && client_decorated, |this| {
                        this.on_mouse_down(MouseButton::Right, |ev, window, _| window.show_window_menu(ev.position))
                    })
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
                    Control::Close => {
                        if close_request(window, cx) {
                            window.remove_window();
                        }
                    }
                }
            })
        })
        .child(glyph)
}

// ── Power off ─────────────────────────────────────────────────────────────

/// Which windows switch off like a CRT when closed, and which are doing it
/// now. `components::power_on_in` reads it to play the switch-off.
#[derive(Default)]
struct PowerOff {
    enabled: HashSet<WindowId>,
    closing: HashSet<WindowId>,
}

impl gpui::Global for PowerOff {}

/// Close this window CRT style: the picture collapses to an amber line and
/// the line shrinks into the centre, then the window closes (320ms). The
/// one exit Ferrite animates — it is the machine switching off. Call from
/// the `open_window` closure, and wrap the root in
/// `components::power_on_in`, which draws it.
///
/// Covers every close request the OS routes through the window (the
/// traffic light, Alt+F4, the Windows title-bar ✕) and Ferrite's own Linux
/// close button. Quitting the whole app (⌘Q) skips it. Reduced motion
/// closes at once.
pub fn power_off_on_close(window: &mut Window, cx: &mut App) {
    let id = window.window_handle().window_id();
    cx.default_global::<PowerOff>().enabled.insert(id);
    window.on_window_should_close(cx, close_request);
}

/// Whether this window is mid power-off.
pub fn powering_off(window: &Window, cx: &App) -> bool {
    cx.try_global::<PowerOff>().is_some_and(|p| p.closing.contains(&window.window_handle().window_id()))
}

/// A request to close `window`: `true` to close now, `false` to wait while
/// it switches off (it then closes itself).
fn close_request(window: &mut Window, cx: &mut App) -> bool {
    let id = window.window_handle().window_id();
    save_remembered(window, cx);
    if crate::motion::reduced(cx) || !cx.try_global::<PowerOff>().is_some_and(|p| p.enabled.contains(&id)) {
        return true;
    }
    if !cx.default_global::<PowerOff>().closing.insert(id) {
        return false; // Already switching off; a second click waits too.
    }
    window.refresh();
    window
        .spawn(cx, async move |cx| {
            cx.background_executor().timer(crate::motion::SLOW + crate::motion::FRAME).await;
            let _ = cx.update(|window, cx| {
                let state = cx.default_global::<PowerOff>();
                state.closing.remove(&id);
                state.enabled.remove(&id);
                window.remove_window();
            });
        })
        .detach();
    false
}

// ── Remembered windows ────────────────────────────────────────────────────

/// The smallest window Ferrite opens (also the OS minimum it sets).
pub const MIN_WINDOW: Size<Pixels> = Size { width: px(480.), height: px(320.) };
/// Room left around a window opened at a size the screen can't fit.
const SCREEN_MARGIN: f32 = 48.;

/// `want`, shrunk (never grown) to fit inside `screen` with a margin, and
/// never below [`MIN_WINDOW`].
pub fn fit_size(want: Size<Pixels>, screen: Size<Pixels>) -> Size<Pixels> {
    let fit = |want: Pixels, room: Pixels, min: Pixels| want.min(room - px(SCREEN_MARGIN)).max(min);
    size(fit(want.width, screen.width, MIN_WINDOW.width), fit(want.height, screen.height, MIN_WINDOW.height))
}

/// How a window was left: windowed, maximized or fullscreen, and its
/// (restore) bounds. Stored as one line of text.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Remembered {
    state: u8,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

impl Remembered {
    fn of(bounds: WindowBounds) -> Self {
        let (state, b) = match bounds {
            WindowBounds::Windowed(b) => (0, b),
            WindowBounds::Maximized(b) => (1, b),
            WindowBounds::Fullscreen(b) => (2, b),
        };
        Remembered { state, x: b.origin.x.into(), y: b.origin.y.into(), w: b.size.width.into(), h: b.size.height.into() }
    }

    fn bounds(&self) -> Bounds<Pixels> {
        Bounds::new(point(px(self.x), px(self.y)), size(px(self.w), px(self.h)))
    }

    fn window_bounds(&self, bounds: Bounds<Pixels>) -> WindowBounds {
        match self.state {
            1 => WindowBounds::Maximized(bounds),
            2 => WindowBounds::Fullscreen(bounds),
            _ => WindowBounds::Windowed(bounds),
        }
    }

    fn serialize(&self) -> String {
        let state = ["windowed", "maximized", "fullscreen"][self.state as usize];
        format!("{state} {:.0} {:.0} {:.0} {:.0}\n", self.x, self.y, self.w, self.h)
    }

    fn parse(text: &str) -> Option<Self> {
        let mut it = text.split_whitespace();
        let state = match it.next()? {
            "windowed" => 0,
            "maximized" => 1,
            "fullscreen" => 2,
            _ => return None,
        };
        let mut n = || it.next()?.parse::<f32>().ok().filter(|v| v.is_finite());
        let (x, y, w, h) = (n()?, n()?, n()?, n()?);
        (w > 0. && h > 0.).then_some(Remembered { state, x, y, w, h })
    }

    /// Where to reopen, given the screens there are now: the saved bounds
    /// if their middle is on one of them (shrunk to fit it, nudged fully
    /// onto it), or `None` when the screen they were on is gone.
    fn place(&self, screens: &[Bounds<Pixels>]) -> Option<Bounds<Pixels>> {
        let saved = self.bounds();
        let screen = screens.iter().find(|s| s.contains(&saved.center()))?;
        let sz = fit_size(saved.size, screen.size + size(px(SCREEN_MARGIN), px(SCREEN_MARGIN)));
        let x = saved.origin.x.max(screen.origin.x).min(screen.origin.x + screen.size.width - sz.width);
        let y = saved.origin.y.max(screen.origin.y).min(screen.origin.y + screen.size.height - sz.height);
        Some(Bounds::new(point(x, y), sz))
    }
}

/// The config directory every Ferrite app writes to:
/// `%APPDATA%\ferrite`, `~/Library/Application Support/ferrite`, or
/// `$XDG_CONFIG_HOME/ferrite` (`~/.config/ferrite`).
pub fn config_dir() -> Option<std::path::PathBuf> {
    use std::path::PathBuf;
    let base = if IS_WINDOWS {
        std::env::var_os("APPDATA").map(PathBuf::from)
    } else if IS_MAC {
        std::env::var_os("HOME").map(|h| PathBuf::from(h).join("Library/Application Support"))
    } else {
        std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from).or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
    };
    base.map(|b| b.join("ferrite"))
}

fn remembered_path(key: &str) -> Option<std::path::PathBuf> {
    config_dir().map(|d| d.join(format!("{key}.window")))
}

/// [`window_options`] that reopen where the window was last left: same
/// size, same place, still maximized or fullscreen. `key` names the file
/// (`<config>/ferrite/<key>.window`); first launch, or when that screen is
/// gone, falls back to `default` centered. Pair with [`remember_window`].
///
/// ```ignore
/// let options = chrome::remembered_window_options("almanac", "Almanac", size(px(1120.), px(760.)), cx);
/// cx.open_window(options, |window, cx| {
///     chrome::remember_window("almanac", window, cx);
///     …
/// })
/// ```
pub fn remembered_window_options(key: &str, title: impl Into<SharedString>, default: Size<Pixels>, cx: &App) -> WindowOptions {
    let mut options = window_options(title, default, cx);
    let saved = remembered_path(key).and_then(|p| std::fs::read_to_string(p).ok()).and_then(|t| Remembered::parse(&t));
    if let Some(saved) = saved {
        let screens: Vec<_> = cx.displays().iter().map(|d| d.visible_bounds()).collect();
        if let Some(bounds) = saved.place(&screens) {
            options.window_bounds = Some(saved.window_bounds(bounds));
        }
    }
    options
}

#[derive(Default)]
struct Remembering {
    keys: std::collections::HashMap<WindowId, &'static str>,
}

impl gpui::Global for Remembering {}

/// Save this window's size, place and state as it changes, for
/// [`remembered_window_options`] to reopen it with. Call once from the
/// `open_window` closure. Checked twice a second, written only when it
/// changed; closing (with [`power_off_on_close`]) saves at once.
pub fn remember_window(key: &'static str, window: &mut Window, cx: &mut App) {
    let id = window.window_handle().window_id();
    cx.default_global::<Remembering>().keys.insert(id, key);
    let mut last = window.window_bounds();
    // Polled rather than observed: gpui only reports resizes to a view's
    // own `Context`, and this has to work from the open_window closure.
    window
        .spawn(cx, async move |cx| {
            loop {
                cx.background_executor().timer(std::time::Duration::from_millis(500)).await;
                let Ok(now) = cx.update(|window, _| window.window_bounds()) else { break };
                if now != last {
                    last = now;
                    if let Some(path) = remembered_path(key) {
                        write_remembered(&path, Remembered::of(now));
                    }
                }
            }
        })
        .detach();
}

fn save_remembered(window: &Window, cx: &App) {
    let id = window.window_handle().window_id();
    let Some(key) = cx.try_global::<Remembering>().and_then(|r| r.keys.get(&id).copied()) else { return };
    if let Some(path) = remembered_path(key) {
        write_remembered(&path, Remembered::of(window.window_bounds()));
    }
}

fn write_remembered(path: &std::path::Path, r: Remembered) {
    // Best effort: losing a window size isn't worth an error.
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::write(path, r.serialize());
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

/// The theme glitch (DESIGN_LANGUAGE §6.1): when the scheme or tone changes,
/// the new palette paints at once (no flash of a half-themed window), and
/// strips of the old one tear away over it for four beats while the window
/// jolts. Nothing here takes input.
fn theme_glitch(children: Vec<AnyElement>, window: &mut Window, cx: &mut App) -> AnyElement {
    let (epoch, from) = crate::theme::last_shift(cx);
    let g = crate::animate::play_on_change("ferrite-theme-glitch", epoch, crate::motion::BASE, window, cx);
    let content = div().size_full().flex().flex_col().children(children);
    let (Some(from), false) = (from, g.done) else {
        return content.into_any_element();
    };
    let bands = crate::animate::glitch_bands(epoch, g);
    let strip = |b: crate::animate::GlitchBand| {
        let ground = hsla(from.bg);
        div()
            .absolute()
            .left_0()
            .right_0()
            .top(gpui::relative(b.top))
            .h(gpui::relative(b.height))
            .map(|el| if b.level >= 1. { el.bg(ground) } else { el.child(dither(dither::flat(b.level)).ink(ground).size_full()) })
            .when(b.lit, |el| el.child(div().absolute().top_0().left_0().right_0().h(px(2.)).bg(hsla(from.accent))))
    };
    div()
        .size_full()
        .relative()
        .child(crate::animate::nudge(content, point(crate::animate::glitch_jolt(g), px(0.))))
        .child(div().absolute().inset_0().children(bands.into_iter().map(strip)))
        .into_any_element()
}

impl RenderOnce for WindowFrame {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = palette(cx);
        let root = div().size_full().flex().flex_col();
        let body = theme_glitch(self.children, window, cx);
        let Decorations::Client { tiling } = window.window_decorations() else {
            return root.child(body);
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
            .child(body)
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
    fn a_big_default_fits_a_small_screen() {
        let laptop = size(px(1366.), px(728.));
        assert_eq!(fit_size(size(px(1040.), px(860.)), laptop), size(px(1040.), px(680.)));
        // Never grown, never below the minimum.
        assert_eq!(fit_size(size(px(600.), px(400.)), laptop), size(px(600.), px(400.)));
        assert_eq!(fit_size(size(px(900.), px(900.)), size(px(400.), px(300.))), MIN_WINDOW);
    }

    #[test]
    fn remembered_bounds_round_trip() {
        let r = Remembered { state: 1, x: -1200., y: 40., w: 1100., h: 700. };
        assert_eq!(Remembered::parse(&r.serialize()), Some(r));
        assert_eq!(Remembered::parse("maximized 1 2 3"), None);
        assert_eq!(Remembered::parse("windowed 0 0 0 10"), None);
        assert_eq!(Remembered::parse("rolled 0 0 10 10"), None);
        assert_eq!(Remembered::parse("windowed 0 0 NaN 10"), None);
    }

    #[test]
    fn remembered_bounds_land_on_a_screen_that_exists() {
        let main = Bounds::new(point(px(0.), px(0.)), size(px(1920.), px(1040.)));
        let left = Bounds::new(point(px(-1280.), px(0.)), size(px(1280.), px(984.)));
        let on_left = Remembered { state: 0, x: -1200., y: 100., w: 900., h: 700. };
        assert_eq!(on_left.place(&[main, left]), Some(on_left.bounds()));
        // Its screen unplugged: fall back to the default.
        assert_eq!(on_left.place(&[main]), None);
        // Hanging off the edge, or too big: pulled on and shrunk.
        let off = Remembered { state: 0, x: 1500., y: 100., w: 800., h: 1800. };
        let placed = off.place(&[main]).unwrap();
        assert!(placed.origin.x + placed.size.width <= px(1920.) && placed.origin.y >= px(0.));
        assert!(placed.size.height <= px(1040.));
    }

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
