//! A mid-level, opionated library for generating PDF documents
//!
//! Provides abstractions over [pdf-writer](https://crates.io/crates/pdf-writer) while including
//! features and utilities such as:
//!
//! * Unicode font embedding
//! * Raster and SVG image embedding
//! * Page generation with laid out text spans, images, or raw PDF contents
//! * Document metadata
//! * Compressed streams where possible
//! * Basic text layout utilities
//!
//! # Hello World Example
//!
//! ```
//! use pdf_gen::colours;
//! use pdf_gen::layout;
//! use pdf_gen::pagesize;
//! use pdf_gen::Document;
//! use pdf_gen::Font;
//! use pdf_gen::{layout::Margins, Page, SpanFont, SpanLayout};
//! use pdf_gen::{In, Pt};
//!
//! fn main() {
//!     // load a font to embed and use
//!     let fira_mono = include_bytes!("../assets/FiraMono-Regular.ttf");
//!     let fira_mono = Font::load(fira_mono.to_vec()).expect("can load font");
//!
//!     // start a document and add the font to it
//!     let mut doc = Document::default();
//!     let fira_mono = doc.add_font(fira_mono);
//!
//!     // create a page that is US Letter paper sized (8.5 x 11 inches)
//!     // with a margin around all edges of the page of 0.5 inches
//!     let mut page = Page::new(pagesize::LETTER, Some(Margins::all(In(0.5).into())));
//!
//!     // calculate where we should place text to have it at the top-left of the page within the margins
//!     let start = layout::baseline_start(&page, &doc.fonts[fira_mono], Pt(16.0));
//!
//!     // add a span of text to the page
//!     page.add_span(SpanLayout {
//!         // that will say "Hello world!"
//!         text: "Hello world!".to_string(),
//!         // that will be presented in size 16pt Fira Mono font
//!         font: SpanFont {
//!             id: fira_mono,
//!             size: Pt(16.0),
//!         },
//!         // that will be black
//!         colour: colours::BLACK,
//!         // and start where we calculated it should go before
//!         coords: start,
//!     });
//!
//!     // don't forget to add the page to the document (or it won't be rendered!)
//!     doc.add_page(page);
//!
//!     // we're going to save the contents to a file on disk, but anywhere where we can write would do
//!     let mut out = std::fs::File::create("hello-world.pdf").unwrap();
//!
//!     // render the document!
//!     doc.write(&mut out).unwrap();
//! }
//! ```

/// re-export some of our dependencies so dependents can use them if they need
pub use ::image as image_crate;
pub use id_arena as id_arena_crate;
pub use pdf_writer as pdf_writer_crate;
pub use usvg as usvg_crate;

mod colour;
pub use colour::*;

mod document;
pub use document::*;

mod font;
pub use font::*;

mod image;
pub use self::image::*;

mod info;
pub use info::*;

/// Utility functions and structures to layout objects (most text) on pages
pub mod layout;

mod page;
pub use page::*;

mod rect;
pub use rect::*;

pub(crate) mod refs;

mod units;
pub use units::*;

mod error;
pub use error::*;

mod outline;
pub use outline::*;
