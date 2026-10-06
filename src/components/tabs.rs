//! The tab strip (docs/COMPONENTS.md #6).
//!
//! `│ LOGS │ METRICS │ CONFIG │` — display-face labels separated by
//! hairlines. The active tab drops onto the content (`bg`, no bottom line)
//! and carries a 2px amber top edge; the rest sit on `surface` in dim ink.
//! Controlled: pass `.selected(i)`, get the new index in `on_select`.

use std::rc::Rc;

use gpui::{
    App, ElementId, FocusHandle, InteractiveElement, IntoElement, MouseButton, ParentElement,
    RenderOnce, Role, SharedString, StatefulInteractiveElement, Styled, Window, div,
    prelude::FluentBuilder as _, px,
};

use crate::fonts::{FerriteText, Scale};
use crate::theme::palette;
use crate::tokens::hsla;

type SelectHandler = Rc<dyn Fn(&usize, &mut Window, &mut App)>;

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
        let count = self.items.len();
        let mut row = div()
            .id(self.id.clone())
            .role(Role::TabList)
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
                    .when(active, |el| {
                        // Grows out from the centre in eased steps when the
                        // selection lands here.
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
    }
}
