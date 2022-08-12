use crate::colour::Colour;
use crate::font::Font;
use crate::image::Image;
use crate::rect::Rect;
use crate::refs::{ObjectReferences, RefType};
use crate::units::*;
use pdf_writer::Finish;
use pdf_writer::{Name, PdfWriter};
use std::io::Write;

use self::pagesize::PageSize;

#[derive(Copy, Clone, PartialEq, Debug)]
pub struct SpanFont {
    pub index: usize,
    pub size: Pt,
}

#[derive(Clone, PartialEq, Debug)]
pub struct SpanLayout {
    pub text: String,
    pub font: SpanFont,
    pub colour: Colour,
    pub coords: (Pt, Pt),
}

#[derive(Clone, PartialEq, Debug)]
pub struct ImageLayout {
    pub image_index: usize,
    pub position: Rect,
}

#[derive(Clone, PartialEq, Debug)]
pub enum PageContents {
    Text(Vec<SpanLayout>),
    Image(ImageLayout),
}

pub struct Page {
    /// The size of the page
    pub media_box: Rect,
    /// Where content can live, i.e. within the margins
    pub content_box: Rect,
    /// The laid out text
    pub contents: Vec<PageContents>,
}

pub struct Margins {
    pub top: Pt,
    pub right: Pt,
    pub bottom: Pt,
    pub left: Pt,
}

impl Margins {
    pub fn trbl(top: Pt, right: Pt, bottom: Pt, left: Pt) -> Margins {
        Margins {
            top,
            right,
            bottom,
            left,
        }
    }

    pub fn all(value: Pt) -> Margins {
        Margins {
            top: value,
            right: value,
            bottom: value,
            left: value,
        }
    }

    pub fn symmetric(vertical: Pt, horizontal: Pt) -> Margins {
        Margins {
            top: vertical,
            right: horizontal,
            bottom: vertical,
            left: horizontal,
        }
    }

    pub fn empty() -> Margins {
        Margins {
            top: Pt(0.0),
            right: Pt(0.0),
            bottom: Pt(0.0),
            left: Pt(0.0),
        }
    }
}

impl Page {
    pub fn new(size: PageSize, margins: Margins) -> Page {
        let (width, height) = size;

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
        }
    }

    pub fn add_span(&mut self, span: SpanLayout) {
        self.contents.push(PageContents::Text(vec![span]));
    }

    pub fn add_image(&mut self, image: ImageLayout) {
        self.contents.push(PageContents::Image(image));
    }

    fn render(&self, fonts: &[Font]) -> Vec<u8> {
        if self.contents.is_empty() {
            return Vec::default();
        }
        let mut content: Vec<u8> = Vec::default();

        for page_content in self.contents.iter() {
            match page_content {
                PageContents::Text(spans) => {
                    write!(&mut content, "q\n").unwrap();
                    let mut current_font: SpanFont = spans.first().unwrap().font;
                    let mut current_colour: Colour = spans.first().unwrap().colour;

                    write!(
                        &mut content,
                        "/F{} {} Tf\n",
                        current_font.index, current_font.size
                    )
                    .unwrap();
                    write!(
                        &mut content,
                        "{} {} {} rg\n",
                        current_colour.r, current_colour.g, current_colour.b
                    )
                    .unwrap();

                    for span in spans.iter() {
                        if span.font != current_font {
                            current_font = span.font;
                            write!(
                                &mut content,
                                "/F{} {} Tf\n",
                                current_font.index, current_font.size
                            )
                            .unwrap();
                        }
                        if span.colour != current_colour {
                            current_colour = span.colour;
                            write!(
                                &mut content,
                                "{} {} {} rg\n",
                                current_colour.r, current_colour.g, current_colour.b
                            )
                            .unwrap();
                        }

                        write!(&mut content, "BT\n").unwrap();
                        write!(&mut content, "{} {} Td\n", span.coords.0, span.coords.1).unwrap();
                        write!(&mut content, "<").unwrap();
                        for ch in span.text.chars() {
                            write!(&mut content, "{:04x}", fonts[0].glyph_id(ch).unwrap()).unwrap();
                        }
                        write!(&mut content, "> Tj\n").unwrap();
                        write!(&mut content, "ET\n").unwrap();
                    }
                    write!(&mut content, "Q\n").unwrap();
                }
                PageContents::Image(image) => {
                    write!(&mut content, "q\n").unwrap();
                    write!(
                        &mut content,
                        "{} 0 0 {} {} {} cm\n",
                        image.position.x2 - image.position.x1,
                        image.position.y2 - image.position.y1,
                        image.position.x1,
                        image.position.y1
                    )
                    .unwrap();
                    write!(&mut content, "/I{} Do\n", image.image_index).unwrap();
                    write!(&mut content, "Q\n").unwrap();
                }
            }
        }

        content
    }

    pub(crate) fn write(
        &self,
        refs: &mut ObjectReferences,
        page_index: usize,
        fonts: &[Font],
        images: &[Image],
        writer: &mut PdfWriter,
    ) {
        let id = refs.get(RefType::Page(page_index)).unwrap();
        let mut page = writer.page(id);
        page.media_box(self.media_box.into());
        page.art_box(self.content_box.into());
        page.parent(refs.get(RefType::PageTree).unwrap());

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

        let rendered = self.render(fonts);
        writer.stream(content_id, rendered.as_slice());
    }
}

pub mod pagesize {
    use crate::units::*;

    pub type PageSize = (Pt, Pt);

    pub const LETTER: (Pt, Pt) = (Pt(8.5 * 72.0), Pt(11.0 * 72.0));
    pub const HALF_LETTER: (Pt, Pt) = (Pt(5.5 * 72.0), Pt(8.5 * 72.0));
    pub const JUNIOR_LEGAL: (Pt, Pt) = (Pt(5.0 * 72.0), Pt(8.0 * 72.0));
    pub const LEGAL: (Pt, Pt) = (Pt(8.5 * 72.0), Pt(13.0 * 72.0));
    pub const TABLOID: (Pt, Pt) = (Pt(11.0 * 72.0), Pt(17.0 * 72.0));
    pub const LEDGER: (Pt, Pt) = (Pt(17.0 * 72.0), Pt(11.0 * 72.0));
}
