use automerge::{AutoCommit, ObjType, ReadDoc, transaction::Transactable};

/// A conflict-free replicated data type (CRDT) wrapper for managing grocery list items.
///
/// `ListDoc` provides a high-level interface to Automerge for storing and manipulating
/// a list of grocery items with automatic conflict resolution. All changes are persisted
/// to a CRDT blob that can be synchronized across devices.
///
/// The underlying data structure is an Automerge list containing string values where
/// items can be prefixed with `[x] ` to indicate completion/toggling.
pub struct ListDoc {
    doc: AutoCommit,
}

impl ListDoc {
    /// Creates a new, empty grocery list document.
    ///
    /// # Example
    /// ```
    /// let list = ListDoc::new();
    /// ```
    pub fn new() -> Self {
        Self {
            doc: AutoCommit::new(),
        }
    }

    /// Adds a new item to the beginning of the list.
    ///
    /// Automatically creates the underlying list structure if it doesn't exist.
    ///
    /// # Arguments
    /// * `name` - The name of the item to add
    ///
    /// # Example
    /// ```
    /// let mut list = ListDoc::new();
    /// list.add_item("milk");
    /// ```
    pub fn add_item(&mut self, name: &str) {
        let items = self
            .doc
            .get(automerge::ROOT, "items")
            .ok()
            .flatten()
            .map(|(_, id)| id)
            .unwrap_or_else(|| {
                self.doc
                    .put_object(automerge::ROOT, "items", ObjType::List)
                    .unwrap()
            });

        self.doc.insert(&items, 0, name).unwrap();
    }

    /// Adds a new item at the specified index in the list.
    ///
    /// Automatically creates the underlying list structure if it doesn't exist.
    ///
    /// # Arguments
    /// * `index` - The position to insert at (0-based)
    /// * `name` - The name of the item to add
    ///
    /// # Example
    /// ```
    /// let mut list = ListDoc::new();
    /// list.add_item_at(0, "eggs");
    /// ```
    pub fn add_item_at(&mut self, index: usize, name: &str) {
        let items = self
            .doc
            .get(automerge::ROOT, "items")
            .ok()
            .flatten()
            .map(|(_, id)| id)
            .unwrap_or_else(|| {
                self.doc
                    .put_object(automerge::ROOT, "items", ObjType::List)
                    .unwrap()
            });

        self.doc.insert(&items, index, name).unwrap();
    }

    /// Retrieves all items in the list.
    ///
    /// Returns items in their current order, including both checked and unchecked items.
    ///
    /// # Returns
    /// A vector of item names. Empty if no items exist.
    ///
    /// # Example
    /// ```
    /// let list = ListDoc::new();
    /// let items = list.get_items();
    /// ```
    pub fn get_items(&self) -> Vec<String> {
        let items = self.doc.get(automerge::ROOT, "items").ok().flatten();

        if let Some((_, obj_id)) = items {
            let len = self.doc.length(&obj_id);
            let mut result = Vec::new();
            for i in 0..len {
                if let Some((value, _)) = self.doc.get(&obj_id, i).ok().flatten() {
                    if let automerge::Value::Scalar(scalar) = value {
                        if let automerge::ScalarValue::Str(s) = scalar.as_ref() {
                            result.push(s.to_string());
                        }
                    }
                }
            }
            result
        } else {
            Vec::new()
        }
    }

    /// Toggles the completion status of an item.
    ///
    /// Items are marked as completed by prefixing with `[x] `. Calling this method
    /// will add or remove that prefix to toggle the state.
    ///
    /// # Arguments
    /// * `index` - The position of the item to toggle (0-based)
    ///
    /// # Example
    /// ```
    /// let mut list = ListDoc::new();
    /// list.add_item("milk");
    /// list.toggle_item(0); // Marks as completed: "[x] milk"
    /// list.toggle_item(0); // Marks as uncompleted: "milk"
    /// ```
    pub fn toggle_item(&mut self, index: usize) {
        let items = self
            .doc
            .get(automerge::ROOT, "items")
            .ok()
            .flatten()
            .unwrap();

        if let Some((value, _)) = self.doc.get(&items.1, index).ok().flatten() {
            if let automerge::Value::Scalar(scalar) = value {
                if let automerge::ScalarValue::Str(s) = scalar.as_ref() {
                    let new_value = if s.starts_with("[x] ") {
                        s.trim_start_matches("[x] ").to_string()
                    } else {
                        format!("[x] {}", s)
                    };
                    self.doc.put(&items.1, index, new_value).unwrap();
                }
            }
        }
    }

    /// Updates the name of an item at the specified index.
    ///
    /// # Arguments
    /// * `index` - The position of the item to update (0-based)
    /// * `new_name` - The new name for the item
    ///
    /// # Example
    /// ```
    /// let mut list = ListDoc::new();
    /// list.add_item("milk");
    /// list.update_item(0, "whole milk");
    /// ```
    pub fn update_item(&mut self, index: usize, new_name: &str) {
        let items = self
            .doc
            .get(automerge::ROOT, "items")
            .ok()
            .flatten()
            .unwrap();

        self.doc.put(&items.1, index, new_name).unwrap();
    }

    /// Removes an item from the list.
    ///
    /// # Arguments
    /// * `index` - The position of the item to remove (0-based)
    ///
    /// # Example
    /// ```
    /// let mut list = ListDoc::new();
    /// list.add_item("milk");
    /// list.remove_item(0);
    /// ```
    pub fn remove_item(&mut self, index: usize) {
        let items = self
            .doc
            .get(automerge::ROOT, "items")
            .ok()
            .flatten()
            .unwrap();

        self.doc.delete(&items.1, index).unwrap();
    }

    /// Serializes the list to a byte blob for persistence or synchronization.
    ///
    /// The returned bytes contain the complete CRDT state and can be loaded
    /// back into a new document with [`load`](Self::load).
    ///
    /// # Returns
    /// A vector of bytes representing the serialized document state.
    ///
    /// # Example
    /// ```
    /// let mut list = ListDoc::new();
    /// list.add_item("milk");
    /// let bytes = list.save();
    /// ```
    pub fn save(&mut self) -> Vec<u8> {
        self.doc.save()
    }

    pub fn load(bytes: &[u8]) -> Self {
        let doc = AutoCommit::load(bytes).unwrap();
        Self { doc }
    }
}
