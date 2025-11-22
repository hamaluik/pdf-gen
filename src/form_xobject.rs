//! Form XObjects for reusable PDF content with transformation support.
//!
//! Form XObjects (also known as XObject Forms) are self-contained content containers
//! that can be placed multiple times within a document. They're rendered once and
//! then referenced wherever needed, which is efficient for repeated content.
//!
//! # When to use Form XObjects
//!
//! - **Booklet imposition**: placing multiple logical pages on a single physical sheet
//! - **Repeated elements**: logos, watermarks, or headers that appear on many pages
//! - **Transformed content**: content that needs rotation, scaling, or translation
//! - **Content reuse**: any content you want to define once and place multiple times
//!
//! # Coordinate system
//!
//! Form XObjects use PDF's coordinate system where (0, 0) is at the bottom-left.
//! When placed on a page, the form's origin aligns with the page origin unless
//! a transformation is applied.

use crate::colour::Colour;
use crate::font::Font;
use crate::image::Image;
use crate::rect::Rect;
use crate::refs::{ObjectReferences, RefType};
use crate::units::*;
use crate::PDFError;
use id_arena::{Arena, Id};
use pdf_writer::{Content, Finish, Name, Pdf};
use std::io::Write;

use crate::page::{ImageLayout, PageContents, SpanFont, SpanLayout};

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

/// A reusable content container (Form XObject) that can be placed on pages
/// with transformations.
///
/// Form XObjects encapsulate content (text, images, raw PDF operations) into
/// a self-contained unit that can be placed anywhere in the document. This is
/// useful for:
///
/// - **Efficiency**: render complex content once and reference it multiple times
/// - **Transformations**: scale, rotate, or translate content when placing it
/// - **Booklet imposition**: place multiple logical pages on a single physical sheet
///
/// # Usage
///
/// 1. Create a `FormXObject` with a bounding box
/// 2. Add content using the same methods as [`Page`](crate::Page)
/// 3. Register it with the document via [`Document::add_form_xobject`](crate::Document::add_form_xobject)
/// 4. Place it on pages using [`Page::add_form_xobject`](crate::Page::add_form_xobject)
///    with a [`FormXObjectLayout`]
///
/// # Example
///
/// ```
/// use pdf_gen::{Document, Page, FormXObject, FormXObjectLayout, Transform, Pt};
/// use pdf_gen::pagesize;
///
/// let mut doc = Document::default();
///
/// // create a 2x2 inch form
/// let mut form = FormXObject::new(Pt(144.0), Pt(144.0));
/// // add_span, add_image, add_raw_content work just like on Page
///
/// let form_id = doc.add_form_xobject(form);
///
/// let mut page = Page::new(pagesize::LETTER, None);
/// // place the form translated 1 inch from the origin
/// page.add_form_xobject(FormXObjectLayout {
///     xobj_id: form_id,
///     transform: Transform::translate(Pt(72.0), Pt(72.0)),
/// });
/// doc.add_page(page);
/// ```
#[derive(Debug)]
pub struct FormXObject {
    /// Bounding box of the form content
    pub bbox: Rect,
    /// Content to render (same types as Page)
    pub contents: Vec<PageContents>,
}

impl FormXObject {
    /// Create a new Form XObject with the given dimensions
    pub fn new(width: Pt, height: Pt) -> Self {
        FormXObject {
            bbox: Rect {
                x1: Pt(0.0),
                y1: Pt(0.0),
                x2: width,
                y2: height,
            },
            contents: Vec::new(),
        }
    }

    /// Create a new Form XObject from an explicit bounding box
    pub fn from_bbox(bbox: Rect) -> Self {
        FormXObject {
            bbox,
            contents: Vec::new(),
        }
    }

    /// Width of the form
    pub fn width(&self) -> Pt {
        self.bbox.x2 - self.bbox.x1
    }

    /// Height of the form
    pub fn height(&self) -> Pt {
        self.bbox.y2 - self.bbox.y1
    }

    /// Add a span of text to the form
    pub fn add_span(&mut self, span: SpanLayout) {
        self.contents.push(PageContents::Text(vec![span]));
    }

    /// Add an image to the form
    pub fn add_image(&mut self, image: ImageLayout) {
        self.contents.push(PageContents::Image(image));
    }

    /// Add arbitrary pdf_writer::Content to the form
    pub fn add_content(&mut self, content: Content) {
        self.contents
            .push(PageContents::RawContent(content.finish()));
    }

    /// Add raw content bytes to the form
    pub fn add_raw_content<I>(&mut self, content: I)
    where
        I: IntoIterator<Item = u8>,
    {
        self.contents
            .push(PageContents::RawContent(content.into_iter().collect()));
    }

    /// Render the form contents to a byte stream
    #[allow(clippy::write_with_newline)]
    fn render(&self, fonts: &Arena<Font>) -> Result<Vec<u8>, std::io::Error> {
        if self.contents.is_empty() {
            return Ok(Vec::default());
        }
        let mut content: Vec<u8> = Vec::default();

        for page_content in self.contents.iter() {
            match page_content {
                PageContents::Text(spans) => {
                    if spans.is_empty() {
                        continue;
                    }

                    write!(&mut content, "q\n")?;
                    let mut current_font: SpanFont = spans.first().unwrap().font;
                    let mut current_colour: Colour = spans.first().unwrap().colour;

                    write!(
                        &mut content,
                        "/F{} {} Tf\n",
                        current_font.id.index(),
                        current_font.size
                    )?;
                    match current_colour {
                        Colour::RGB { r, g, b } => write!(&mut content, "{r} {g} {b} rg\n")?,
                        Colour::CMYK { c, m, y, k } => write!(&mut content, "{c} {m} {y} {k} k\n")?,
                        Colour::Grey { g } => write!(&mut content, "{g} g\n")?,
                    }

                    for span in spans.iter() {
                        if span.font != current_font {
                            current_font = span.font;
                            write!(
                                &mut content,
                                "/F{} {} Tf\n",
                                current_font.id.index(),
                                current_font.size
                            )?;
                        }
                        if span.colour != current_colour {
                            current_colour = span.colour;
                            match current_colour {
                                Colour::RGB { r, g, b } => {
                                    write!(&mut content, "{r} {g} {b} rg\n")?
                                }
                                Colour::CMYK { c, m, y, k } => {
                                    write!(&mut content, "{c} {m} {y} {k} k\n")?
                                }
                                Colour::Grey { g } => write!(&mut content, "{g} g\n")?,
                            }
                        }

                        write!(&mut content, "BT\n")?;
                        write!(&mut content, "{} {} Td\n", span.coords.0, span.coords.1)?;
                        write!(&mut content, "<")?;
                        for ch in span.text.chars() {
                            write!(
                                &mut content,
                                "{:04x}",
                                fonts[current_font.id].glyph_id(ch).unwrap_or_else(|| fonts
                                    [current_font.id]
                                    .replacement_glyph_id()
                                    .unwrap_or_else(|| fonts[current_font.id]
                                        .glyph_id('?')
                                        .expect("font has '?' glyph")))
                            )?;
                        }
                        write!(&mut content, "> Tj\n")?;
                        write!(&mut content, "ET\n")?;
                    }
                    write!(&mut content, "Q\n")?;
                }
                PageContents::Image(image) => {
                    write!(&mut content, "q\n")?;
                    write!(
                        &mut content,
                        "{} 0 0 {} {} {} cm\n",
                        image.position.x2 - image.position.x1,
                        image.position.y2 - image.position.y1,
                        image.position.x1,
                        image.position.y1
                    )?;
                    write!(&mut content, "/I{} Do\n", image.image_index)?;
                    write!(&mut content, "Q\n")?;
                }
                PageContents::RawContent(c) => {
                    write!(&mut content, "q\n")?;
                    content.write_all(c.as_slice())?;
                    write!(&mut content, "\nQ\n")?;
                }
                PageContents::FormXObject(layout) => {
                    let t = &layout.transform;
                    write!(&mut content, "q\n")?;
                    write!(
                        &mut content,
                        "{} {} {} {} {} {} cm\n",
                        t.a, t.b, t.c, t.d, t.e, t.f
                    )?;
                    write!(&mut content, "/X{} Do\n", layout.xobj_id.index())?;
                    write!(&mut content, "Q\n")?;
                }
            }
        }

        Ok(content)
    }

    /// Write this Form XObject to the PDF using a pre-generated ref.
    /// The ref must be generated before calling this method.
    pub(crate) fn write_with_ref(
        &self,
        refs: &ObjectReferences,
        xobj_index: usize,
        fonts: &Arena<Font>,
        images: &Arena<Image>,
        all_form_xobjects: &Arena<FormXObject>,
        writer: &mut Pdf,
    ) -> Result<(), PDFError> {
        let xobj_ref = refs
            .get(RefType::FormXObject(xobj_index))
            .expect("FormXObject ref should be pre-generated");

        let rendered = self.render(fonts)?;
        let compressed = miniz_oxide::deflate::compress_to_vec_zlib(
            &rendered,
            miniz_oxide::deflate::CompressionLevel::DefaultCompression as u8,
        );

        let mut xobj = writer.form_xobject(xobj_ref, &compressed);
        xobj.filter(pdf_writer::Filter::FlateDecode);
        xobj.bbox(self.bbox.into());

        // add resources (fonts, images, and other form xobjects)
        let mut resources = xobj.resources();
        let mut resource_fonts = resources.fonts();
        for (i, _) in fonts.iter().enumerate() {
            resource_fonts.pair(
                Name(format!("F{i}").as_bytes()),
                refs.get(RefType::Font(i)).unwrap(),
            );
        }
        resource_fonts.finish();

        let mut resource_xobjects = resources.x_objects();
        for (i, _) in images.iter().enumerate() {
            resource_xobjects.pair(
                Name(format!("I{i}").as_bytes()),
                refs.get(RefType::Image(i)).unwrap(),
            );
        }
        for (i, _) in all_form_xobjects.iter().enumerate() {
            resource_xobjects.pair(
                Name(format!("X{i}").as_bytes()),
                refs.get(RefType::FormXObject(i)).unwrap(),
            );
        }
        resource_xobjects.finish();
        resources.finish();

        Ok(())
    }
}

/// Specifies how to place a Form XObject on a page.
///
/// Combines the form to render with a transformation that controls where
/// and how it appears. The transformation is applied when the form is
/// rendered, allowing the same form to be placed multiple times with
/// different positions, scales, or rotations.
#[derive(Debug)]
pub struct FormXObjectLayout {
    /// The Form XObject to place (obtained from [`Document::add_form_xobject`](crate::Document::add_form_xobject))
    pub xobj_id: Id<FormXObject>,
    /// Transformation matrix applied when rendering this placement
    pub transform: Transform,
}
