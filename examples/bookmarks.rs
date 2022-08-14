use pdf_gen::colours;
use pdf_gen::layout;
use pdf_gen::pagesize;
use pdf_gen::Document;
use pdf_gen::Font;
use pdf_gen::Rect;
use pdf_gen::{layout::Margins, Page, SpanFont, SpanLayout};
use pdf_gen::{In, Pt};

fn main() {
    let fira_mono = include_bytes!("../assets/FiraMono-Regular.ttf");
    let fira_mono = Font::load(fira_mono).expect("can load font");

    let mut doc = Document::default();
    let fira_mono_idx = doc.add_font(fira_mono);

    let pages: Vec<&str> = vec!["Page A", "Page B"];
    for (pi, pagename) in pages.into_iter().enumerate() {
        let mut page = Page::new(pagesize::A6, Some(Margins::all(In(0.5).into())));

        let start = layout::baseline_start(&page, &doc.fonts[fira_mono_idx], Pt(24.0));
        page.add_span(SpanLayout {
            text: pagename.to_string(),
            font: SpanFont {
                index: fira_mono_idx,
                size: Pt(24.0),
            },
            colour: colours::BLACK,
            coords: start,
        });

        let start = (
            start.0,
            start.1 - doc.fonts[fira_mono_idx].line_height(Pt(24.0)),
        );
        let link_label = format!("Link to page {}", (1 - pi) + 1);
        page.add_intradocument_link(
            Rect {
                x1: start.0,
                y1: start.1,
                x2: start.0
                    + layout::width_of_text(&link_label, &doc.fonts[fira_mono_idx], Pt(24.0)),
                y2: start.1 + doc.fonts[fira_mono_idx].ascent(Pt(24.0)),
            },
            1 - pi,
        );
        page.add_span(SpanLayout {
            text: link_label,
            font: SpanFont {
                index: fira_mono_idx,
                size: Pt(24.0),
            },
            colour: colours::BLACK,
            coords: start,
        });

        let page_index = doc.add_page(page);
        doc.add_bookmark(pagename, page_index);
    }

    // we're going to save the contents to a file on disk, but anywhere where we can write would do
    let mut out = std::fs::File::create("bookmarks.pdf").unwrap();

    // render the document!
    doc.write(&mut out).unwrap();
}
