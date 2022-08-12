use pdf_gen::document::Document;
use pdf_gen::image::Image;
use pdf_gen::info::Info;
use pdf_gen::page::{ImageLayout, Margins, Page};
use pdf_gen::rect::Rect;
use pdf_gen::units::*;

fn main() {
    let mut doc = Document::new();
    doc.set_info(
        Info::new()
            .title("Image Test")
            .author("Kenton Hamaluik")
            .subject("Development Test / Example")
            .clone(),
    );

    let pagesize = pdf_gen::page::pagesize::LETTER;

    let image = Image::new_from_disk("./assets/image.jpg").unwrap();
    let (w, h) = (Pt(image.width / 2.0), Pt(image.height / 2.0));
    let x = (pagesize.0 - w) / 2.0;
    let y = (pagesize.1 - h) / 2.0;
    doc.add_image(image);
    let mut page = Page::new(pagesize, Margins::all(In(0.5).into()));
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

    let mut out = std::fs::File::create("image.pdf").unwrap();
    doc.write(&mut out).unwrap();
}
