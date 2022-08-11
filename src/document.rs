use crate::{
    font::Font,
    image::Image,
    info::Info,
    page::Page,
    refs::{ObjectReferences, RefType},
};
use pdf_writer::{PdfWriter, Ref};
use std::io::Write;

pub struct Document<'f> {
    pub refs: ObjectReferences,
    pub info: Option<Info>,
    pub pages: Vec<Page>,
    sorted_page_refs: Vec<Ref>,
    pub fonts: Vec<Font<'f>>,
    pub images: Vec<Image>,
}

impl<'f> Document<'f> {
    pub fn new() -> Document<'f> {
        Document {
            refs: ObjectReferences::new(),
            info: None,
            pages: Vec::default(),
            sorted_page_refs: Vec::default(),
            fonts: Vec::default(),
            images: Vec::default(),
        }
    }

    pub fn info(&mut self, info: Info) {
        self.info = Some(info);
    }

    pub fn add_page(&mut self, page: Page) {
        let id = self.refs.gen(RefType::Page(self.pages.len()));
        self.pages.push(page);
        self.sorted_page_refs.push(id);
    }

    pub fn add_font(&mut self, font: Font<'f>) {
        self.refs.gen(RefType::Font(self.fonts.len()));
        self.fonts.push(font);
    }

    pub fn add_image(&mut self, image: Image) {
        self.refs.gen(RefType::Image(self.images.len()));
        self.images.push(image);
    }

    pub fn write<W: Write>(self, mut w: W) -> std::io::Result<()> {
        let Document {
            mut refs,
            info,
            pages,
            sorted_page_refs,
            fonts,
            images,
        } = self;

        let catalog_id = refs.gen(RefType::Catalog);
        let page_tree_id = refs.gen(RefType::PageTree);

        let mut writer = PdfWriter::new();
        if let Some(info) = info {
            info.write(&mut refs, &mut writer);
        }
        writer.catalog(catalog_id).pages(page_tree_id);

        writer
            .pages(page_tree_id)
            .count(sorted_page_refs.len() as i32)
            .kids(sorted_page_refs);

        for (i, font) in fonts.iter().enumerate() {
            font.write(&mut refs, i, &mut writer); // TODO: error handling
        }

        for (i, image) in images.iter().enumerate() {
            image.write(&mut refs, i, &mut writer).unwrap(); // TODO: error handling
        }

        for (i, page) in pages.iter().enumerate() {
            page.write(
                &mut refs,
                i,
                fonts.as_slice(),
                images.as_slice(),
                &mut writer,
            );
        }

        w.write_all(writer.finish().as_slice())
    }
}
