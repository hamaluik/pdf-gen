use pdf_gen::pagesize;
use pdf_gen::pagesize::PageOrientation;
use pdf_gen::Document;
use pdf_gen::In;
use pdf_gen::Page;

fn main() {
    let mut doc = Document::default();
    for _ in 0..2 {
        let mut page = Page::new(
            pagesize::LETTER.landscape(),
            Some(pdf_gen::layout::Margins::all(In(0.125))),
        );

        let content = {
            let mut content = pdf_gen::pdf_writer_crate::Content::new();

            let mut x = page.content_box.x1;
            let y0 = page.content_box.y1;
            let y1 = page.content_box.y2;
            while x <= page.content_box.x2 {
                content.move_to(x.into(), y0.into());
                content.line_to(x.into(), y1.into());
                x += In(0.125).into();
            }

            let mut y = y0;
            let x0 = page.content_box.x1;
            let x1 = page.content_box.x2;
            while y <= y1 {
                content.move_to(x0.into(), y.into());
                content.line_to(x1.into(), y.into());
                y += In(0.125).into();
            }

            content.set_stroke_cmyk(0.25, 0.0, 0.0, 0.0);
            content.set_line_width(0.5);
            content.stroke();

            content
        };
        page.add_content(content);
        doc.add_page(page);
    }

    let mut out = std::fs::File::create("graph-paper.pdf").unwrap();
    doc.write(&mut out).unwrap();
}
