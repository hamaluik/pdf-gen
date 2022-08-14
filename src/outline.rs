use std::{cell::RefCell, rc::Rc};

use pdf_writer::{types::OutlineItemFlags, Finish, PdfWriter, TextStr};

use crate::refs::{ObjectReferences, RefType};

#[derive(Default, Debug)]
pub struct Outline {
    pub entries: Vec<Rc<RefCell<OutlineEntry>>>,
    next_index: usize,
}

#[derive(Debug)]
pub struct OutlineEntry {
    pub index: usize,
    pub page_index: usize,
    pub title: String,
    pub italic: bool,
    pub bold: bool,
    pub parent: Option<Rc<RefCell<OutlineEntry>>>,
    pub children: Vec<Rc<RefCell<OutlineEntry>>>,
}

impl OutlineEntry {
    pub fn bolded(&mut self) -> &mut Self {
        self.bold = true;
        self
    }

    pub fn italicized(&mut self) -> &mut Self {
        self.italic = true;
        self
    }
}

impl Outline {
    pub fn add_bookmark(
        &mut self,
        parent: Option<Rc<RefCell<OutlineEntry>>>,
        page_index: usize,
        title: String,
    ) -> Rc<RefCell<OutlineEntry>> {
        let entry = OutlineEntry {
            index: self.next_index,
            page_index,
            title,
            italic: false,
            bold: false,
            parent: parent.clone(),
            children: Vec::default(),
        };
        self.next_index += 1;
        let entry = Rc::new(RefCell::new(entry));
        if let Some(parent) = parent {
            parent.borrow_mut().children.push(entry.clone());
        } else {
            self.entries.push(entry.clone());
        }
        entry
    }

    pub fn generate_next_index(&mut self) -> usize {
        let ret = self.next_index;
        self.next_index += 1;
        ret
    }

    fn generate_entry_ids(
        &self,
        refs: &mut ObjectReferences,
        entries: &[Rc<RefCell<OutlineEntry>>],
    ) {
        for entry in entries {
            refs.gen(RefType::OutlineEntry(entry.borrow().index));
            self.generate_entry_ids(refs, &entry.borrow().children.as_slice());
        }
    }

    fn write_outline_entries(
        &self,
        entries: &[Rc<RefCell<OutlineEntry>>],
        refs: &mut ObjectReferences,
        writer: &mut PdfWriter,
    ) {
        for (i, entry) in entries.iter().enumerate() {
            self.write_outline_entries(entry.borrow().children.as_slice(), refs, writer);

            let mut item = writer.outline_item(
                refs.get(RefType::OutlineEntry(entry.borrow().index))
                    .unwrap(),
            );

            item.title(TextStr(entry.borrow().title.as_str()));
            item.dest_direct()
                .page(refs.get(RefType::Page(entry.borrow().page_index)).unwrap())
                .fit();

            let mut flags: OutlineItemFlags = OutlineItemFlags::empty();
            flags.set(OutlineItemFlags::BOLD, entry.borrow().bold);
            flags.set(OutlineItemFlags::ITALIC, entry.borrow().italic);
            item.flags(flags);

            if let Some(parent) = &entry.borrow().parent {
                item.parent(
                    refs.get(RefType::OutlineEntry(parent.borrow().index))
                        .unwrap(),
                );
            } else {
                item.parent(refs.get(RefType::Outlines).unwrap());
            }
            if i > 0 {
                item.prev(
                    refs.get(RefType::OutlineEntry(entries[i - 1].borrow().index))
                        .unwrap(),
                );
            }
            if i < entries.len() - 1 {
                item.next(
                    refs.get(RefType::OutlineEntry(entries[i + 1].borrow().index))
                        .unwrap(),
                );
            }
            if !entry.borrow().children.is_empty() {
                item.count(entry.borrow().children.len() as i32 * -1);
                item.first(
                    refs.get(RefType::OutlineEntry(
                        entry.borrow().children.first().unwrap().borrow().index,
                    ))
                    .unwrap(),
                );
                item.last(
                    refs.get(RefType::OutlineEntry(
                        entry.borrow().children.last().unwrap().borrow().index,
                    ))
                    .unwrap(),
                );
            }
        }
    }

    pub(crate) fn write(&self, refs: &mut ObjectReferences, writer: &mut PdfWriter) {
        // generate IDs for everything
        let outlines_id = refs.gen(RefType::Outlines);
        self.generate_entry_ids(refs, self.entries.as_slice());

        // write the root outline
        let mut outline = writer.outline(outlines_id);
        if !self.entries.is_empty() {
            outline.first(
                refs.get(RefType::OutlineEntry(
                    self.entries.first().unwrap().borrow().index,
                ))
                .unwrap(),
            );
            outline.last(
                refs.get(RefType::OutlineEntry(
                    self.entries.last().unwrap().borrow().index,
                ))
                .unwrap(),
            );
        }
        outline.finish();

        self.write_outline_entries(self.entries.as_slice(), refs, writer);
    }
}
