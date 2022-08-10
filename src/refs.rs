use pdf_writer::Ref;
use std::collections::HashMap;

#[derive(Eq, PartialEq, Hash, Copy, Clone, Debug)]
pub enum RefType {
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
}

pub struct ObjectReferences {
    refs: HashMap<RefType, Ref>,
    next_id: i32,
}

impl ObjectReferences {
    pub fn new() -> ObjectReferences {
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

    pub fn get(&self, ref_type: RefType) -> Option<Ref> {
        self.refs.get(&ref_type).map(Clone::clone)
    }

    pub fn gen(&mut self, ref_type: RefType) -> Ref {
        let id = self.new_id();
        self.refs.insert(ref_type, id.clone());
        id
    }

    pub fn iter(&self) -> std::collections::hash_map::Iter<'_, RefType, Ref> {
        self.refs.iter()
    }
}
