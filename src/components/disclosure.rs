//! Disclosure: the [`accordion`] — stacked sections that open and close.
//!
//! ```text
//!  ┌──────────────────────────────────────────┐
//!  │ v [ GENERAL ]                        3   │   open: chevron down, content
//!  │   theme        iron                      │   unrolls in with a scan edge
//!  │   telemetry    off                       │
//!  ├──────────────────────────────────────────┤
//!  │ > [ ADVANCED ]                       12  │   closed
//!  └──────────────────────────────────────────┘
//! ```
//!
//! Which sections are open is the accordion's own state (seed it with
//! [`Accordion::open`]), like the tree's expansion. `.single()` keeps at most
//! one open. Every header is a tab stop; Enter/Space toggle it. Opening
//! animates (unroll), closing is instant — exits don't animate.
//!
//! One section on its own is a collapsible: `accordion(id).section(..)`.

use std::collections::HashSet;

use gpui::{
    AnyElement, App, ElementId, FocusHandle, InteractiveElement, IntoElement, MouseButton, ParentElement,
    RenderOnce, Role, SharedString, StatefulInteractiveElement, Styled, Window, div,
    prelude::FluentBuilder as _, px,
};

use crate::fonts::{FerriteText, Scale};
use crate::icon::{Icon, icon};
use crate::theme::palette;
use crate::tokens::hsla;

/// One section: a key, a title, optional meta, and its content.
pub struct AccordionSection {
    key: SharedString,
    title: SharedString,
    meta: Option<SharedString>,
    children: Vec<AnyElement>,
}

/// A section keyed by its title. Use [`AccordionSection::key`] when two
/// titles could collide.
pub fn accordion_section(title: impl Into<SharedString>) -> AccordionSection {
    let title = title.into();
    AccordionSection { key: title.clone(), title, meta: None, children: Vec::new() }
}

impl AccordionSection {
    pub fn key(mut self, key: impl Into<SharedString>) -> Self {
        self.key = key.into();
        self
    }

    /// Dim right-aligned text in the header (a count, a status).
    pub fn meta(mut self, meta: impl Into<SharedString>) -> Self {
        self.meta = Some(meta.into());
        self
    }
}

impl ParentElement for AccordionSection {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

#[derive(IntoElement)]
pub struct Accordion {
    id: ElementId,
    sections: Vec<AccordionSection>,
    seed: Vec<SharedString>,
    single: bool,
}

pub fn accordion(id: impl Into<ElementId>) -> Accordion {
    Accordion { id: id.into(), sections: Vec::new(), seed: Vec::new(), single: false }
}

impl Accordion {
    pub fn section(mut self, section: AccordionSection) -> Self {
        self.sections.push(section);
        self
    }

    /// Sections open on first render, by key.
    pub fn open(mut self, keys: impl IntoIterator<Item = impl Into<SharedString>>) -> Self {
        self.seed = keys.into_iter().map(Into::into).collect();
        self
    }

    /// At most one section open: opening one closes the others.
    pub fn single(mut self) -> Self {
        self.single = true;
        self
    }
}

/// Toggle `key` in `open`, closing the rest when `single`.
pub fn toggle_section(open: &mut HashSet<SharedString>, key: &SharedString, single: bool) {
    if open.remove(key) {
        return;
    }
    if single {
        open.clear();
    }
    open.insert(key.clone());
}

impl RenderOnce for Accordion {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = palette(cx);
        let seed = self.seed;
        let state = window.use_keyed_state(self.id.clone(), cx, move |_, _| seed.into_iter().collect::<HashSet<SharedString>>());
        let single = self.single;
        let mut root = div().id(self.id.clone()).flex().flex_col().border_1().border_color(hsla(p.line));

        for (i, section) in self.sections.into_iter().enumerate() {
            let open = state.read(cx).contains(&section.key);
            let header_id = ElementId::NamedChild(std::sync::Arc::new(self.id.clone()), format!("h-{}", section.key).into());
            let focus: FocusHandle = window.use_keyed_state(header_id.clone(), cx, |_, cx| cx.focus_handle()).read(cx).clone();
            let key = section.key.clone();
            let toggler = state.clone();
            let header = div()
                .id(header_id)
                .role(Role::Button)
                .aria_expanded(open)
                .aria_label(section.title.clone())
                .track_focus(&focus)
                .tab_stop(true)
                .flex()
                .flex_row()
                .items_center()
                .gap_2()
                .h(px(32.))
                .px_2()
                .bg(hsla(if open { p.surface } else { p.bg }))
                .when(i > 0, |el| el.border_t_1())
                .border_color(hsla(p.line))
                .hover(|s| s.bg(hsla(p.raised)))
                .focus_visible(|s| s.bg(hsla(p.raised)).text_color(hsla(p.accent_text)))
                .child(icon(if open { Icon::ChevronDown } else { Icon::ChevronRight }).fit(px(16.)).color(hsla(if open { p.accent } else { p.fg_dim })))
                .child(
                    div()
                        .flex_1()
                        .display(Scale::X1, window)
                        .text_color(hsla(if open { p.fg } else { p.fg_dim }))
                        .child(format!("[ {} ]", section.title.to_uppercase())),
                )
                .when_some(section.meta, |el, m| el.child(div().display(Scale::X1, window).text_color(hsla(p.fg_faint)).child(m)))
                .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
                .on_click(move |_, _, cx| {
                    toggler.update(cx, |open, cx| {
                        toggle_section(open, &key, single);
                        cx.notify();
                    })
                });
            root = root.child(header);
            if open {
                let body = div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .px_3()
                    .py_2()
                    .border_t_1()
                    .border_color(hsla(p.line))
                    .children(section.children);
                let reveal_id = ElementId::NamedChild(std::sync::Arc::new(self.id.clone()), format!("b-{}", section.key).into());
                root = root.child(super::fx::unroll_in(reveal_id, 0u8, body));
            }
        }
        root
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_mode_keeps_one_open() {
        let (a, b): (SharedString, SharedString) = ("a".into(), "b".into());
        let mut open = HashSet::new();
        toggle_section(&mut open, &a, true);
        toggle_section(&mut open, &b, true);
        assert_eq!(open.len(), 1);
        assert!(open.contains(&b));
        toggle_section(&mut open, &b, true);
        assert!(open.is_empty(), "toggling the open one closes it");
        toggle_section(&mut open, &a, false);
        toggle_section(&mut open, &b, false);
        assert_eq!(open.len(), 2, "multi mode keeps both");
    }
}
