//! Shared content rendering logic for pages and form XObjects.

use crate::colour::Colour;
use crate::font::Font;
use crate::page::{PageContents, SpanFont, SpanLayout};
use id_arena::Arena;
use std::io::Write;

/// Renders page contents to a PDF content stream.
///
/// This is the shared implementation used by both `Page::render()` and
/// `FormXObject::render()` to convert high-level content items into
/// low-level PDF operators.
#[allow(clippy::write_with_newline)]
pub(crate) fn render_contents(
    contents: &[PageContents],
    fonts: &Arena<Font>,
) -> Result<Vec<u8>, std::io::Error> {
    if contents.is_empty() {
        return Ok(Vec::default());
    }

    let mut content: Vec<u8> = Vec::default();

    for page_content in contents.iter() {
        match page_content {
            PageContents::Text(spans) => {
                render_text_spans(&mut content, spans, fonts)?;
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

#[allow(clippy::write_with_newline)]
fn render_text_spans(
    content: &mut Vec<u8>,
    spans: &[SpanLayout],
    fonts: &Arena<Font>,
) -> Result<(), std::io::Error> {
    if spans.is_empty() {
        return Ok(());
    }

    write!(content, "q\n")?;

    // unwrap is safe, as we know spans isn't empty
    let mut current_font: SpanFont = spans.first().unwrap().font;
    let mut current_colour: Colour = spans.first().unwrap().colour;

    write!(
        content,
        "/F{} {} Tf\n",
        current_font.id.index(),
        current_font.size
    )?;
    write_colour(content, current_colour)?;

    for span in spans.iter() {
        if span.font != current_font {
            current_font = span.font;
            write!(
                content,
                "/F{} {} Tf\n",
                current_font.id.index(),
                current_font.size
            )?;
        }
        if span.colour != current_colour {
            current_colour = span.colour;
            write_colour(content, current_colour)?;
        }

        write!(content, "BT\n")?;
        write!(content, "{} {} Td\n", span.coords.0, span.coords.1)?;
        write!(content, "<")?;
        for ch in span.text.chars() {
            write!(
                content,
                "{:04x}",
                fonts[current_font.id].glyph_id(ch).unwrap_or_else(|| fonts[current_font.id]
                    .replacement_glyph_id()
                    .unwrap_or_else(|| fonts[current_font.id]
                        .glyph_id('?')
                        .expect("font has '?' glyph")))
            )?;
        }
        write!(content, "> Tj\n")?;
        write!(content, "ET\n")?;
    }

    write!(content, "Q\n")?;
    Ok(())
}

#[allow(clippy::write_with_newline)]
fn write_colour(content: &mut Vec<u8>, colour: Colour) -> Result<(), std::io::Error> {
    match colour {
        Colour::RGB { r, g, b } => write!(content, "{r} {g} {b} rg\n"),
        Colour::CMYK { c, m, y, k } => write!(content, "{c} {m} {y} {k} k\n"),
        Colour::Grey { g } => write!(content, "{g} g\n"),
    }
}
