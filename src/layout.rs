use crate::colour::Colour;
use crate::document::Document;
use crate::font::Font;
use crate::page::*;
use crate::rect::Rect;
use crate::units::Pt;

/// Calculates the coordinates of where text can start on a page to be just within the top left
/// margin, taking into account the ascending height of the font and the font size.
///
/// Example:
/// ```
///
/// ```
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
    font_index: usize,
    font_size: Pt,
    text: &mut Vec<(String, Colour)>,
    bounding_box: Rect,
) -> (Pt, Pt) {
    if text.is_empty() {
        return start;
    }

    let scaling: Pt = font_size / document.fonts[font_index].face.units_per_em() as f32;
    let leading: Pt = scaling * document.fonts[font_index].face.line_gap() as f32;
    let ascent: Pt = scaling * document.fonts[font_index].face.ascender() as f32;
    let descent: Pt = scaling * document.fonts[font_index].face.descender() as f32;
    let line_gap: Pt = leading + ascent - descent;

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
                    text.insert(0, (remaining, colour));
                }

                // move to the next line
                x = start.0;
                y -= line_gap;

                // finish off our current span
                break 'chars;
            }

            let gid = document.fonts[font_index]
                .face
                .glyph_index(ch)
                .expect("font contains glyph for char");

            let hadv = scaling
                * document.fonts[font_index]
                    .face
                    .glyph_hor_advance(gid)
                    .unwrap_or_default() as f32;

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
            page.add_span(span);
        }
    }

    (x, y)
}

/// Calculate
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
