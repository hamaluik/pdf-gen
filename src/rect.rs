use crate::units::*;

/// A rectangle, specified by two opposite corners.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Rect {
    /// The x-coordinate of the first (typically, lower-left) corner.
    pub x1: Pt,
    /// The y-coordinate of the first (typically, lower-left) corner.
    pub y1: Pt,
    /// The x-coordinate of the second (typically, upper-right) corner.
    pub x2: Pt,
    /// The y-coordinate of the second (typically, upper-right) corner.
    pub y2: Pt,
}

impl From<Rect> for pdf_writer::Rect {
    fn from(r: Rect) -> Self {
        pdf_writer::Rect {
            x1: r.x1.into(),
            y1: r.y1.into(),
            x2: r.x2.into(),
            y2: r.y2.into(),
        }
    }
}

impl From<&Rect> for pdf_writer::Rect {
    fn from(r: &Rect) -> Self {
        pdf_writer::Rect {
            x1: r.x1.into(),
            y1: r.y1.into(),
            x2: r.x2.into(),
            y2: r.y2.into(),
        }
    }
}

impl From<pdf_writer::Rect> for Rect {
    fn from(r: pdf_writer::Rect) -> Self {
        Rect {
            x1: Pt(r.x1),
            y1: Pt(r.y1),
            x2: Pt(r.x2),
            y2: Pt(r.y2),
        }
    }
}

impl From<&pdf_writer::Rect> for Rect {
    fn from(r: &pdf_writer::Rect) -> Self {
        Rect {
            x1: Pt(r.x1),
            y1: Pt(r.y1),
            x2: Pt(r.x2),
            y2: Pt(r.y2),
        }
    }
}
