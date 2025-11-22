use crate::units::Pt;

/// Margins are used when laying out objects on a page. There is no control
/// preventing objects on pages to overflow the margins—the margins are there
/// as guidelines for layout functions. Additionally, the margins are applied
/// to [`Page`](crate::Page)s to determine the `ContentBox` attribute of each page in the
/// generated PDF
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Margins {
    pub top: Pt,
    pub right: Pt,
    pub bottom: Pt,
    pub left: Pt,
}

impl Margins {
    /// Create margins by specifying individual components in a clockwise fashion
    /// starting at the top (in the same order as CSS margins)
    pub fn trbl(top: Pt, right: Pt, bottom: Pt, left: Pt) -> Margins {
        Margins {
            top,
            right,
            bottom,
            left,
        }
    }

    /// Create margins where all values are equal
    pub fn all<D: Into<Pt>>(value: D) -> Margins {
        let value: Pt = value.into();
        Margins {
            top: value,
            right: value,
            bottom: value,
            left: value,
        }
    }

    /// Create margins by specifying different values for vertical (top and bottom)
    /// and horizontal (left and right) margins
    pub fn symmetric(vertical: Pt, horizontal: Pt) -> Margins {
        Margins {
            top: vertical,
            right: horizontal,
            bottom: vertical,
            left: horizontal,
        }
    }

    /// Create margins where all values are 0.0
    pub fn empty() -> Margins {
        Margins {
            top: Pt(0.0),
            right: Pt(0.0),
            bottom: Pt(0.0),
            left: Pt(0.0),
        }
    }

    /// Utility method to add a gutter to the left of the page,
    /// usually for even-numbered pages in bound documents
    pub fn with_gutter_left(&self, gutter: Pt) -> Margins {
        Margins {
            top: self.top,
            right: self.right,
            bottom: self.bottom,
            left: self.left + gutter,
        }
    }

    /// Utility method to add a gutter to the right of the page,
    /// usually for odd-numbered pages in bound documents
    pub fn with_gutter_right(&self, gutter: Pt) -> Margins {
        Margins {
            top: self.top,
            right: self.right + gutter,
            bottom: self.bottom,
            left: self.left,
        }
    }

    /// Utility function to add a gutter to either the left or the right
    /// side of the page, depending on whether the page index is:
    /// * _even_ => left
    /// * _odd_ => right
    pub fn with_gutter(&self, gutter: Pt, page_index: usize) -> Margins {
        if page_index % 2 == 0 {
            self.with_gutter_left(gutter)
        } else {
            self.with_gutter_right(gutter)
        }
    }
}
