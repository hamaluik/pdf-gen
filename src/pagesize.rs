//! Pre-defined page sizes for common paper formats.
//!
//! All sizes are provided in portrait orientation (width, height) where width ≤ height.
//! Use the [`PageOrientation`](crate::pagesize::PageOrientation) trait to convert between portrait and landscape.
//!
//! # Available Sizes
//!
//! ## North American
//! `LETTER`, `HALF_LETTER`, `JUNIOR_LEGAL`, `LEGAL`, `TABLOID`, `LEDGER`
//!
//! ## ANSI
//! `ANSI_A` through `ANSI_E`
//!
//! ## ISO A-Series
//! `A0` through `A6`
//!
//! ## Traditional
//! `FOLIO`, `QUARTO`, `OCTAVO`
//!
//! # Example
//!
//! ```
//! use pdf_gen::pagesize::{LETTER, A4, PageOrientation};
//!
//! // use a standard size
//! let page_size = LETTER;
//!
//! // convert to landscape
//! let landscape = A4.landscape();
//! ```

use crate::units::*;

/// Page dimensions as (width, height) in points.
pub type PageSize = (Pt, Pt);

// north american sizes
pub const LETTER: PageSize = (Pt(8.5 * 72.0), Pt(11.0 * 72.0));
pub const HALF_LETTER: PageSize = (Pt(5.5 * 72.0), Pt(8.5 * 72.0));
pub const JUNIOR_LEGAL: PageSize = (Pt(5.0 * 72.0), Pt(8.0 * 72.0));
pub const LEGAL: PageSize = (Pt(8.5 * 72.0), Pt(13.0 * 72.0));
pub const TABLOID: PageSize = (Pt(11.0 * 72.0), Pt(17.0 * 72.0));
pub const LEDGER: PageSize = (Pt(17.0 * 72.0), Pt(11.0 * 72.0));

// ansi sizes
pub const ANSI_A: PageSize = (Pt(8.5 * 72.0), Pt(11.0 * 72.0));
pub const ANSI_B: PageSize = (Pt(11.0 * 72.0), Pt(17.0 * 72.0));
pub const ANSI_C: PageSize = (Pt(17.0 * 72.0), Pt(22.0 * 72.0));
pub const ANSI_D: PageSize = (Pt(22.0 * 72.0), Pt(34.0 * 72.0));
pub const ANSI_E: PageSize = (Pt(34.0 * 72.0), Pt(44.0 * 72.0));

// traditional sizes
pub const FOLIO: PageSize = (Pt(12.0 * 72.0), Pt(19.0 * 72.0));
pub const QUARTO: PageSize = (Pt(9.5 * 72.0), Pt(12.0 * 72.0));
pub const OCTAVO: PageSize = (Pt(6.0 * 72.0), Pt(9.0 * 72.0));

// iso a-series (converted from mm to points)
pub const A0: PageSize = (Pt(841.0 * 72.0 / 25.4), Pt(1189.0 * 72.0 / 25.4));
pub const A1: PageSize = (Pt(594.0 * 72.0 / 25.4), Pt(841.0 * 72.0 / 25.4));
pub const A2: PageSize = (Pt(420.0 * 72.0 / 25.4), Pt(594.0 * 72.0 / 25.4));
pub const A3: PageSize = (Pt(297.0 * 72.0 / 25.4), Pt(420.0 * 72.0 / 25.4));
pub const A4: PageSize = (Pt(210.0 * 72.0 / 25.4), Pt(297.0 * 72.0 / 25.4));
pub const A5: PageSize = (Pt(148.0 * 72.0 / 25.4), Pt(210.0 * 72.0 / 25.4));
pub const A6: PageSize = (Pt(105.0 * 72.0 / 25.4), Pt(148.0 * 72.0 / 25.4));

/// Convert page sizes between portrait and landscape orientations.
pub trait PageOrientation {
    /// Returns the size in portrait orientation (width ≤ height).
    fn portrait(self) -> Self;
    /// Returns the size in landscape orientation (width ≥ height).
    fn landscape(self) -> Self;
}

impl PageOrientation for PageSize {
    fn portrait(self) -> Self {
        if self.0 <= self.1 {
            self
        } else {
            (self.1, self.0)
        }
    }

    fn landscape(self) -> PageSize {
        if self.0 >= self.1 {
            self
        } else {
            (self.1, self.0)
        }
    }
}
