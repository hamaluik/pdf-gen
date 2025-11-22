use crate::colour::Colour;
use crate::font::Font;
use crate::image::Image;
use crate::layout::Margins;
use crate::rect::Rect;
use crate::refs::{ObjectReferences, RefType};
use crate::{units::*, PDFError};
use id_arena::{Arena, Id};
use pdf_writer::{Content, Finish};
use pdf_writer::{Name, PdfWriter};
use std::io::Write;

pub use self::pagesize::PageSize;

/// What font to use for a given span of text
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct SpanFont {
    /// Which font to use for the span
    pub id: Id<Font>,
    /// The size of the text
    pub size: Pt,
}

impl SpanFont {
    fn font_index(&self) -> usize {
        self.id.index()
    }
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

    #[allow(clippy::write_with_newline)]
    fn render(&self, fonts: &Arena<Font>) -> Result<Vec<u8>, std::io::Error> {
        if self.contents.is_empty() {
            return Ok(Vec::default());
        }
        let mut content: Vec<u8> = Vec::default();

        'contents: for page_content in self.contents.iter() {
            match page_content {
                PageContents::Text(spans) => {
                    if spans.is_empty() {
                        continue 'contents;
                    }

                    write!(&mut content, "q\n")?;
                    // unwrap is safe, as we know spans isn't empty
                    let mut current_font: SpanFont = spans.first().unwrap().font;
                    let mut current_colour: Colour = spans.first().unwrap().colour;

                    write!(
                        &mut content,
                        "/F{} {} Tf\n",
                        current_font.font_index(),
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
                                current_font.font_index(),
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
                                    //.expect("Font has replacement glyph")
                                    .unwrap_or_else(|| fonts[current_font.id]
                                        .glyph_id('?')
                                        .expect("Font has '?' glyph!")))
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
            }
        }

        Ok(content)
    }

    pub(crate) fn write(
        &self,
        refs: &mut ObjectReferences,
        page_index: usize,
        page_order: &Vec<Id<Page>>,
        fonts: &Arena<Font>,
        images: &Arena<Image>,
        writer: &mut PdfWriter,
    ) -> Result<(), PDFError> {
        // unwrap is ok, because we SHOULD panic if this page index doesn't already exist
        // as the references are managed by the library (specifically, Document::write)
        let id = refs.get(RefType::Page(page_index)).unwrap();
        let mut page = writer.page(id);
        page.media_box(self.media_box.into());
        page.art_box(self.content_box.into());
        page.parent(refs.get(RefType::PageTree).unwrap());

        if !self.links.is_empty() {
            let mut annotations = page.annotations();
            for link in self.links.iter() {
                // convert link target to page_order index for ref lookup
                let page_ref = match link.page {
                    PageLinkReference::ById(id) => {
                        // find the position of this arena ID in page_order
                        page_order
                            .iter()
                            .position(|&page_id| page_id == id)
                            .ok_or(PDFError::PageMissing)?
                    }
                    PageLinkReference::ByIndex(idx) => idx,
                };

                let mut annotation = annotations.push();
                annotation.subtype(pdf_writer::types::AnnotationType::Link);
                annotation.rect(link.position.into());
                annotation.flags(pdf_writer::types::AnnotationFlags::INVISIBLE);
                annotation.border(0.0, 0.0, 0.0, None);
                annotation.color_transparent();
                annotation
                    .action()
                    .action_type(pdf_writer::types::ActionType::GoTo)
                    .destination_direct()
                    .page(refs.get(RefType::Page(page_ref)).unwrap())
                    .fit();
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
        resource_xobjects.finish();
        resources.finish();

        let content_id = refs.gen(RefType::ContentForPage(page_index));
        page.contents(content_id);
        page.finish();

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

/// Pre-defined page sizes for common usage
pub mod pagesize {
    use crate::units::*;

    /// The size of a page in points
    pub type PageSize = (Pt, Pt);

    pub const LETTER: (Pt, Pt) = (Pt(8.5 * 72.0), Pt(11.0 * 72.0));
    pub const HALF_LETTER: (Pt, Pt) = (Pt(5.5 * 72.0), Pt(8.5 * 72.0));
    pub const JUNIOR_LEGAL: (Pt, Pt) = (Pt(5.0 * 72.0), Pt(8.0 * 72.0));
    pub const LEGAL: (Pt, Pt) = (Pt(8.5 * 72.0), Pt(13.0 * 72.0));
    pub const TABLOID: (Pt, Pt) = (Pt(11.0 * 72.0), Pt(17.0 * 72.0));
    pub const LEDGER: (Pt, Pt) = (Pt(17.0 * 72.0), Pt(11.0 * 72.0));

    pub const ANSI_A: (Pt, Pt) = (Pt(8.5 * 72.0), Pt(11.0 * 72.0));
    pub const ANSI_B: (Pt, Pt) = (Pt(11.0 * 72.0), Pt(17.0 * 72.0));
    pub const ANSI_C: (Pt, Pt) = (Pt(17.0 * 72.0), Pt(22.0 * 72.0));
    pub const ANSI_D: (Pt, Pt) = (Pt(22.0 * 72.0), Pt(34.0 * 72.0));
    pub const ANSI_E: (Pt, Pt) = (Pt(34.0 * 72.0), Pt(44.0 * 72.0));

    pub const FOLIO: (Pt, Pt) = (Pt(12.0 * 72.0), Pt(19.0 * 72.0));
    pub const QUARTO: (Pt, Pt) = (Pt(9.5 * 72.0), Pt(12.0 * 72.0));
    pub const OCTAVO: (Pt, Pt) = (Pt(6.0 * 72.0), Pt(9.0 * 72.0));

    pub const A0: (Pt, Pt) = (Pt(841.0 * 72.0 / 25.4), Pt(1189.0 * 72.0 / 25.4));
    pub const A1: (Pt, Pt) = (Pt(594.0 * 72.0 / 25.4), Pt(841.0 * 72.0 / 25.4));
    pub const A2: (Pt, Pt) = (Pt(420.0 * 72.0 / 25.4), Pt(594.0 * 72.0 / 25.4));
    pub const A3: (Pt, Pt) = (Pt(297.0 * 72.0 / 25.4), Pt(420.0 * 72.0 / 25.4));
    pub const A4: (Pt, Pt) = (Pt(210.0 * 72.0 / 25.4), Pt(297.0 * 72.0 / 25.4));
    pub const A5: (Pt, Pt) = (Pt(148.0 * 72.0 / 25.4), Pt(210.0 * 72.0 / 25.4));
    pub const A6: (Pt, Pt) = (Pt(105.0 * 72.0 / 25.4), Pt(148.0 * 72.0 / 25.4));

    pub trait PageOrientation {
        fn portrait(self) -> Self;
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
}
