use pdf_gen::layout;
use pdf_gen::pagesize;
use pdf_gen::Document;
use pdf_gen::Font;
use pdf_gen::{layout::Margins, Page};
use pdf_gen::{In, Pt};

fn main() {
    // load a font to embed and use
    let font = include_bytes!("../assets/CrimsonPro-Regular.ttf");
    let font = Font::load(font.to_vec()).expect("can load font");

    let mut doc = Document::default();
    let font = doc.add_font(font);

    let mut page = Page::new(pagesize::LETTER, Some(Margins::all(In(0.5))));
    let bbox = page.content_box.clone();
    layout::layout_text_spring(&doc, &mut page, font, Pt(16.), &lipsum::lipsum(200), bbox);

    doc.add_page(page);
    let mut out = std::fs::File::create("text-layout.pdf").unwrap();
    doc.write(&mut out).unwrap();
}
