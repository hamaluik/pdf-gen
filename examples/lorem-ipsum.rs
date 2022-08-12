use pdf_gen::colour::Colour;
use pdf_gen::document::Document;
use pdf_gen::font::Font;
use pdf_gen::info::Info;
use pdf_gen::layout;
use pdf_gen::page::{Margins, Page, SpanFont, SpanLayout};
use pdf_gen::units::*;

fn main() {
    let fira_mono = include_bytes!("../assets/FiraMono-Regular.ttf");
    let fira_mono = Font::load(fira_mono).expect("can load font");

    let mut doc = Document::new();
    doc.add_font(fira_mono);
    doc.set_info(
        Info::new()
            .title("Lorem Ipsum Test")
            .author("Kenton Hamaluik")
            .subject("Development Test / Example")
            .clone(),
    );

    let mut text: Vec<(String, Colour)> = vec![
        (
            format!("{}\n{}\n", lipsum::lipsum(3), lipsum::lipsum(4)),
            Colour {
                r: 0.0,
                g: 0.0,
                b: 0.0,
            },
        ),
        (
            lipsum::lipsum(256),
            Colour {
                r: 0.25,
                g: 0.25,
                b: 0.25,
            },
        ),
    ];

    let mut page_index = 0;
    while !text.is_empty() {
        // add a 0.5in gutter
        let mut margins = Margins::all(In(0.5).into());
        if page_index % 2 == 0 {
            margins.left += In(0.5).into();
        } else {
            margins.right += In(0.5).into();
        }

        let page_size = pdf_gen::page::pagesize::HALF_LETTER;
        let mut page = Page::new(page_size, margins);
        let start = layout::baseline_start(&page, &doc.fonts[0], Pt(16.0));
        let bbox = page.content_box.clone();
        layout::layout_text(&doc, &mut page, start, 0, Pt(16.0), &mut text, bbox);

        // add a page number!
        let page_number_text = format!("Page {}", page_index + 1);
        let px = if page_index % 2 == 0 {
            page.content_box.x2 - layout::width_of_text(&page_number_text, &doc.fonts[0], Pt(10.0))
        } else {
            page.content_box.x1
        };
        page.add_span(SpanLayout {
            text: page_number_text,
            font: SpanFont {
                index: 0,
                size: Pt(10.0),
            },
            colour: Colour {
                r: 0.5,
                g: 0.5,
                b: 0.5,
            },
            coords: (px, In(0.25).into()),
        });

        doc.add_page(page);
        page_index += 1;
    }

    let mut out = std::fs::File::create("lorem-ipsum.pdf").unwrap();
    doc.write(&mut out).unwrap();
}
