//! The tab strip (docs/COMPONENTS.md #6).
//!
//! `│ LOGS │ METRICS │ CONFIG │` — display-face labels separated by
//! hairlines. The active tab drops onto the content (`bg`, no bottom line)
//! and carries a 2px amber top edge; the rest sit on `surface` in dim ink.
//! Controlled: pass `.selected(i)`, get the new index in `on_select`.
//!
//! Motion: when the selection changes the amber edge *seeks* — it steps
//! from the old tab to the new one, resizing as it goes.

use std::cell::RefCell;
use std::rc::Rc;

use gpui::{
    App, Bounds, ElementId, FocusHandle, InteractiveElement, IntoElement, MouseButton, ParentElement,
    Pixels, RenderOnce, Role, SharedString, StatefulInteractiveElement, Styled, Window, div,
    prelude::FluentBuilder as _, px,
};

use crate::fonts::{FerriteText, Scale};
use crate::theme::palette;
use crate::tokens::hsla;

type SelectHandler = Rc<dyn Fn(&usize, &mut Window, &mut App)>;

/// What seeking needs between frames: where the edge came from, and where
/// each tab was last laid out (row-relative x, width).
struct Seek {
    from: usize,
    last: usize,
    tabs: Rc<RefCell<Vec<(Pixels, Pixels)>>>,
}

#[derive(IntoElement)]
pub struct Tabs {
    id: ElementId,
    items: Vec<(SharedString, Option<SharedString>)>,
    selected: usize,
    on_select: Option<SelectHandler>,
}

pub fn tabs(id: impl Into<ElementId>) -> Tabs {
    Tabs { id: id.into(), items: Vec::new(), selected: 0, on_select: None }
}

impl Tabs {
    pub fn tab(mut self, label: impl Into<SharedString>) -> Self {
        self.items.push((label.into(), None));
        self
    }

    /// A tab with a dim count/status after the label: `LOGS 12`.
    pub fn tab_with_meta(mut self, label: impl Into<SharedString>, meta: impl Into<SharedString>) -> Self {
        self.items.push((label.into(), Some(meta.into())));
        self
    }

    pub fn selected(mut self, index: usize) -> Self {
        self.selected = index;
        self
    }

    pub fn on_select(mut self, handler: impl Fn(&usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_select = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for Tabs {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = palette(cx);
        let bar = crate::animate::play_on_change("bar", self.selected, crate::motion::BASE, window, cx).eased();
        let selected = self.selected;
        let seek_id = ElementId::NamedChild(std::sync::Arc::new(self.id.clone()), "seek".into());
        let seek = window.use_keyed_state(seek_id, cx, |_, _| Seek { from: selected, last: selected, tabs: Rc::default() });
        if seek.read(cx).last != selected {
            seek.update(cx, |s, _| {
                s.from = s.last;
                s.last = selected;
            });
        }
        let tabs = seek.read(cx).tabs.clone();
        let from = seek.read(cx).from;
        // Seek between the two tabs' last laid-out positions; with none yet
        // (first change before a layout), fall back to growing in place.
        let seeking = (bar < 1. && from != selected)
            .then(|| {
                let laid_out = tabs.borrow();
                Some((*laid_out.get(from)?, *laid_out.get(selected)?))
            })
            .flatten()
            .map(|((x0, w0), (x1, w1))| (x0 + (x1 - x0) * bar, w0 + (w1 - w0) * bar));
        let count = self.items.len();
        let record = tabs.clone();
        // `on_children_prepainted` lives on plain divs, so it goes before `.id`.
        let mut row = div()
            .on_children_prepainted(move |bounds: Vec<Bounds<Pixels>>, _, _| {
                let Some(origin) = bounds.first().map(|b| b.left()) else { return };
                *record.borrow_mut() = bounds.iter().take(count).map(|b| (b.left() - origin, b.size.width)).collect();
            })
            .id(self.id.clone())
            .role(Role::TabList)
            .relative()
            .flex()
            .flex_row()
            .items_end()
            .h(px(32.))
            .bg(hsla(p.surface));

        for (i, (label, meta)) in self.items.into_iter().enumerate() {
            let active = i == self.selected;
            let id = ElementId::NamedChild(self.id.clone().into(), format!("tab-{i}").into());
            let focus: FocusHandle = window.use_keyed_state(id.clone(), cx, |_, cx| cx.focus_handle()).read(cx).clone();
            let handler = self.on_select.clone();
            row = row.child(
                div()
                    .id(id)
                    .role(Role::Tab)
                    .aria_selected(active)
                    .aria_position_in_set(i + 1)
                    .aria_size_of_set(count)
                    .aria_label(label.clone())
                    .track_focus(&focus)
                    .tab_stop(true)
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_2()
                    .h_full()
                    .px_3()
                    .relative()
                    .border_r_1()
                    .border_color(hsla(p.line))
                    .map(|el| {
                        if active {
                            // No bottom line: the active tab opens onto the content.
                            el.bg(hsla(p.bg)).text_color(hsla(p.fg))
                        } else {
                            el.border_b_1()
                                .text_color(hsla(p.fg_dim))
                                .hover(|s| s.text_color(hsla(p.fg)).bg(hsla(p.raised)))
                        }
                    })
                    .focus_visible(|s| s.text_color(hsla(p.accent_text)))
                    // gpui has no per-side border colors, so the amber top
                    // edge is its own 2px bar.
                    .when(active && seeking.is_none(), |el| {
                        // Grows out from the centre in eased steps when the
                        // selection lands here and there's nowhere to seek from.
                        let w = bar;
                        el.child(div().absolute().top_0().left(gpui::relative((1. - w) / 2.)).w(gpui::relative(w)).h(px(2.)).bg(hsla(p.accent)))
                    })
                    .display(Scale::X1, window)
                    .child(label.to_uppercase())
                    .when_some(meta, |el, m| el.child(div().text_color(hsla(p.fg_faint)).child(m)))
                    .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
                    .when_some(handler, |el, h| el.on_click(move |_, window, cx| h(&i, window, cx))),
            );
        }
        // The rest of the strip carries the hairline the active tab breaks.
        row.child(div().flex_1().h_full().border_b_1().border_color(hsla(p.line)))
            .when_some(seeking, |el, (x, w)| el.child(div().absolute().top_0().left(x).w(w).h(px(2.)).bg(hsla(p.accent))))
    }
}
