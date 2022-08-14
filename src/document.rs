use crate::{
    font::Font,
    image::Image,
    info::Info,
    outline::Outline,
    page::Page,
    refs::{ObjectReferences, RefType},
    OutlineEntry, PDFError,
};
use pdf_writer::{Finish, PdfWriter, Ref};
use std::io::Write;

#[derive(Default)]
/// A document is the main object that stores all the contents of the PDF
/// then renders it out with a call to [Document::write]
pub struct Document<'f> {
    pub info: Option<Info>,
    pub pages: Vec<Page>,
    //sorted_page_refs: Vec<Ref>,
    pub fonts: Vec<Font<'f>>,
    pub images: Vec<Image>,
    pub outline: Outline,
}

impl<'f> Document<'f> {
    /// Sets information about the document. If not provided, no information block will be
    /// written to the PDF
    pub fn set_info(&mut self, info: Info) {
        self.info = Some(info);
    }

    /// Add a page to the document, returning the index of that page within the document.
    /// This index can be used to refer to the page if needed, provided that you don't
    /// remove or reorder the pages in the document.
    pub fn add_page(&mut self, page: Page) -> usize {
        self.pages.push(page);
        self.pages.len() - 1
    }

    /// Add a font to the document structure. Note that fonts are stored "globally" within
    /// the document, such that any page can access it by referring to it by its index /
    /// reference. The returned value is the index of the font, which is valid so long as
    /// you don't ever remove or reorder fonts from / in the document.
    pub fn add_font(&mut self, font: Font<'f>) -> usize {
        self.fonts.push(font);
        self.fonts.len() - 1
    }

    /// Add an image to the document structure. Note that images are stored "globally"
    /// within the document, such that any page can access and re-use images by referring
    /// to it by its its / reference. The returned value is the index of the image, which
    /// is valid so long as you don't ever remove or reorder images from / in the document.
    pub fn add_image(&mut self, image: Image) -> usize {
        self.images.push(image);
        self.images.len() - 1
    }

    /// Add a bookmark in the document outline pointing to a page with a given index. For now,
    /// this will always fit the entire page into view when navigating to the bookmark.
    pub fn add_bookmark<S: ToString>(&mut self, title: S, page_index: usize) -> &mut OutlineEntry {
        self.outline.add_bookmark(page_index, title.to_string())
    }

    /// Write the entire document to the writer. Note: although this can write to arbitrary
    /// streams, the entire document is "rendered" in memory first. If you have a very large
    /// document, this could allocate a significant amount of memory. This limitation is due
    /// to the underlying pdf-writer implementation, which may be removed in the future.
    ///
    /// Until `write` is called, all references are un-resolved, so pages, fonts, images, etc
    /// can be added / edited / reordered / removed as you like, provided you keep track of
    /// references in your page contents yourself (i.e., if you have 2 fonts and decided to
    /// change the order of them before writing, then you should update all font_index
    /// references on all pages to reflect the change). Calling `write` will automatically
    /// generate PDF objects and corresponding references to those objects.
    pub fn write<W: Write>(self, mut w: W) -> Result<(), PDFError> {
        let Document {
            info,
            pages,
            //sorted_page_refs,
            fonts,
            images,
            outline,
        } = self;

        let mut refs = ObjectReferences::new();

        let catalog_id = refs.gen(RefType::Catalog);
        let page_tree_id = refs.gen(RefType::PageTree);

        let mut writer = PdfWriter::new();
        if let Some(info) = info {
            info.write(&mut refs, &mut writer);
        }

        let page_refs: Vec<Ref> = pages
            .iter()
            .enumerate()
            .map(|(i, _)| refs.gen(RefType::Page(i)))
            .collect();

        writer
            .pages(page_tree_id)
            //.count(sorted_page_refs.len() as i32)
            .count(pages.len() as i32)
            .kids(page_refs);

        for (i, font) in fonts.iter().enumerate() {
            font.write(&mut refs, i, &mut writer);
        }

        for (i, image) in images.iter().enumerate() {
            image.write(&mut refs, i, &mut writer)?;
        }

        for (i, page) in pages.iter().enumerate() {
            page.write(
                &mut refs,
                i,
                fonts.as_slice(),
                images.as_slice(),
                &mut writer,
            )?;
        }

        outline.write(&mut refs, &mut writer);

        let mut catalog = writer.catalog(catalog_id);
        catalog.pages(page_tree_id);
        catalog.outlines(refs.get(RefType::Outlines).unwrap());
        catalog.finish();

        w.write_all(writer.finish().as_slice()).map_err(Into::into)
    }
}
