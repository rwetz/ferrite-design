//! The Ferrite button — the first native replacement for a gpui-component
//! widget (docs/COMPONENTS.md #2).
//!
//! Its *behaviour* is ported from gpui-component's `Button`: a per-element
//! focus handle, keyboard activation (gpui fires `on_click` for Enter/Space
//! on a focused element), no focus-steal on mouse down, disabled and
//! loading states that swallow clicks, an accessibility role and label,
//! tooltips. Its *look* is Ferrite's:
//!
//! | Variant | Rest | Hover | Pressed |
//! |---|---|---|---|
//! | `primary` | amber fill, dark label | brighter amber | inverse: dark fill, amber label |
//! | `secondary` (default) | `raised` box, 1px `line_strong` | `line` fill | inverse: `fg` fill, `bg` label |
//! | `ghost` | text-mode `[ LABEL ]`, no box | brackets turn amber | inverse |
//! | `danger` | 1px danger frame, danger label | danger fill | inverse |
//!
//! Disabled is a dashed frame with faint ink. Loading swaps the glyph for a
//! spinner (give loading-capable buttons a glyph so their width doesn't
//! change mid-click, and keep the label stable). `selected(true)` holds the pressed (inverse) look — a toggle
//! button. Focus (keyboard only) is a 1px accent frame.
//!
//! The builder mirrors gpui-component's, so migrating is mostly an import
//! change: `Button::new("run").label("Run").primary().on_click(..)`.

use std::rc::Rc;

use gpui::{
    App, ClickEvent, ElementId, FocusHandle, InteractiveElement, IntoElement, MouseButton,
    ParentElement, RenderOnce, Role, SharedString, StatefulInteractiveElement, Styled, Window,
    div, prelude::FluentBuilder as _, px,
};

use super::ticker::spinner;
use crate::icon::{Icon, icon};
use super::tooltip::{kbd, tooltip};
use crate::fonts::{FerriteText, Scale};
use crate::theme::palette;
use crate::tokens::hsla;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Variant {
    Primary,
    #[default]
    Secondary,
    Ghost,
    Danger,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Size {
    /// 24px tall. Toolbars, dense rows.
    Small,
    /// 32px tall — two display rows.
    #[default]
    Medium,
}

type ClickHandler = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>;

#[derive(IntoElement)]
pub struct Button {
    id: ElementId,
    label: Option<SharedString>,
    icon: Option<Icon>,
    glyph: Option<SharedString>,
    shortcut: Option<SharedString>,
    tooltip: Option<SharedString>,
    variant: Variant,
    size: Size,
    disabled: bool,
    loading: bool,
    selected: bool,
    full_width: bool,
    tab_index: isize,
    on_click: Option<ClickHandler>,
}

impl Button {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            label: None,
            icon: None,
            glyph: None,
            shortcut: None,
            tooltip: None,
            variant: Variant::default(),
            size: Size::default(),
            disabled: false,
            loading: false,
            selected: false,
            full_width: false,
            tab_index: 0,
            on_click: None,
        }
    }

    /// The label. Rendered UPPERCASE in the display face.
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// A leading pixel icon. Preferred over [`Button::glyph`]: icons are drawn
    /// on the display font's grid and never fall back to a system font. With
    /// no label, the button becomes square.
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// A leading text glyph. Only characters the display face has
    /// (`fonts::display_has`), e.g. `» × ■ ● ↑`. Anything else falls back to
    /// a system font and looks wrong; use [`Button::icon`].
    pub fn glyph(mut self, glyph: impl Into<SharedString>) -> Self {
        self.glyph = Some(glyph.into());
        self
    }

    /// A trailing keycap hint, e.g. `"Ctrl+R"`. Display only — it does not
    /// bind the key.
    pub fn shortcut(mut self, shortcut: impl Into<SharedString>) -> Self {
        self.shortcut = Some(shortcut.into());
        self
    }

    pub fn tooltip(mut self, text: impl Into<SharedString>) -> Self {
        self.tooltip = Some(text.into());
        self
    }

    pub fn primary(mut self) -> Self {
        self.variant = Variant::Primary;
        self
    }

    pub fn secondary(mut self) -> Self {
        self.variant = Variant::Secondary;
        self
    }

    pub fn ghost(mut self) -> Self {
        self.variant = Variant::Ghost;
        self
    }

    pub fn danger(mut self) -> Self {
        self.variant = Variant::Danger;
        self
    }

    pub fn small(mut self) -> Self {
        self.size = Size::Small;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Inert while loading, but keeps its colors (it's busy, not unavailable).
    pub fn loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Hold the pressed look: a toggle button.
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    pub fn full_width(mut self) -> Self {
        self.full_width = true;
        self
    }

    pub fn tab_index(mut self, index: isize) -> Self {
        self.tab_index = index;
        self
    }

    pub fn on_click(mut self, handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }
}

/// Resolved colors for one state.
#[derive(Clone, Copy)]
struct Look {
    bg: Option<u32>,
    fg: u32,
    border: Option<u32>,
}

impl RenderOnce for Button {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = palette(cx);
        let interactive = !self.disabled && !self.loading;

        let focus: FocusHandle = window
            .use_keyed_state(self.id.clone(), cx, |_, cx| cx.focus_handle())
            .read(cx)
            .clone();

        // Rest / hover / pressed per variant. Pressed is inverse video.
        let (rest, hover, pressed) = match self.variant {
            Variant::Primary => (
                Look { bg: Some(p.accent), fg: p.accent_fg, border: Some(p.accent) },
                Look { bg: Some(p.accent_hover), fg: p.accent_fg, border: Some(p.accent_hover) },
                Look { bg: Some(p.accent_fg), fg: p.accent, border: Some(p.accent) },
            ),
            Variant::Secondary => (
                Look { bg: Some(p.raised), fg: p.fg, border: Some(p.line_strong) },
                Look { bg: Some(p.line), fg: p.fg, border: Some(p.line_strong) },
                Look { bg: Some(p.fg), fg: p.bg, border: Some(p.fg) },
            ),
            Variant::Ghost => (
                Look { bg: None, fg: p.fg_dim, border: None },
                Look { bg: None, fg: p.fg, border: None },
                Look { bg: Some(p.fg), fg: p.bg, border: None },
            ),
            Variant::Danger => (
                Look { bg: None, fg: p.danger, border: Some(p.danger) },
                Look { bg: Some(p.danger), fg: p.danger_fg, border: Some(p.danger) },
                Look { bg: Some(p.danger_fg), fg: p.danger, border: Some(p.danger) },
            ),
        };
        let base = if self.selected { pressed } else { rest };
        let ghost = self.variant == Variant::Ghost;

        let (height, pad_x, icon_max) = match self.size {
            Size::Small => (px(24.), px(8.), px(22.)),
            Size::Medium => (px(32.), px(12.), px(30.)),
        };
        let label = self.label.as_ref().map(|l| l.to_uppercase());
        // Icon-only buttons are exactly square.
        let icon_only = label.is_none() && self.shortcut.is_none() && (self.icon.is_some() || self.glyph.is_some());
        let accessible_name = self.label.clone().or_else(|| self.tooltip.clone());

        // Ghost buttons wear their frame as text: `[ LABEL ]`. The brackets
        // light up amber on hover via a group, so they need to know the id.
        let group: SharedString = format!("ferrite-btn-{}", self.id).into();
        let bracket = |s: &'static str| {
            div()
                .text_color(hsla(p.fg_faint))
                .when(interactive, |el| el.group_hover(group.clone(), |s| s.text_color(hsla(p.accent))))
                .child(s)
        };

        let content = div()
            .flex()
            .flex_row()
            .items_center()
            .gap_2()
            .display(Scale::X1, window)
            .when(ghost && !self.selected && !icon_only, |el| el.child(bracket("[")))
            .map(|el| {
                if self.loading {
                    el.child(spinner(ElementId::NamedChild(self.id.clone().into(), "spin".into())).color(hsla(base.fg)))
                } else {
                    el.when_some(self.icon, |el, i| {
                        el.child(icon(i).fit(icon_max).color(hsla(if self.disabled { p.fg_faint } else { base.fg })))
                    })
                    .when_some(self.glyph.clone(), |el, g| el.child(g))
                }
            })
            .when_some(label, |el, l| el.child(l))
            .when(ghost && !self.selected && !icon_only, |el| el.child(bracket("]")))
            .when_some(self.shortcut.clone(), |el, k| el.child(kbd(&k)));

        let on_click = self.on_click.clone();

        // Click acknowledgement: the button floods with its text color and
        // dissolves back through the Bayer ramp in three frames.
        let acks = window.use_keyed_state(ElementId::NamedChild(std::sync::Arc::new(self.id.clone()), "acks".into()), cx, |_, _| 0u32);
        let ack = crate::animate::play_on_change(
            ElementId::NamedChild(std::sync::Arc::new(self.id.clone()), "ack".into()),
            *acks.read(cx),
            crate::motion::FAST,
            window,
            cx,
        );
        // Capped at ▓ (75%) so the label still reads through the flash.
        let flash = (!ack.done)
            .then(|| (crate::animate::dissolve_level(ack) * crate::dither::level::DARK * 16.).round() / 16.)
            .filter(|l| *l > 0.);

        div()
            .id(self.id.clone())
            .group(group.clone())
            .role(Role::Button)
            .when_some(accessible_name, |el, name| el.aria_label(name))
            .when(self.selected, |el| el.aria_toggled(gpui::Toggled::True))
            .track_focus(&focus)
            .when(interactive, |el| el.tab_stop(true).tab_index(self.tab_index))
            .when(!interactive, |el| el.tab_stop(false))
            .flex()
            .flex_shrink_0()
            .items_center()
            .justify_center()
            .h(height)
            .map(|el| if icon_only { el.w(height) } else { el.px(if ghost { px(4.) } else { pad_x }) })
            .when(self.full_width, |el| el.w_full())
            .border_1()
            .map(|el| {
                if self.disabled {
                    el.border_dashed()
                        .border_color(hsla(if ghost { p.bg } else { p.line_strong }))
                        .text_color(hsla(p.fg_faint))
                } else {
                    el.border_color(hsla(base.border.unwrap_or(p.bg)))
                        .when_some(base.bg, |el, bg| el.bg(hsla(bg)))
                        .text_color(hsla(base.fg))
                }
            })
            // Ghost borders are invisible at rest; keep the 1px so focus can
            // show without the layout shifting.
            .when(ghost && !self.disabled && base.border.is_none(), |el| el.border_color(gpui::transparent_black()))
            .when(interactive && !self.selected, |el| {
                el.hover(move |s| {
                    let s = s.text_color(hsla(hover.fg));
                    let s = match hover.bg { Some(bg) => s.bg(hsla(bg)), None => s };
                    match hover.border { Some(b) => s.border_color(hsla(b)), None => s }
                })
                .active(move |s| {
                    let s = s.text_color(hsla(pressed.fg));
                    let s = match pressed.bg { Some(bg) => s.bg(hsla(bg)), None => s };
                    match pressed.border { Some(b) => s.border_color(hsla(b)), None => s }
                })
            })
            .when(interactive, |el| el.focus_visible(|s| s.border_color(hsla(p.accent))))
            .when(!self.disabled, |el| {
                // Clicking must not steal keyboard focus from where the user was.
                el.on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
            })
            .when_some(on_click.filter(|_| interactive), |el, handler| {
                el.on_click(move |event, window, cx| {
                    acks.update(cx, |n, cx| {
                        *n += 1;
                        cx.notify();
                    });
                    handler(event, window, cx)
                })
            })
            .when_some(flash, |el, level| {
                el.relative().child(
                    div()
                        .absolute()
                        .inset_0()
                        .child(crate::dither::dither(crate::dither::flat(level)).ink(hsla(base.fg)).size_full()),
                )
            })
            .when_some(self.tooltip.clone(), |el, text| {
                let t = tooltip(text);
                let t = match &self.shortcut { Some(k) => t.kbd(k.clone()), None => t };
                el.tooltip(t.builder())
            })
            .child(content)
    }
}
