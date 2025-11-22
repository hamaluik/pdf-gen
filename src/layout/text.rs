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
/// When laying out text, we want to avoid breaking words mid-character. This struct
/// captures enough state to "rewind" the layout process back to a previous position
/// if we discover that the current word/token won't fit on the line.
#[derive(Clone)]
struct BreakPoint {
    /// index into the output spans vec where we can break
    span_idx: usize,
    /// character index within that span's text (break happens AFTER this index)
    char_idx: usize,
    /// index into the input text queue - which input span we were processing
    input_idx: usize,
    /// character index within the input span (where to resume from)
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

    // tracks the most recent valid break point on the current line
    let mut last_break: Option<BreakPoint> = None;
    // tracks x position at start of current line (for detecting if we have any break points)
    let mut line_start_x = start.0;

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

        // record span boundary as a potential break point (before we start this span)
        // but only if we're not at the start of a line
        if x > line_start_x && !spans.is_empty() {
            last_break = Some(BreakPoint {
                span_idx: spans.len() - 1,
                char_idx: spans.last().map(|s| s.text.chars().count()).unwrap_or(0),
                input_idx,
                input_char_idx: 0,
            });
        }

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
                line_start_x = x;
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
                // overflow detected - try to break at a natural boundary

                if let Some(ref bp) = last_break {
                    // we have a break point - rewind to it
                    // truncate spans back to break point
                    while spans.len() > bp.span_idx + 1 {
                        spans.pop();
                    }
                    // truncate the break point span's text if needed
                    if let Some(last_span) = spans.last_mut() {
                        let truncated: String = last_span.text.chars().take(bp.char_idx).collect();
                        last_span.text = truncated;
                    }

                    // move to next line
                    x = start.0 + wrap_offset;
                    y -= line_gap;
                    line_start_x = x;

                    // check for vertical overflow
                    if y < bounding_box.y1 + descent {
                        // return everything from break point onwards
                        let remaining: String = span_chars[bp.input_char_idx..].iter().collect();
                        text.drain(..bp.input_idx);
                        if !remaining.is_empty() {
                            text.insert(0, (remaining, colour, font));
                        }
                        break 'inputspans;
                    }

                    // restart from break point
                    input_idx = bp.input_idx;
                    let (ref restart_span_orig, restart_colour, restart_font) = text[input_idx];
                    let restart_span = restart_span_orig
                        .replace('\t', &" ".repeat(TABSIZE))
                        .replace("\r\n", "\n")
                        .replace('\r', "\n");
                    let restart_chars: Vec<char> = restart_span.chars().collect();

                    // skip leading whitespace at start of new line
                    let mut restart_ci = bp.input_char_idx;
                    while restart_ci < restart_chars.len()
                        && restart_chars[restart_ci].is_whitespace()
                        && restart_chars[restart_ci] != '\n'
                    {
                        restart_ci += 1;
                    }

                    // update text queue to reflect consumed content
                    if restart_ci < restart_chars.len() {
                        let remaining: String = restart_chars[restart_ci..].iter().collect();
                        text[input_idx] = (remaining, restart_colour, restart_font);
                    } else {
                        // consumed entire span, move to next
                        input_idx += 1;
                        if input_idx < text.len() {
                            text.drain(..input_idx);
                            input_idx = 0;
                        }
                    }

                    last_break = None;
                    continue 'inputspans;
                } else {
                    // no break point available - force character break (existing behavior)
                    spans.push(current_span.clone());

                    x = start.0 + wrap_offset;
                    y -= line_gap;
                    line_start_x = x;
                    last_break = None;

                    if y < bounding_box.y1 + descent {
                        let remaining: String = span_chars[ci..].iter().collect();
                        text.drain(..=input_idx);
                        if !remaining.is_empty() {
                            text.insert(0, (remaining, colour, font));
                        }
                        break 'inputspans;
                    } else {
                        current_span.text.clear();
                        current_span.text.push(ch);
                        current_span.coords.0 = x;
                        current_span.coords.1 = y;
                        x += hadv;
                        ci += 1;
                        continue 'chars;
                    }
                }
            } else {
                // no overflow - add character and track break points
                current_span.text.push(ch);
                x += hadv;

                // record whitespace as break point (break AFTER the whitespace)
                if ch.is_whitespace() {
                    last_break = Some(BreakPoint {
                        span_idx: spans.len(),
                        char_idx: current_span.text.chars().count(),
                        input_idx,
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
