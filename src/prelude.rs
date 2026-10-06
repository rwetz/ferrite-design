//! Everything a Ferrite view usually needs, in one import:
//!
//! ```ignore
//! use ferrite_design::prelude::*;
//! ```
//!
//! Brings in the components, the effects, the palette accessors and type
//! helpers, icons, dither, motion and the gpui traits component builders
//! rely on (`Styled`, `ParentElement`, `InteractiveElement`, `FluentBuilder`…).
//! It deliberately does *not* glob-import gpui itself: name gpui items
//! (`div`, `px`, `Context`, …) from gpui as usual.

pub use crate::components::*;
pub use crate::components::calendar::Date;
pub use crate::components::tag::Tone;
pub use crate::animate::{self, Edge};
pub use crate::chrome::{self, title_bar, window_frame};
pub use crate::dither::{self, dither};
pub use crate::fonts::{FerriteText, Scale, display_size};
pub use crate::icon::{Icon, icon};
pub use crate::motion;
pub use crate::theme::{self, Appearance, palette};
pub use crate::tokens::{hsla, hsla_a, space, text};

pub use gpui::prelude::FluentBuilder as _;
pub use gpui::{InteractiveElement as _, ParentElement as _, StatefulInteractiveElement as _, Styled as _};
