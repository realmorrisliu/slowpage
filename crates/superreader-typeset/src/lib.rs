//! Headless chapter typesetter for the owned EPUB engine.
//!
//! The shell paints those runs for an EPUB. `SPIKE_TYPESET=1` is the same
//! request. `SPIKE_TYPESET=0` stays on WKWebView. A chapter the crate cannot
//! open stays on WKWebView.
//! icu_segmenter is Unicode-3.0. rbook is Apache-2.0. harfrust is MIT.
//! skrifa is MIT OR Apache-2.0. tiny-skia is BSD-3-Clause.

mod book;
mod compose;
mod face;
mod gate;
mod hyphen;
mod interact;
mod kp;
mod parse;
mod types;

pub use book::{load_chapter, LoadedChapter};
pub use compose::{ChapterLayout, Typesetter};
pub use face::{list_faces, BOLD_FACE, BOLD_ITALIC_FACE};
pub use gate::{nearest_rank_p95, phase1, Phase1Failure, Phase1Input};
pub use interact::{
    blocked_ease, chapter_plain, chapter_text, turn_sentence, ChapterText, SLIDE_MS,
};
pub use types::*;
