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

use crate::content::render_contents;
use crate::font::Font;
use crate::image::Image;
use crate::rect::Rect;
use crate::refs::{ObjectReferences, RefType};
use crate::transform::Transform;
use crate::units::*;
use crate::PDFError;
use id_arena::{Arena, Id};
use pdf_writer::{Content, Finish, Name, Pdf};

use crate::page::{ImageLayout, PageContents, SpanLayout};

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

    fn render(&self, fonts: &Arena<Font>) -> Result<Vec<u8>, std::io::Error> {
        render_contents(&self.contents, fonts)
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
