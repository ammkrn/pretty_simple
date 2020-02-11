use crate::doc::Doc;

pub const MAX_PRIORITY : usize = 1024;

#[derive(Debug, Clone)]
pub struct Parenable {
    pub doc : Doc,
    pub priority : usize,
}

impl Parenable {
    pub fn new(doc : Doc, priority : usize) -> Self {
        Parenable {
            priority,
            doc
        }
    }

    pub fn new_max(doc : Doc) -> Self {
        Parenable {
            priority : MAX_PRIORITY,
            doc
        }
    }

    pub fn maybe_surround(&self, target_priority : usize) -> Doc {
        // If the given `Parenable`'s priority is less
        // than some given priority, surround with
        // parenthesis.
        if self.priority < target_priority {
            Doc::from("(").concat(self.doc.clone()).concat(")")
        } else {
            self.doc.clone()
        }
    }
}