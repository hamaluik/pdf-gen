//! Text layout utilities for positioning content on pages.
//!
//! This module provides tools for calculating text positions and laying out
//! multi-line text with automatic wrapping. The layout functions handle
//! page overflow by returning unconsumed text that can be laid out on
//! subsequent pages.
//!
//! # Layout Functions
//!
//! Three layout strategies are available:
//!
//! - [`layout_text_naive`](crate::layout::layout_text_naive) - character-by-character wrapping, splits words at line boundaries
//! - [`layout_text_natural`](crate::layout::layout_text_natural) - word-aware wrapping, keeps words and tokens intact
//! - [`layout_text_spring`](crate::layout::layout_text_spring) - justified text with variable word spacing
//!
//! # Example
//!
//! ```
//! use pdf_gen::{Document, Page, Font, Colour, SpanFont, Pt};
//! use pdf_gen::layout::{Margins, baseline_start, layout_text_natural};
//! use pdf_gen::pagesize;
//!
//! let font_data = include_bytes!("../assets/FiraMono-Regular.ttf");
//! let font = Font::load(font_data.to_vec()).expect("can load font");
//!
//! let mut doc = Document::default();
//! let font_id = doc.add_font(font);
//!
//! let mut page = Page::new(pagesize::LETTER, Some(Margins::all(Pt(72.0))));
//! let start = baseline_start(&page, &doc.fonts[font_id], Pt(12.0));
//! let bbox = page.content_box;
//!
//! let mut text = vec![(
//!     "Hello, world!".to_string(),
//!     Colour::Grey { g: 0.0 },
//!     SpanFont { id: font_id, size: Pt(12.0) },
//! )];
//!
//! layout_text_natural(&doc, &mut page, start, &mut text, Pt(0.0), bbox);
//! doc.add_page(page);
//! ```

mod margins;
mod text;

pub use margins::*;
pub use text::*;
