//! A selectable list row — the unit of every file list, process table and
//! settings page.
//!
//! `▌ name.rs                    12 KB` — a 2px amber bar marks the
//! selected row (with an `accent_dim` wash), hover lifts to `raised`.
//! Optional leading glyph, trailing meta in dim body type. Rows don't own
//! scrolling or virtualisation: put them in `virtual_list`
//! for long lists (docs/COMPONENTS.md keeps that wrapped).

use std::rc::Rc;

use gpui::{
    App, ClickEvent, ElementId, FocusHandle, InteractiveElement, IntoElement, MouseButton,
    ParentElement, RenderOnce, Role, SharedString, StatefulInteractiveElement, Styled, Window,
    div, prelude::FluentBuilder as _, px,
};

use crate::fonts::FerriteText;
use crate::icon::{Icon, icon};
use crate::theme::palette;
use crate::tokens::{hsla, text};

type ClickHandler = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>;

#[derive(IntoElement)]
pub struct ListItem {
    id: ElementId,
    label: SharedString,
    icon: Option<Icon>,
    glyph: Option<SharedString>,
    meta: Option<SharedString>,
    selected: bool,
    disabled: bool,
    on_click: Option<ClickHandler>,
}

pub fn list_item(id: impl Into<ElementId>, label: impl Into<SharedString>) -> ListItem {
    ListItem { id: id.into(), label: label.into(), icon: None, glyph: None, meta: None, selected: false, disabled: false, on_click: None }
}

impl ListItem {
    /// A leading pixel icon (preferred over a glyph).
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn glyph(mut self, glyph: impl Into<SharedString>) -> Self {
        self.glyph = Some(glyph.into());
        self
    }

    pub fn meta(mut self, meta: impl Into<SharedString>) -> Self {
        self.meta = Some(meta.into());
        self
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn on_click(mut self, handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for ListItem {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = palette(cx);
        let focus: FocusHandle = window.use_keyed_state(self.id.clone(), cx, |_, cx| cx.focus_handle()).read(cx).clone();
        let ink = if self.disabled { p.fg_faint } else { p.fg };

        div()
            .id(self.id.clone())
            .role(Role::ListItem)
            .aria_selected(self.selected)
            .aria_label(self.label.clone())
            .track_focus(&focus)
            .when(!self.disabled, |el| el.tab_stop(true))
            .relative()
            .flex()
            .flex_row()
            .items_center()
            .gap_2()
            .h(crate::theme::row_height(cx))
            .pl(px(12.))
            .pr_2()
            .body(text::BASE)
            .text_color(hsla(ink))
            .border_1()
            .border_color(gpui::transparent_black())
            .children(super::selection(format!("list-{}", self.id), self.selected, window, cx))
            .when(!self.selected && !self.disabled, |el| el.hover(|s| s.bg(hsla(p.raised))))
            .when(!self.disabled, |el| el.focus_visible(|s| s.border_color(hsla(p.accent))))
            .when_some(self.icon, |el, i| {
                el.child(icon(i).color(hsla(if self.selected { p.accent_text } else { p.fg_dim })))
            })
            .when_some(self.glyph, |el, g| {
                el.child(div().w(px(14.)).text_color(hsla(if self.selected { p.accent_text } else { p.fg_dim })).child(g))
            })
            .child(div().flex_1().min_w_0().overflow_hidden().whitespace_nowrap().text_ellipsis().child(self.label))
            .when_some(self.meta, |el, m| el.child(div().body(text::SM).text_color(hsla(p.fg_dim)).child(m)))
            .when(!self.disabled, |el| el.on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default()))
            .when_some(self.on_click.filter(|_| !self.disabled), |el, h| el.on_click(move |e, w, cx| h(e, w, cx)))
    }
}
