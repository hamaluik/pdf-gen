use crate::{
    font::Font,
    image::Image,
    info::Info,
    outline::Outline,
    page::Page,
    refs::{ObjectReferences, RefType},
    OutlineEntry, PDFError,
};
use id_arena::{Arena, Id};
use pdf_writer::{Finish, PdfWriter, Ref};
use std::{cell::RefCell, io::Write, rc::Rc};

#[derive(Default)]
/// A document is the main object that stores all the contents of the PDF
/// then renders it out with a call to [Document::write]
pub struct Document {
    pub info: Option<Info>,
    pub pages: Arena<Page>,
    pub page_order: Vec<Id<Page>>,
    pub fonts: Arena<Font>,
    pub images: Arena<Image>,
    pub outline: Outline,
}

impl Document {
    /// Sets information about the document. If not provided, no information block will be
    /// written to the PDF
    pub fn set_info(&mut self, info: Info) {
        self.info = Some(info);
    }

    /// Add a page to the document, returning the index of that page within the document.
    /// This index can be used to refer to the page if needed, provided that you don't
    /// remove or reorder the pages in the document. The page will be added to the end
    /// of the document.
    pub fn add_page(&mut self, page: Page) -> Id<Page> {
        let id = self.pages.alloc(page);
        self.page_order.push(id);
        id
    }

    /// Add a page to the document, inserting it before the page identified by `next`.
    /// If there is no page identified by `next`, the page will be added to the end of
    /// the document.
    pub fn insert_page_before_id(&mut self, page: Page, next: Id<Page>) -> Id<Page> {
        let id = self.pages.alloc(page);
        if let Some(index) = self.index_of_page(next) {
            if index > self.page_order.len() {
                self.page_order.push(id);
            } else {
                self.page_order.insert(index, id);
            }
        } else {
            self.page_order.push(id);
        }
        id
    }

    /// Add a page to the document, inserting it after the page identified by `previous`.
    /// If there is no page identified by `previous`, the page will be added to the end
    /// of the document.
    pub fn insert_page_after_id(&mut self, page: Page, previous: Id<Page>) -> Id<Page> {
        let id = self.pages.alloc(page);
        if let Some(index) = self.index_of_page(previous) {
            let index = index + 1;
            if index > self.page_order.len() {
                self.page_order.push(id);
            } else {
                self.page_order.insert(index, id);
            }
        } else {
            self.page_order.push(id);
        }
        id
    }

    /// Get the 0-based index of a page given its ID. Note that changing the page order
    /// after this call _will_ invalidate the returned page index
    pub fn index_of_page(&self, page: Id<Page>) -> Option<usize> {
        self.page_order
            .iter()
            .enumerate()
            .find(|&(_, p)| *p == page)
            .map(|(i, _)| i)
    }

    /// Get the page Id of a page at the given index. Returns [None] if
    /// `page_index >= self.page_order.len()`.
    pub fn id_of_page_index(&self, page_index: usize) -> Option<Id<Page>> {
        self.page_order.get(page_index).map(|i| *i)
    }

    /// Add a font to the document structure. Note that fonts are stored "globally" within
    /// the document, such that any page can access it by referring to it by its index /
    /// reference. The returned value is the index of the font, which is valid so long as
    /// you don't ever remove or reorder fonts from / in the document.
    pub fn add_font(&mut self, font: Font) -> Id<Font> {
        self.fonts.alloc(font)
    }

    /// Add an image to the document structure. Note that images are stored "globally"
    /// within the document, such that any page can access and re-use images by referring
    /// to it by its its / reference. The returned value is the index of the image, which
    /// is valid so long as you don't ever remove or reorder images from / in the document.
    pub fn add_image(&mut self, image: Image) -> Id<Image> {
        self.images.alloc(image)
    }

    /// Add a bookmark in the document outline pointing to a page with a given index. For now,
    /// this will always fit the entire page into view when navigating to the bookmark.
    pub fn add_bookmark<S: ToString>(
        &mut self,
        parent: Option<Rc<RefCell<OutlineEntry>>>,
        title: S,
        page_index: usize,
    ) -> Rc<RefCell<OutlineEntry>> {
        self.outline
            .add_bookmark(parent, page_index, title.to_string())
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
            page_order,
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

        // generate page refs keyed by page_order index (not arena index) so that
        // bookmarks and links can reference pages by their position in the document
        let page_refs: Vec<Ref> = page_order
            .iter()
            .enumerate()
            .map(|(i, _id)| refs.gen(RefType::Page(i)))
            .collect();

        writer
            .pages(page_tree_id)
            //.count(sorted_page_refs.len() as i32)
            .count(page_refs.len() as i32)
            .kids(page_refs);

        for (i, font) in fonts.iter() {
            font.write(&mut refs, i, &mut writer);
        }

        for (i, image) in images.iter() {
            image.write(&mut refs, i.index(), &mut writer)?;
        }

        for (page_index, id) in page_order.iter().enumerate() {
            let page = pages.get(*id).ok_or(PDFError::PageMissing)?;
            page.write(
                &mut refs,
                page_index,
                &page_order,
                &fonts,
                &images,
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
