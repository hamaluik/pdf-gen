//! 2D transformation matrices for PDF content positioning.

use crate::units::*;
use pdf_writer::Content;

/// A transformation matrix for positioning Form XObjects on a page.
///
/// Uses the standard PDF transformation matrix where (0,0) is at the bottom-left.
/// The matrix is represented as [a, b, c, d, e, f] corresponding to:
/// ```text
/// | a  b  0 |
/// | c  d  0 |
/// | e  f  1 |
/// ```
///
/// # Composing transforms
///
/// Transforms can be chained using [`then`](Transform::then) or the builder methods
/// [`with_translate`](Transform::with_translate) and [`with_scale`](Transform::with_scale).
/// Operations are applied in the order they're chained.
///
/// ```
/// use pdf_gen::{Transform, Pt};
///
/// // scale content to half size, then move it 72 points right and up
/// let transform = Transform::scale(0.5, 0.5)
///     .with_translate(Pt(72.0), Pt(72.0));
/// ```
#[derive(Debug, Clone, Copy)]
pub struct Transform {
    pub a: f32,
    pub b: f32,
    pub c: f32,
    pub d: f32,
    pub e: f32,
    pub f: f32,
}

impl Default for Transform {
    fn default() -> Self {
        Self::identity()
    }
}

impl Transform {
    /// Identity transform (no transformation)
    pub fn identity() -> Self {
        Transform {
            a: 1.0,
            b: 0.0,
            c: 0.0,
            d: 1.0,
            e: 0.0,
            f: 0.0,
        }
    }

    /// Create a translation transform
    pub fn translate(x: Pt, y: Pt) -> Self {
        Transform {
            a: 1.0,
            b: 0.0,
            c: 0.0,
            d: 1.0,
            e: *x,
            f: *y,
        }
    }

    /// Create a scaling transform
    pub fn scale(sx: f32, sy: f32) -> Self {
        Transform {
            a: sx,
            b: 0.0,
            c: 0.0,
            d: sy,
            e: 0.0,
            f: 0.0,
        }
    }

    /// Create a rotation transform (angle in radians)
    pub fn rotate(angle: f32) -> Self {
        let cos = angle.cos();
        let sin = angle.sin();
        Transform {
            a: cos,
            b: sin,
            c: -sin,
            d: cos,
            e: 0.0,
            f: 0.0,
        }
    }

    /// Combine this transform with another (self * other)
    pub fn then(self, other: Transform) -> Self {
        Transform {
            a: self.a * other.a + self.b * other.c,
            b: self.a * other.b + self.b * other.d,
            c: self.c * other.a + self.d * other.c,
            d: self.c * other.b + self.d * other.d,
            e: self.e * other.a + self.f * other.c + other.e,
            f: self.e * other.b + self.f * other.d + other.f,
        }
    }

    /// Add a translation to this transform
    pub fn with_translate(self, x: Pt, y: Pt) -> Self {
        self.then(Transform::translate(x, y))
    }

    /// Add a scale to this transform
    pub fn with_scale(self, sx: f32, sy: f32) -> Self {
        self.then(Transform::scale(sx, sy))
    }

    /// Write the transform to a PDF content stream
    pub fn write_to_content(&self, content: &mut Content) {
        content.transform([self.a, self.b, self.c, self.d, self.e, self.f]);
    }
}
