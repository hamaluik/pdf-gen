use crate::colour::Colour;
use crate::document::Document;
use crate::font::Font;
use crate::page::*;
use crate::rect::Rect;
use crate::units::Pt;

/// Margins are used when laying out objects on a page. There is no control
/// preventing objects on pages to overflow the margins—the margins are there
/// as guidelines for layout functions. Additionally, the margins are applied
/// to [Page]s to determine the `ContentBox` attribute of each page in the
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
    pub fn all(value: Pt) -> Margins {
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

    /// Utility method to add a guttern to the right of the page,
    /// usually for even-numbered pages
    pub fn with_gutter_left(&self, gutter: Pt) -> Margins {
        Margins {
            top: self.top,
            right: self.right,
            bottom: self.bottom,
            left: self.left + gutter,
        }
    }

    /// Utility method to add a gutter to the right of the page,
    /// usually for odd-numbered pages
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

/// Calculates the coordinates of where text can start on a page to be just within the top left
/// margin, taking into account the ascending height of the font and the font size. Text is laid
/// out according to the `ContentBox` of the page, which is usually derived from the page size
/// and accompanying margins.
pub fn baseline_start(page: &Page, font: &Font, size: Pt) -> (Pt, Pt) {
    let scaling: Pt = size / Pt(font.face.units_per_em() as f32);
    let ascent: Pt = scaling * font.face.ascender() as f32;
    let x = page.content_box.x1;
    let y = page.content_box.y2 - ascent;
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
    document: &Document,
    page: &mut Page,
    start: (Pt, Pt),
    text: &mut Vec<(String, Colour, SpanFont)>,
    wrap_offset: Pt,
    bounding_box: Rect,
) -> (Pt, Pt) {
    if text.is_empty() {
        return start;
    }

    const TABSIZE: usize = 4;

    let mut x = start.0;
    let mut y = start.1;

    let mut spans: Vec<SpanLayout> = Vec::with_capacity(text.len());

    'inputspans: while !text.is_empty() {
        let (span, colour, font) = text.remove(0);
        let SpanFont {
            index: font_index,
            size: font_size,
        } = font;

        let scaling: Pt = font_size / document.fonts[font_index].face.units_per_em() as f32;
        let leading: Pt = scaling * document.fonts[font_index].face.line_gap() as f32;
        let ascent: Pt = scaling * document.fonts[font_index].face.ascender() as f32;
        let descent: Pt = scaling * document.fonts[font_index].face.descender() as f32;
        let line_gap: Pt = leading + ascent - descent;

        // replace tabs with spaces
        let span = span.replace('\t', &" ".repeat(TABSIZE));
        // normalize newlines
        let span = span.replace("\r\n", "\n").replace('\r', "\n");

        let mut current_span: SpanLayout = SpanLayout {
            text: "".into(),
            font: SpanFont {
                index: font_index,
                size: font_size,
            },
            colour,
            coords: (x, y),
        };

        'chars: for (ci, ch) in span.chars().enumerate() {
            if ch == '\n' {
                // collect what's left and push it to the front of the queue
                let remaining: String = span.chars().skip(ci + 1).collect();
                if !remaining.is_empty() {
                    text.insert(
                        0,
                        (
                            remaining,
                            colour,
                            SpanFont {
                                index: font_index,
                                size: font_size,
                            },
                        ),
                    );
                }

                // move to the next line
                x = start.0;
                y -= line_gap;

                // check if we would now overflow on the bottom
                if y < bounding_box.y1 + descent {
                    // yup, we're going to overflow. That's okay, just return our leftovers
                    // collect what's left of our current input span
                    let remaining: String = span.chars().skip(ci).collect();
                    if !remaining.is_empty() {
                        text.insert(
                            0,
                            (
                                remaining,
                                colour,
                                SpanFont {
                                    index: font_index,
                                    size: font_size,
                                },
                            ),
                        );
                    }

                    spans.push(current_span.clone());
                    break 'inputspans;
                } else {
                    // finish off our current span
                    break 'chars;
                }
            }

            let gid = document.fonts[font_index]
                .face
                .glyph_index(ch)
                .unwrap_or_else(|| {
                    document.fonts[font_index]
                        .face
                        .glyph_index('\u{FFFD}')
                        .expect("Font has a replacement glyph")
                });

            let hadv = scaling
                * document.fonts[font_index]
                    .face
                    .glyph_hor_advance(gid)
                    .unwrap_or_default() as f32;

            if x + hadv >= bounding_box.x2 {
                // stop the current span
                spans.push(current_span.clone());

                // start a new span on the next line
                x = start.0 + wrap_offset;
                y -= line_gap;

                // check if we're overflowing on the bottom
                if y < bounding_box.y1 + descent {
                    // yup, we're going to overflow. That's okay, just return our leftovers
                    // collect what's left of our current input span
                    let remaining: String = span.chars().skip(ci).collect();
                    if !remaining.is_empty() {
                        text.insert(
                            0,
                            (
                                remaining,
                                colour,
                                SpanFont {
                                    index: font_index,
                                    size: font_size,
                                },
                            ),
                        );
                    }

                    spans.push(current_span.clone());
                    break 'inputspans;
                } else {
                    // not overflowing the bottom yet,
                    current_span.text.clear();
                    current_span.text.push(ch);
                    current_span.coords.0 = x;
                    current_span.coords.1 = y;

                    x += hadv;
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
            page.add_span(span);
        }
    }

    (x, y)
}

/// Calculate the width of a given string of text given the font and font size
pub fn width_of_text(text: &str, font: &Font, size: Pt) -> Pt {
    let scaling = size / font.face.units_per_em() as f32;
    text.chars()
        .filter_map(|ch| font.glyph_id(ch))
        .map(|gid| {
            scaling
                * font
                    .face
                    .glyph_hor_advance(ttf_parser::GlyphId(gid))
                    .unwrap_or_default() as f32
        })
        .sum()
}
