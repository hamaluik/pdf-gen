use std::collections::VecDeque;

use crate::colour::Colour;
use crate::document::Document;
use crate::font::Font;
use crate::page::*;
use crate::rect::Rect;
use crate::units::Pt;
use id_arena::Id;
use owned_ttf_parser::AsFaceRef;

/// Calculates the vertical offset from a text coordinate to the font's baseline.
///
/// In PDF, text coordinates specify the baseline position. This function returns
/// the negative ascent value, which can be added to a y-coordinate to account
/// for the font's ascender height when positioning text from a top reference point.
pub fn baseline_offset(font: &Font, size: Pt) -> Pt {
    let scaling: Pt = size / Pt(font.face.as_face_ref().units_per_em() as f32);
    let ascent: Pt = scaling * font.face.as_face_ref().ascender() as f32;
    Pt(0.) - ascent
}

/// Calculates the coordinates of where text can start on a page to be just within the top left
/// margin, taking into account the ascending height of the font and the font size. Text is laid
/// out according to the `ContentBox` of the page, which is usually derived from the page size
/// and accompanying margins.
pub fn baseline_start(page: &Page, font: &Font, size: Pt) -> (Pt, Pt) {
    let ascent = baseline_offset(font, size);
    let x = page.content_box.x1;
    let y = page.content_box.y2 + ascent;
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
pub fn layout_text_naive(
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
            id: font_id,
            size: font_size,
        } = font;

        let scaling: Pt =
            font_size / document.fonts[font_id].face.as_face_ref().units_per_em() as f32;
        let leading: Pt = scaling * document.fonts[font_id].face.as_face_ref().line_gap() as f32;
        let ascent: Pt = scaling * document.fonts[font_id].face.as_face_ref().ascender() as f32;
        let descent: Pt = scaling * document.fonts[font_id].face.as_face_ref().descender() as f32;
        let line_gap: Pt = leading + ascent - descent;

        // replace tabs with spaces
        let span = span.replace('\t', &" ".repeat(TABSIZE));
        // normalize newlines
        let span = span.replace("\r\n", "\n").replace('\r', "\n");

        let mut current_span: SpanLayout = SpanLayout {
            text: "".into(),
            font: SpanFont {
                id: font_id,
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
                                id: font_id,
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
                                    id: font_id,
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

            let gid = document.fonts[font_id]
                .face
                .as_face_ref()
                .glyph_index(ch)
                .unwrap_or_else(|| {
                    document.fonts[font_id]
                        .face
                        .as_face_ref()
                        .glyph_index('\u{FFFD}')
                        //.expect("Font has a replacement glyph")
                        .unwrap_or_else(|| {
                            document.fonts[font_id]
                                .face
                                .as_face_ref()
                                .glyph_index('?')
                                .expect("font has a question mark glyph")
                        })
                });

            let hadv = scaling
                * document.fonts[font_id]
                    .face
                    .as_face_ref()
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
                                    id: font_id,
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

/// Tracks a position in the text layout where we can safely break to a new line.
///
/// Break points are only tracked within the current input span (at whitespace positions).
/// This simplifies the rewind logic by avoiding cross-span state.
#[derive(Clone)]
struct BreakPoint {
    /// index into the output spans vec where we can break
    output_span_idx: usize,
    /// character index within that output span's text
    output_char_idx: usize,
    /// character index within the current input span (where to resume from)
    input_char_idx: usize,
}

/// Lays out colored text spans with natural boundary wrapping.
///
/// # Wrapping Behavior
///
/// The algorithm tracks potential break points as it processes text and uses them
/// when a line overflows. Break points are recorded at:
///
/// 1. **Whitespace** - After any space, tab, or other whitespace character
/// 2. **Span boundaries** - Between input spans (useful for syntax-highlighted code
///    where each span represents a token)
///
/// When text would overflow the line width, the layout "rewinds" to the most recent
/// break point and continues on the next line. This keeps words and syntax tokens
/// intact. If no break point exists (e.g., a single very long identifier), the
/// algorithm falls back to character-level breaking to ensure text never overflows
/// the bounding box.
///
/// # Page Overflow
///
/// This function consumes the `text` parameter. Any content remaining in `text` after
/// the function returns represents text that would overflow the page vertically.
/// Callers should create a new page and call this function again with the remaining
/// text.
///
/// # Returns
///
/// The (x, y) coordinates where layout stopped, useful for continuing layout with
/// additional content.
pub fn layout_text_natural(
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

    // tracks the most recent valid break point within the current input span
    let mut last_break: Option<BreakPoint>;

    let mut input_idx = 0usize;
    'inputspans: while input_idx < text.len() {
        let (ref span_orig, colour, font) = text[input_idx];
        let SpanFont {
            id: font_id,
            size: font_size,
        } = font;

        let scaling: Pt =
            font_size / document.fonts[font_id].face.as_face_ref().units_per_em() as f32;
        let leading: Pt = scaling * document.fonts[font_id].face.as_face_ref().line_gap() as f32;
        let ascent: Pt = scaling * document.fonts[font_id].face.as_face_ref().ascender() as f32;
        let descent: Pt = scaling * document.fonts[font_id].face.as_face_ref().descender() as f32;
        let line_gap: Pt = leading + ascent - descent;

        // replace tabs with spaces
        let span = span_orig.replace('\t', &" ".repeat(TABSIZE));
        // normalize newlines
        let span = span.replace("\r\n", "\n").replace('\r', "\n");

        // reset break points at span boundaries - we only track breaks within the current span
        last_break = None;

        let mut current_span: SpanLayout = SpanLayout {
            text: "".into(),
            font: SpanFont {
                id: font_id,
                size: font_size,
            },
            colour,
            coords: (x, y),
        };

        let span_chars: Vec<char> = span.chars().collect();
        let mut ci = 0usize;

        'chars: while ci < span_chars.len() {
            let ch = span_chars[ci];

            if ch == '\n' {
                // push current span before newline
                if !current_span.text.is_empty() {
                    spans.push(current_span.clone());
                }

                // move to the next line
                x = start.0;
                y -= line_gap;
                last_break = None; // reset break points for new line

                // check if we would now overflow on the bottom
                if y < bounding_box.y1 + descent {
                    // return leftovers: rest of this span + remaining input spans
                    let remaining: String = span_chars[ci..].iter().collect();
                    text.drain(..=input_idx);
                    if !remaining.is_empty() {
                        text.insert(0, (remaining, colour, font));
                    }
                    break 'inputspans;
                } else {
                    // start fresh span on new line
                    ci += 1;
                    current_span = SpanLayout {
                        text: "".into(),
                        font: SpanFont {
                            id: font_id,
                            size: font_size,
                        },
                        colour,
                        coords: (x, y),
                    };
                    continue 'chars;
                }
            }

            let gid = document.fonts[font_id]
                .face
                .as_face_ref()
                .glyph_index(ch)
                .unwrap_or_else(|| {
                    document.fonts[font_id]
                        .face
                        .as_face_ref()
                        .glyph_index('\u{FFFD}')
                        .unwrap_or_else(|| {
                            document.fonts[font_id]
                                .face
                                .as_face_ref()
                                .glyph_index('?')
                                .expect("font has a question mark glyph")
                        })
                });

            let hadv = scaling
                * document.fonts[font_id]
                    .face
                    .as_face_ref()
                    .glyph_hor_advance(gid)
                    .unwrap_or_default() as f32;

            if x + hadv >= bounding_box.x2 {
                // overflow - try to break at whitespace within current span

                if let Some(bp) = last_break.take() {
                    // rewind to whitespace break point (within current span only)
                    while spans.len() > bp.output_span_idx + 1 {
                        spans.pop();
                    }
                    if let Some(last_span) = spans.last_mut() {
                        last_span.text = last_span.text.chars().take(bp.output_char_idx).collect();
                    }

                    // move to next line
                    x = start.0 + wrap_offset;
                    y -= line_gap;

                    // check for vertical overflow
                    if y < bounding_box.y1 + descent {
                        let remaining: String = span_chars[bp.input_char_idx..].iter().collect();
                        text.drain(..=input_idx);
                        if !remaining.is_empty() {
                            text.insert(0, (remaining, colour, font));
                        }
                        break 'inputspans;
                    }

                    // continue from break point (skip leading whitespace)
                    ci = bp.input_char_idx;
                    while ci < span_chars.len()
                        && span_chars[ci].is_whitespace()
                        && span_chars[ci] != '\n'
                    {
                        ci += 1;
                    }

                    current_span = SpanLayout {
                        text: "".into(),
                        font: SpanFont {
                            id: font_id,
                            size: font_size,
                        },
                        colour,
                        coords: (x, y),
                    };
                    continue 'chars;
                } else {
                    // no break point - force character break
                    spans.push(current_span.clone());

                    x = start.0 + wrap_offset;
                    y -= line_gap;

                    if y < bounding_box.y1 + descent {
                        let remaining: String = span_chars[ci..].iter().collect();
                        text.drain(..=input_idx);
                        if !remaining.is_empty() {
                            text.insert(0, (remaining, colour, font));
                        }
                        break 'inputspans;
                    }

                    current_span.text.clear();
                    current_span.text.push(ch);
                    current_span.coords = (x, y);
                    x += hadv;
                    ci += 1;
                    continue 'chars;
                }
            } else {
                // no overflow - add character and track break points
                current_span.text.push(ch);
                x += hadv;

                // record whitespace as break point (within current span only)
                if ch.is_whitespace() && ch != '\n' {
                    last_break = Some(BreakPoint {
                        output_span_idx: spans.len(),
                        output_char_idx: current_span.text.chars().count(),
                        input_char_idx: ci + 1,
                    });
                }

                ci += 1;
            }
        }

        if !current_span.text.is_empty() {
            spans.push(current_span);
        }
        input_idx += 1;
    }

    // drain processed input spans
    if input_idx > 0 && input_idx <= text.len() {
        text.drain(..input_idx);
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
    let scaling = size / font.face.as_face_ref().units_per_em() as f32;
    text.chars()
        .filter_map(|ch| font.glyph_id(ch))
        .map(|gid| {
            scaling
                * font
                    .face
                    .as_face_ref()
                    .glyph_hor_advance(owned_ttf_parser::GlyphId(gid))
                    .unwrap_or_default() as f32
        })
        .sum()
}

pub fn layout_text_spring(
    document: &Document,
    page: &mut Page,
    font_id: Id<Font>,
    size: Pt,
    text: &str,
    bounding_box: Rect,
) {
    struct Word<'a> {
        word: &'a str,
        width: Pt,
    }

    let font = document.fonts.get(font_id).expect("can get font");

    // split the text into words separated by springs (spaces)
    let mut words: VecDeque<Word> = VecDeque::default();
    for word in text.split_whitespace() {
        let width = width_of_text(word, font, size);
        words.push_back(Word { word, width });
    }

    let mut y = bounding_box.y2 + baseline_offset(font, size);
    let max_width = bounding_box.x2 - bounding_box.x1;
    let space_width = width_of_text(" ", font, size);

    'layout: loop {
        let mut words_width = Pt(0.);
        let mut line: Vec<Word> = Vec::default();
        'line: loop {
            if words.is_empty() {
                break 'line;
            }

            // try adding the word to the line
            let word = words.pop_front().expect("words is not empty");
            let word_width = word.width;
            line.push(word);

            words_width += word_width;
            let spaces_width = space_width * ((line.len() - 1) as f32);

            // check for overflow
            if words_width + spaces_width >= max_width {
                // overflowing!
                // see if we can squish the spaces down to fit
                if words_width + (spaces_width * 0.8) <= max_width {
                    // yes we can!
                    break 'line;
                } else {
                    // nope, that would be too tight. move this word back to the list and
                    // start a new line
                    words_width -= word_width;
                    words.push_front(line.pop().expect("word in line"));
                }
            } else {
                // not overflowing yet, we can add more text
            }
        }

        if !line.is_empty() {
            let mut x = bounding_box.x1;
            let space_width = (max_width - words_width) / ((line.len() - 1) as f32);
            for word in line {
                page.add_span(SpanLayout {
                    text: word.word.to_string(),
                    font: SpanFont { id: font_id, size },
                    colour: crate::colours::BLACK,
                    coords: (x, y),
                });
                x += word.width + space_width;
            }
        } else if line.is_empty() || words.is_empty() {
            break 'layout;
        }

        y += baseline_offset(font, size);
    }
}
