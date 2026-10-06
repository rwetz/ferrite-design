//! The shared cache of rasterised images (dither fields, pixel icons).
//!
//! Anything Ferrite draws pixel-by-pixel is rendered once into a BGRA image
//! at device resolution and painted afterwards as a single textured quad
//! (PITFALLS §17). This cache owns those images: bounded, least-recently-used
//! eviction, and evicted textures are freed from the GPU atlas so window
//! resizes can't leak.

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

use gpui::{Bounds, Corners, Hsla, Pixels, RenderImage, Window, point, px, size};

/// What an image is, precisely enough that equal keys mean equal pixels.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Key {
    Dither { field: [u32; 3], w: u32, h: u32, cell: u32, ink: [u8; 4], paper: [u8; 4] },
    Icon { icon: u8, scale: u32, ink: [u8; 4] },
}

/// Past this many images the least recently used one is evicted.
const CAP: usize = 128;

#[derive(Default)]
struct Cache {
    images: HashMap<Key, (Arc<RenderImage>, u64)>,
    clock: u64,
}

thread_local! {
    static CACHE: RefCell<Cache> = RefCell::default();
}

/// The cached image for `key`, rendering it with `make() -> (w, h, bgra)` on
/// a miss.
pub(crate) fn image(key: Key, window: &mut Window, make: impl FnOnce() -> (u32, u32, Vec<u8>)) -> Arc<RenderImage> {
    let (image, evicted) = CACHE.with_borrow_mut(|cache| {
        cache.clock += 1;
        let now = cache.clock;
        if let Some((image, used)) = cache.images.get_mut(&key) {
            *used = now;
            return (image.clone(), None);
        }
        let (w, h, bytes) = make();
        let buffer = image::RgbaImage::from_raw(w, h, bytes).expect("raster size matches buffer");
        let image = Arc::new(RenderImage::new([image::Frame::new(buffer)]));
        cache.images.insert(key, (image.clone(), now));
        let evicted = (cache.images.len() > CAP).then(|| {
            let oldest = *cache.images.iter().min_by_key(|(_, (_, used))| *used).unwrap().0;
            cache.images.remove(&oldest).unwrap().0
        });
        (image, evicted)
    });
    if let Some(old) = evicted {
        let _ = window.drop_image(old);
    }
    image
}

/// Paint an image whose pixels are exactly `w`×`h` device pixels at a
/// device-snapped origin, so it lands 1:1 on the screen grid (no blur).
pub(crate) fn paint(image: Arc<RenderImage>, origin_x: f32, origin_y: f32, w: u32, h: u32, window: &mut Window) {
    let sf = window.scale_factor();
    let target: Bounds<Pixels> = Bounds::new(
        point(px((origin_x * sf).round() / sf), px((origin_y * sf).round() / sf)),
        size(px(w as f32 / sf), px(h as f32 / sf)),
    );
    let _ = window.paint_image(target, target, Corners::default(), image, 0, false);
}

pub(crate) fn bgra(color: Hsla) -> [u8; 4] {
    let c = color.to_rgb();
    let b = |f: f32| (f.clamp(0.0, 1.0) * 255.0).round() as u8;
    [b(c.b), b(c.g), b(c.r), b(c.a)]
}
