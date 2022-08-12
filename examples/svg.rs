use pdf_gen::Document;
use pdf_gen::Image;
use pdf_gen::Info;
use pdf_gen::Pt;
use pdf_gen::Rect;
use pdf_gen::{ImageLayout, Page};

fn main() {
    let mut doc = Document::default();
    doc.set_info(
        Info::new()
            .title("SVG Test")
            .author("Kenton Hamaluik")
            .subject("Development Test / Example")
            .clone(),
    );

    let pagesize = pdf_gen::pagesize::LETTER;

    let image = Image::new_from_disk("./assets/tiger.svg").unwrap();
    let (w, h) = (Pt(image.width / 2.0), Pt(image.height / 2.0));
    let x = (pagesize.0 - w) / 2.0;
    let y = (pagesize.1 - h) / 2.0;
    doc.add_image(image);
    let mut page = Page::new(pagesize, None);
    page.add_image(ImageLayout {
        image_index: 0,
        position: Rect {
            x1: x,
            y1: y,
            x2: x + w,
            y2: y + h,
        },
    });
    doc.add_page(page);

    let mut out = std::fs::File::create("svg.pdf").unwrap();
    doc.write(&mut out).unwrap();
}
