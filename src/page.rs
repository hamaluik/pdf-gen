use crate::colour::Colour;
use crate::content::render_contents;
use crate::font::Font;
use crate::form_xobject::{FormXObject, FormXObjectLayout};
use crate::image::Image;
use crate::layout::Margins;
use crate::pagesize::PageSize;
use crate::rect::Rect;
use crate::refs::{ObjectReferences, RefType};
use crate::{units::*, PDFError};
use id_arena::{Arena, Id};
use pdf_writer::{Content, Finish};
use pdf_writer::{Name, Pdf, Ref};

/// What font to use for a given span of text
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct SpanFont {
    /// Which font to use for the span
    pub id: Id<Font>,
    /// The size of the text
    pub size: Pt,
}


/// A section of text to be laid out onto a page
#[derive(Clone, PartialEq, Debug)]
pub struct SpanLayout {
    /// The actual text to print on the page
    pub text: String,
    /// What font should be used to print the text
    pub font: SpanFont,
    /// The colour of the span of text
    pub colour: Colour,
    /// The coordinates of where the text should start on the page,
    /// measured from the bottom-left corner of the page to the
    /// horizontal beginning and baseline of the text
    pub coords: (Pt, Pt),
}

/// An image to be laid out onto a page
#[derive(Clone, PartialEq, Debug)]
pub struct ImageLayout {
    /// Which image within the document to print
    pub image_index: usize,
    /// Where the image should be laid out on the page, relative to
    /// the bottom-left corner of the page
    pub position: Rect,
}

/// The types of content that can be rendered on the page
#[derive(Debug)]
pub enum PageContents {
    /// A block of text (broken into spans)
    Text(Vec<SpanLayout>),
    /// An image
    Image(ImageLayout),
    /// Raw content, typically rendered by [pdf_writer::Content]. The
    /// content **MUST** be **UNCOMPRESSED**.
    RawContent(Vec<u8>),
    /// A Form XObject placed with a transformation
    FormXObject(FormXObjectLayout),
}

/// A reference to page via its Id or 0-based page index
#[derive(Debug)]
pub enum PageLinkReference {
    /// Refer to a page by it's Id (resilient to page re-ordering)
    ById(Id<Page>),
    /// Refer to a page by it's 0-based index (will fail page-reordering but
    /// doesn't require you to know the page Id of a page that hasn't been
    /// created yet)
    ByIndex(usize),
}

/// An annotated region on the page that when clicked on, will navigate to the
/// given page index
#[derive(Debug)]
pub struct IntraDocumentLink {
    /// The bounding box for the link
    pub position: Rect,

    /// The page to navigate to when clicked
    pub page: PageLinkReference,
}

/// A page in the document
#[derive(Debug)]
pub struct Page {
    /// The size of the page
    pub media_box: Rect,
    /// Where content can live, i.e. within the margins
    pub content_box: Rect,
    /// The laid out text
    pub contents: Vec<PageContents>,
    /// Any links that are on the page
    pub links: Vec<IntraDocumentLink>,
}

impl Page {
    /// Create a new page with the given size. Margins can be specified, which will determine the
    /// `ContentBox` property of the page in the resulting PDF. If margins are not specified, the
    /// default margins (0 pt) are used.
    pub fn new(size: PageSize, margins: Option<Margins>) -> Page {
        let (width, height) = size;
        let margins = margins.unwrap_or_default();

        Page {
            media_box: Rect {
                x1: Pt(0.0),
                y1: Pt(0.0),
                x2: width,
                y2: height,
            },
            content_box: Rect {
                x1: margins.left,
                y1: margins.bottom,
                x2: width - margins.right,
                y2: height - margins.top,
            },
            contents: Vec::default(),
            links: Vec::default(),
        }
    }

    /// Add a span of text to the page, in the layering order that it was added
    pub fn add_span(&mut self, span: SpanLayout) {
        self.contents.push(PageContents::Text(vec![span]));
    }

    /// Add an image to the page, in the layering order that it was added
    pub fn add_image(&mut self, image: ImageLayout) {
        self.contents.push(PageContents::Image(image));
    }

    /// Add arbitrary `pdf_writer::Content` to the page. Surrounds the content by the `q` and `Q`
    /// operators to segregate the drawing content from other operations
    ///
    /// Note that fonts are referred to by name as `/Fi` where `i` is the font index
    /// Note that image xobjects are referred to by name as `/Ii` where `i` is the image index
    pub fn add_content(&mut self, content: Content) {
        self.contents
            .push(PageContents::RawContent(content.finish()));
    }

    /// Add content, rendering it yourself. Refer to the pdf specifications (pdf_reference_1-7)
    /// for full information about how to render this.
    ///
    /// Note that fonts are referred to by name as `/Fi` where `i` is the font index
    /// Note that image xobjects are referred to by name as `/Ii` where `i` is the image index
    pub fn add_raw_content<I>(&mut self, content: I)
    where
        I: IntoIterator<Item = u8>,
    {
        self.contents
            .push(PageContents::RawContent(content.into_iter().collect()));
    }

    /// Add a Form XObject to the page with the given transformation.
    /// Form XObjects are reusable content containers that can be placed with
    /// transformations (scale, rotate, translate).
    pub fn add_form_xobject(&mut self, layout: FormXObjectLayout) {
        self.contents.push(PageContents::FormXObject(layout));
    }

    /// Add a link on the page that when clicked will navigate to the given page index
    pub fn add_intradocument_link_by_id(&mut self, position: Rect, page: Id<Page>) {
        self.links.push(IntraDocumentLink {
            position,
            page: PageLinkReference::ById(page),
        });
    }

    /// Add a link on the page that when clicked will navigate to the given page index
    pub fn add_intradocument_link_by_index(&mut self, position: Rect, page: usize) {
        self.links.push(IntraDocumentLink {
            position,
            page: PageLinkReference::ByIndex(page),
        });
    }

    fn render(&self, fonts: &Arena<Font>) -> Result<Vec<u8>, std::io::Error> {
        render_contents(&self.contents, fonts)
    }

    pub(crate) fn write(
        &self,
        refs: &mut ObjectReferences,
        page_index: usize,
        page_order: &[Id<Page>],
        fonts: &Arena<Font>,
        images: &Arena<Image>,
        form_xobjects: &Arena<FormXObject>,
        writer: &mut Pdf,
    ) -> Result<(), PDFError> {
        // unwrap is ok, because we SHOULD panic if this page index doesn't already exist
        // as the references are managed by the library (specifically, Document::write)
        let id = refs.get(RefType::Page(page_index)).unwrap();
        let mut page = writer.page(id);
        page.media_box(self.media_box.into());
        page.art_box(self.content_box.into());
        page.parent(refs.get(RefType::PageTree).unwrap());

        // collect annotation data for later writing
        let mut annotation_data: Vec<(Ref, Rect, Ref)> = Vec::new();
        if !self.links.is_empty() {
            // generate refs for all annotations
            let annotation_refs: Vec<Ref> = self
                .links
                .iter()
                .map(|_| refs.gen(RefType::Annotation(page_index, annotation_data.len())))
                .collect();

            // set annotation refs on the page
            page.annotations(annotation_refs.iter().copied());

            // collect data needed to write annotations after finishing the page
            for (link, annot_ref) in self.links.iter().zip(annotation_refs.iter()) {
                let target_page_ref = match link.page {
                    PageLinkReference::ById(id) => page_order
                        .iter()
                        .position(|&page_id| page_id == id)
                        .ok_or(PDFError::PageMissing)?,
                    PageLinkReference::ByIndex(idx) => idx,
                };
                annotation_data.push((
                    *annot_ref,
                    link.position,
                    refs.get(RefType::Page(target_page_ref)).unwrap(),
                ));
            }
        }

        let mut resources = page.resources();
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
        for (i, _) in form_xobjects.iter().enumerate() {
            resource_xobjects.pair(
                Name(format!("X{i}").as_bytes()),
                refs.get(RefType::FormXObject(i)).unwrap(),
            );
        }
        resource_xobjects.finish();
        resources.finish();

        let content_id = refs.gen(RefType::ContentForPage(page_index));
        page.contents(content_id);
        page.finish();

        // write annotations after finishing the page
        for (annot_ref, position, target_page_ref) in annotation_data {
            let mut annotation = writer.annotation(annot_ref);
            annotation.subtype(pdf_writer::types::AnnotationType::Link);
            annotation.rect(position.into());
            annotation.flags(pdf_writer::types::AnnotationFlags::INVISIBLE);
            annotation.border(0.0, 0.0, 0.0, None);
            annotation.color_transparent();
            annotation
                .action()
                .action_type(pdf_writer::types::ActionType::GoTo)
                .destination()
                .page(target_page_ref)
                .fit();
        }

        let rendered = self.render(fonts)?;
        let compressed = miniz_oxide::deflate::compress_to_vec_zlib(
            &rendered,
            miniz_oxide::deflate::CompressionLevel::DefaultCompression as u8,
        );
        writer
            .stream(content_id, compressed.as_slice())
            .filter(pdf_writer::Filter::FlateDecode);

        Ok(())
    }
}

