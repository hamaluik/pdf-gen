use pdf_writer::Ref;
use std::collections::HashMap;

#[derive(Eq, PartialEq, Hash, Copy, Clone, Debug)]
pub(crate) enum RefType {
    Catalog,
    Info,
    PageTree,
    Page(usize),
    Font(usize),
    ContentForPage(usize),
    CidFont(usize),
    ToUnicode(usize),
    FontDescriptor(usize),
    FontData(usize),
    Image(usize),
    ImageMask(usize),
    Outlines,
    OutlineEntry(usize),
}

pub(crate) struct ObjectReferences {
    refs: HashMap<RefType, Ref>,
    next_id: i32,
}

impl ObjectReferences {
    pub(crate) fn new() -> ObjectReferences {
        ObjectReferences {
            refs: HashMap::new(),
            next_id: 3,
        }
    }

    fn new_id(&mut self) -> Ref {
        let id = self.next_id;
        self.next_id += 1;
        Ref::new(id)
    }

    /// Warning: only do if you're sure you know what you're doing!
    pub(crate) fn set_next_id(&mut self, id: Ref) {
        self.next_id = id.get();
    }

    pub(crate) fn get(&self, ref_type: RefType) -> Option<Ref> {
        self.refs.get(&ref_type).map(Clone::clone)
    }

    pub(crate) fn gen(&mut self, ref_type: RefType) -> Ref {
        let id = self.new_id();
        self.refs.insert(ref_type, id);
        id
    }
}
