use crate::colour::Colour;
use crate::font::Font;
use crate::image::Image;
use crate::refs::{ObjectReferences, RefType};
use pdf_writer::Finish;
use pdf_writer::{Name, PdfWriter, Rect};
use std::io::Write;

#[derive(Copy, Clone, PartialEq, Debug)]
pub struct SpanFont {
    pub index: usize,
    pub size: f32,
}

#[derive(Clone, PartialEq, Debug)]
pub struct SpanLayout {
    pub text: String,
    pub font: SpanFont,
    pub colour: Colour,
    pub coords: (f32, f32),
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
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

impl Margins {
    pub fn trbl(top: f32, right: f32, bottom: f32, left: f32) -> Margins {
        Margins {
            top,
            right,
            bottom,
            left,
        }
    }

    pub fn all(value: f32) -> Margins {
        Margins {
            top: value,
            right: value,
            bottom: value,
            left: value,
        }
    }

    pub fn symmetric(vertical: f32, horizontal: f32) -> Margins {
        Margins {
            top: vertical,
            right: horizontal,
            bottom: vertical,
            left: horizontal,
        }
    }
}

impl Page {
    pub fn new(width: f32, height: f32, margins: Margins) -> Page {
        Page {
            media_box: Rect {
                x1: 0.0,
                y1: 0.0,
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

    pub fn baseline_start(&self, font: &Font, size: f32) -> (f32, f32) {
        let scaling = size / font.face.units_per_em() as f32;
        let ascent = font.face.ascender() as f32 * scaling;
        let x = self.content_box.x1;
        let y = self.content_box.y2 - ascent;
        (x, y)
    }

    /// Lays out text in a character-by-character manner, splitting all words at the exact end
    /// and not adding anything to the left. i.e. if the input were "asdf asdf" and the page
    /// only fit 6 characters wide, this will split the text in: "asdf a\nsdf". Applies these
    /// spans to the page contents, keeping colours intact for all rendered text.
    ///
    /// NOTE: this consumes the text parameter. Any content left in the text parameter after
    /// this function finishes is text that would have overflowed the page. Normally you would
    /// then create a new page and layout the text on that page as well.
    ///
    /// Returns the page coordinates of where the layout stopped, in case you ended up short
    pub fn layout_text(
        &mut self,
        start: (f32, f32),
        font: (usize, &Font, f32),
        text: &mut Vec<(String, Colour)>,
        bounding_box: Rect,
    ) -> (f32, f32) {
        if text.is_empty() {
            return start;
        }

        let scaling = font.2 / font.1.face.units_per_em() as f32;
        let leading = font.1.face.line_gap() as f32 * scaling;
        let ascent = font.1.face.ascender() as f32 * scaling;
        let descent = font.1.face.descender() as f32 * scaling;
        let line_gap = leading + ascent - descent;

        const TABSIZE: usize = 4;

        let mut x = start.0;
        let mut y = start.1;

        let mut spans: Vec<SpanLayout> = Vec::with_capacity(text.len());

        'inputspans: while !text.is_empty() {
            let (span, colour) = text.remove(0);
            // replace tabs with spaces
            let span = span.replace(
                "\t",
                std::iter::repeat(' ')
                    .take(TABSIZE)
                    .collect::<String>()
                    .as_str(),
            );
            // normalize newlines
            let span = span.replace("\r\n", "\n").replace("\r", "\n");

            let mut current_span: SpanLayout = SpanLayout {
                text: "".into(),
                font: SpanFont {
                    index: font.0,
                    size: font.2,
                },
                colour,
                coords: (x, y),
            };

            'chars: for (ci, ch) in span.chars().enumerate() {
                if ch == '\n' {
                    // collect what's left and push it to the front of the queue
                    let remaining: String = span.chars().skip(ci + 1).collect();
                    if !remaining.is_empty() {
                        text.insert(0, (remaining, colour));
                    }

                    // move to the next line
                    x = start.0;
                    y -= line_gap;

                    // finish off our current span
                    break 'chars;
                }

                let gid = font
                    .1
                    .face
                    .glyph_index(ch)
                    .expect("font contains glyph for char");

                let hadv = font.1.face.glyph_hor_advance(gid).unwrap_or_default() as f32 * scaling;

                if x + hadv >= bounding_box.x2 {
                    spans.push(current_span.clone());

                    x = start.0 + hadv;
                    y -= line_gap;

                    // check if we're overflowing on the bottom
                    if y < bounding_box.y1 + descent {
                        // yup, we're going to overflow. That's okay, just return our leftovers
                        // collect what's left of our current input span
                        let remaining: String = span.chars().skip(ci).collect();
                        if !remaining.is_empty() {
                            text.insert(0, (remaining, colour));
                        }

                        spans.push(current_span.clone());
                        break 'inputspans;
                    } else {
                        // not overflowing the bottom yet
                        current_span.text.clear();
                        current_span.text.push(ch);
                        current_span.coords.0 = start.0;
                        current_span.coords.1 = y;
                    }
                } else {
                    current_span.text.push(ch);
                    x += hadv;
                }
            }

            spans.push(current_span.clone());
        }

        for span in spans.into_iter() {
            if !span.text.is_empty() {
                self.add_span(span);
            }
        }

        (x, y)
    }

    /// ignores newlines / any glyphs not in the font
    pub fn width_of_text(text: &str, font: &Font, size: f32) -> f32 {
        let scaling = size / font.face.units_per_em() as f32;
        text.chars()
            .filter_map(|ch| font.glyph_id(ch))
            .map(|gid| {
                font.face
                    .glyph_hor_advance(ttf_parser::GlyphId(gid))
                    .unwrap_or_default() as f32
                    * scaling
            })
            .sum()
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

    pub fn write(
        &self,
        refs: &mut ObjectReferences,
        page_index: usize,
        fonts: &[Font],
        images: &[Image],
        writer: &mut PdfWriter,
    ) {
        let id = refs.get(RefType::Page(page_index)).unwrap();
        let mut page = writer.page(id);
        page.media_box(self.media_box.clone());
        page.art_box(self.content_box.clone());
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
