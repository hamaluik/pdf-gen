use pdf_gen::colours;
use pdf_gen::layout;
use pdf_gen::pagesize;
use pdf_gen::Document;
use pdf_gen::Font;
use pdf_gen::{layout::Margins, Page, SpanFont, SpanLayout};
use pdf_gen::{In, Pt};

fn main() {
    let fira_mono = include_bytes!("../assets/FiraMono-Regular.ttf");
    let fira_mono = Font::load(fira_mono).expect("can load font");

    let mut doc = Document::default();
    let fira_mono_idx = doc.add_font(fira_mono);

    let pages: Vec<&str> = vec!["Page A", "Page B"];
    for pagename in pages.into_iter() {
        let mut page = Page::new(pagesize::A6, Some(Margins::all(In(0.5).into())));

        let start = layout::baseline_start(&page, &doc.fonts[fira_mono_idx], Pt(16.0));
        page.add_span(SpanLayout {
            text: pagename.to_string(),
            font: SpanFont {
                index: fira_mono_idx,
                size: Pt(32.0),
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
