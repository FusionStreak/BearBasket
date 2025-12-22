use automerge::{AutoCommit, ObjType, ReadDoc, transaction::Transactable};

pub struct ListDoc {
    doc: AutoCommit,
}

impl ListDoc {
    pub fn new() -> Self {
        Self {
            doc: AutoCommit::new(),
        }
    }

    pub fn add_item(&mut self, name: &str) {
        let items = self
            .doc
            .get(automerge::ROOT, "items")
            .ok()
            .flatten()
            .unwrap_or_else(|| {
                self.doc
                    .put_object(automerge::ROOT, "items", ObjType::List)
                    .unwrap()
            });

        self.doc.insert(&items, 0, name).unwrap();
    }

    pub fn save(&self) -> Vec<u8> {
        self.doc.save()
    }

    pub fn load(bytes: &[u8]) -> Self {
        let doc = AutoCommit::load(bytes).unwrap();
        Self { doc }
    }
}
