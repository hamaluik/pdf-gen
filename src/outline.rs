use pdf_writer::{Finish, Name, Null, PdfWriter, TextStr};

use crate::refs::{ObjectReferences, RefType};

#[derive(Default, Debug)]
pub struct Outline {
    pub entries: Vec<OutlineEntry>,
    next_index: usize,
}

#[derive(Debug)]
pub struct OutlineEntry {
    pub index: usize,
    pub page_index: usize,
    pub title: String,
}

impl Outline {
    pub fn add_bookmark(&mut self, page_index: usize, title: String) {
        let entry = OutlineEntry {
            index: self.next_index,
            page_index,
            title,
        };
        self.next_index += 1;
        self.entries.push(entry);
    }

    pub(crate) fn write(&self, refs: &mut ObjectReferences, writer: &mut PdfWriter) {
        // generate IDs for everything
        let outlines_id = refs.gen(RefType::Outlines);
        for entry in self.entries.iter() {
            refs.gen(RefType::OutlineEntry(entry.index));
        }

        // write the root outline
        let mut outline = writer.outline(outlines_id);
        if !self.entries.is_empty() {
            outline.first(
                refs.get(RefType::OutlineEntry(self.entries.first().unwrap().index))
                    .unwrap(),
            );
            outline.last(
                refs.get(RefType::OutlineEntry(self.entries.last().unwrap().index))
                    .unwrap(),
            );
        }
        outline.finish();

        // write all our items
        for (i, entry) in self.entries.iter().enumerate() {
            let mut item =
                writer.outline_item(refs.get(RefType::OutlineEntry(entry.index)).unwrap());
            item.parent(refs.get(RefType::Outlines).unwrap());
            item.title(TextStr(entry.title.as_str()));
            if i > 0 {
                item.prev(
                    refs.get(RefType::OutlineEntry(self.entries[i - 1].index))
                        .unwrap(),
                );
            }
            if i < self.entries.len() - 1 {
                item.next(
                    refs.get(RefType::OutlineEntry(self.entries[i + 1].index))
                        .unwrap(),
                );
            }
            item.dest_direct()
                .page(refs.get(RefType::Page(entry.page_index)).unwrap())
                .item(Name(b"FitH"))
                .item(Null);
        }
    }
}
