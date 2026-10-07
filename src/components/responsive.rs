//! Content built for the room it actually gets.
//!
//! gpui has no zoom: `px` are logical pixels and the display face is only
//! crisp at whole multiples of its 8×16 cell. So "bigger when the window is
//! bigger" can't be one global switch; each region decides how to use its
//! space. [`responsive`] tells it how much space that is, and
//! [`whole_scale`] turns that into a crisp integer multiple.
//!
//! ```ignore
//! responsive("sky", |room, window, cx| {
//!     let cols = (f32::from(room.width) / 8.) as usize;   // one display cell = 8px
//!     ascii_film("film", frames.clone()).cols(cols).into_any_element()
//! })
//! .flex_1()
//! ```

use std::rc::Rc;

use gpui::{
    AnyElement, App, Bounds, ElementId, InteractiveElement, IntoElement, ParentElement, Pixels, RenderOnce, Size, StyleRefinement,
    Styled, Window, canvas, div,
};

type Build = Rc<dyn Fn(Size<Pixels>, &mut Window, &mut App) -> AnyElement>;

/// A box that hands its own measured size to `build`. Size it like any div
/// (`.flex_1()`, `.size_full()`); it never sizes itself from its content.
///
/// The size is measured at paint and used on the next render, so after a
/// resize the content catches up one frame later, and on its very first
/// frame (size zero) it draws nothing. Inside `power_on_in` neither shows.
#[derive(IntoElement)]
pub struct Responsive {
    id: ElementId,
    style: StyleRefinement,
    build: Build,
}

pub fn responsive(id: impl Into<ElementId>, build: impl Fn(Size<Pixels>, &mut Window, &mut App) -> AnyElement + 'static) -> Responsive {
    Responsive { id: id.into(), style: StyleRefinement::default(), build: Rc::new(build) }
}

impl Styled for Responsive {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Responsive {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let room = window.use_keyed_state(self.id.clone(), cx, |_, _| Size::<Pixels>::default());
        let size = *room.read(cx);
        let measure = canvas(
            move |bounds: Bounds<Pixels>, window, cx| {
                if *room.read(cx) != bounds.size {
                    room.update(cx, |r, _| *r = bounds.size);
                    window.refresh();
                }
            },
            |_, _, _, _| {},
        )
        .absolute()
        .inset_0();

        let mut root = div();
        *root.style() = self.style;
        let content = (size.width > Pixels::ZERO && size.height > Pixels::ZERO).then(|| (self.build)(size, window, cx));
        root.id(self.id).relative().overflow_hidden().child(measure).children(content)
    }
}

/// The largest whole multiple (1..=`max`) at which something `natural` in
/// size fits in `room`. 1 when even that doesn't fit: callers clip or
/// scroll, they don't go below their designed size.
pub fn whole_scale(room: Size<Pixels>, natural: Size<Pixels>, max: u32) -> u32 {
    let ratio = |r: Pixels, n: Pixels| if n > Pixels::ZERO { f32::from(r) / f32::from(n) } else { f32::MAX };
    let k = ratio(room.width, natural.width).min(ratio(room.height, natural.height));
    (k.floor() as u32).clamp(1, max.max(1))
}

#[cfg(test)]
mod tests {
    use super::whole_scale;
    use gpui::{px, size};

    #[test]
    fn whole_scale_fits_both_ways() {
        let natural = size(px(400.), px(100.));
        assert_eq!(whole_scale(size(px(1000.), px(1000.)), natural, 4), 2); // width-bound
        assert_eq!(whole_scale(size(px(4000.), px(310.)), natural, 4), 3); // height-bound
        assert_eq!(whole_scale(size(px(9000.), px(9000.)), natural, 4), 4); // capped
        assert_eq!(whole_scale(size(px(100.), px(50.)), natural, 4), 1); // never below 1
        assert_eq!(whole_scale(size(px(800.), px(200.)), size(px(0.), px(0.)), 3), 3);
    }
}
