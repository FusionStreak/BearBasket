use automerge::{AutoCommit, ObjType, ReadDoc, transaction::Transactable};
use uuid::Uuid;

use crate::error::{CoreError, Result};
use crate::model::Item;

/// A conflict-free replicated data type (CRDT) wrapper for managing grocery list items.
///
/// `ListDoc` provides a high-level interface to Automerge for storing and manipulating
/// a list of grocery items with automatic conflict resolution. All changes are persisted
/// to a CRDT blob that can be synchronized across devices.
///
/// Items are stored as JSON-serialized objects in an Automerge list, allowing for
/// rich item data including name, quantity, category, notes, and checked state.
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
        let mut doc = AutoCommit::new();
        // Initialize the items list
        doc.put_object(automerge::ROOT, "items", ObjType::List)
            .unwrap();
        Self { doc }
    }

    /// Gets or creates the items list object.
    fn get_or_create_items(&mut self) -> automerge::ObjId {
        self.doc
            .get(automerge::ROOT, "items")
            .ok()
            .flatten()
            .map(|(_, id)| id)
            .unwrap_or_else(|| {
                self.doc
                    .put_object(automerge::ROOT, "items", ObjType::List)
                    .unwrap()
            })
    }

    /// Gets the items list object for reading.
    fn get_items(&self) -> Option<automerge::ObjId> {
        self.doc
            .get(automerge::ROOT, "items")
            .ok()
            .flatten()
            .map(|(_, id)| id)
    }

    /// Adds a new item to the beginning of the list.
    ///
    /// # Arguments
    /// * `name` - The name of the item to add
    /// * `quantity` - Optional quantity
    /// * `category` - Optional category for grouping
    /// * `notes` - Optional notes
    ///
    /// # Returns
    /// The created Item with its generated ID.
    ///
    /// # Example
    /// ```
    /// let mut list = ListDoc::new();
    /// let item = list.add_item("milk", Some(2), Some("Dairy"), None)?;
    /// ```
    pub fn add_item(
        &mut self,
        name: &str,
        quantity: Option<u32>,
        category: Option<String>,
        notes: Option<String>,
    ) -> Result<Item> {
        self.add_item_at(0, name, quantity, category, notes)
    }

    /// Adds a new item at the specified index in the list.
    ///
    /// # Arguments
    /// * `index` - The position to insert at (0-based)
    /// * `name` - The name of the item to add
    /// * `quantity` - Optional quantity
    /// * `category` - Optional category for grouping
    /// * `notes` - Optional notes
    ///
    /// # Returns
    /// The created Item with its generated ID.
    ///
    /// # Example
    /// ```
    /// let mut list = ListDoc::new();
    /// let item = list.add_item_at(0, "eggs", Some(12), None, None)?;
    /// ```
    pub fn add_item_at(
        &mut self,
        index: usize,
        name: &str,
        quantity: Option<u32>,
        category: Option<String>,
        notes: Option<String>,
    ) -> Result<Item> {
        let items = self.get_or_create_items();
        let id = Uuid::new_v4().to_string();

        let item = Item {
            id: id.clone(),
            name: name.to_string(),
            checked: false,
            quantity,
            category,
            notes,
        };

        let json =
            serde_json::to_string(&item).map_err(|e| CoreError::Serialization(e.to_string()))?;

        self.doc
            .insert(&items, index, json)
            .map_err(|e| CoreError::Crdt(e.to_string()))?;

        Ok(item)
    }

    /// Retrieves all items in the list.
    ///
    /// Returns items in their current order, including both checked and unchecked items.
    ///
    /// # Returns
    /// A vector of Items. Empty if no items exist.
    ///
    /// # Example
    /// ```
    /// let list = ListDoc::new();
    /// let items = list.get_all_items();
    /// ```
    pub fn get_all_items(&self) -> Vec<Item> {
        let Some(items_id) = self.get_items() else {
            return Vec::new();
        };

        let len = self.doc.length(&items_id);
        let mut result = Vec::with_capacity(len);

        for i in 0..len {
            if let Some(item) = self.get_item_at_index(&items_id, i) {
                result.push(item);
            }
        }

        result
    }

    /// Helper to get an item at a specific index.
    fn get_item_at_index(&self, items_id: &automerge::ObjId, index: usize) -> Option<Item> {
        let (value, _) = self.doc.get(items_id, index).ok()??;

        if let automerge::Value::Scalar(scalar) = value
            && let automerge::ScalarValue::Str(s) = scalar.as_ref()
        {
            // Try to parse as JSON Item
            if let Ok(item) = serde_json::from_str::<Item>(s) {
                return Some(item);
            }
            // Legacy format: plain string, possibly with [x] prefix
            let (checked, name) = if s.starts_with("[x] ") {
                (true, s.trim_start_matches("[x] ").to_string())
            } else {
                (false, s.to_string())
            };
            return Some(Item {
                id: Uuid::new_v4().to_string(),
                name,
                checked,
                quantity: None,
                category: None,
                notes: None,
            });
        }
        None
    }

    /// Gets a single item by its ID.
    ///
    /// # Arguments
    /// * `item_id` - The unique identifier of the item
    ///
    /// # Returns
    /// The item and its index, or None if not found.
    pub fn get_item(&self, item_id: &str) -> Option<(Item, usize)> {
        let items_id = self.get_items()?;
        let len = self.doc.length(&items_id);

        for i in 0..len {
            if let Some(item) = self.get_item_at_index(&items_id, i)
                && item.id == item_id
            {
                return Some((item, i));
            }
        }
        None
    }

    /// Toggles the completion status of an item by ID.
    ///
    /// # Arguments
    /// * `item_id` - The unique identifier of the item to toggle
    ///
    /// # Returns
    /// The updated item, or an error if not found.
    ///
    /// # Example
    /// ```
    /// let mut list = ListDoc::new();
    /// let item = list.add_item("milk", None, None, None)?;
    /// let toggled = list.toggle_item(&item.id)?;
    /// assert!(toggled.checked);
    /// ```
    pub fn toggle_item(&mut self, item_id: &str) -> Result<Item> {
        let (mut item, index) = self
            .get_item(item_id)
            .ok_or_else(|| CoreError::ItemNotFound(0))?;

        item.checked = !item.checked;
        self.update_item_at_index(index, &item)?;
        Ok(item)
    }

    /// Toggles the completion status of an item by index.
    ///
    /// # Arguments
    /// * `index` - The position of the item to toggle (0-based)
    ///
    /// # Returns
    /// The updated item, or an error if not found.
    pub fn toggle_item_at(&mut self, index: usize) -> Result<Item> {
        let items_id = self
            .get_items()
            .ok_or_else(|| CoreError::ItemNotFound(index))?;

        let mut item = self
            .get_item_at_index(&items_id, index)
            .ok_or_else(|| CoreError::ItemNotFound(index))?;

        item.checked = !item.checked;
        self.update_item_at_index(index, &item)?;
        Ok(item)
    }

    /// Updates an item's properties by ID.
    ///
    /// # Arguments
    /// * `item_id` - The unique identifier of the item to update
    /// * `name` - Optional new name
    /// * `quantity` - Optional new quantity
    /// * `category` - Optional new category
    /// * `notes` - Optional new notes
    ///
    /// # Returns
    /// The updated item, or an error if not found.
    ///
    /// # Example
    /// ```
    /// let mut list = ListDoc::new();
    /// let item = list.add_item("milk", None, None, None)?;
    /// let updated = list.update_item(&item.id, Some("whole milk"), Some(2), None, None)?;
    /// ```
    pub fn update_item(
        &mut self,
        item_id: &str,
        name: Option<String>,
        quantity: Option<Option<u32>>,
        category: Option<Option<String>>,
        notes: Option<Option<String>>,
    ) -> Result<Item> {
        let (mut item, index) = self
            .get_item(item_id)
            .ok_or_else(|| CoreError::ItemNotFound(0))?;

        if let Some(n) = name {
            item.name = n;
        }
        if let Some(q) = quantity {
            item.quantity = q;
        }
        if let Some(c) = category {
            item.category = c;
        }
        if let Some(n) = notes {
            item.notes = n;
        }

        self.update_item_at_index(index, &item)?;
        Ok(item)
    }

    /// Helper to update an item at a specific index.
    fn update_item_at_index(&mut self, index: usize, item: &Item) -> Result<()> {
        let items = self.get_or_create_items();
        let json =
            serde_json::to_string(item).map_err(|e| CoreError::Serialization(e.to_string()))?;

        self.doc
            .put(&items, index, json)
            .map_err(|e| CoreError::Crdt(e.to_string()))?;

        Ok(())
    }

    /// Removes an item from the list by ID.
    ///
    /// # Arguments
    /// * `item_id` - The unique identifier of the item to remove
    ///
    /// # Returns
    /// Ok if removed, or an error if not found.
    ///
    /// # Example
    /// ```
    /// let mut list = ListDoc::new();
    /// let item = list.add_item("milk", None, None, None)?;
    /// list.remove_item(&item.id)?;
    /// ```
    pub fn remove_item(&mut self, item_id: &str) -> Result<()> {
        let (_, index) = self
            .get_item(item_id)
            .ok_or_else(|| CoreError::ItemNotFound(0))?;

        self.remove_item_at(index)
    }

    /// Removes an item from the list by index.
    ///
    /// # Arguments
    /// * `index` - The position of the item to remove (0-based)
    ///
    /// # Returns
    /// Ok if removed, or an error if index is out of bounds.
    ///
    /// # Example
    /// ```
    /// let mut list = ListDoc::new();
    /// list.add_item("milk", None, None, None)?;
    /// list.remove_item_at(0)?;
    /// ```
    pub fn remove_item_at(&mut self, index: usize) -> Result<()> {
        let items = self
            .get_items()
            .ok_or_else(|| CoreError::ItemNotFound(index))?;

        let len = self.doc.length(&items);
        if index >= len {
            return Err(CoreError::ItemNotFound(index));
        }

        self.doc
            .delete(&items, index)
            .map_err(|e| CoreError::Crdt(e.to_string()))?;

        Ok(())
    }

    /// Returns the number of items in the list.
    pub fn len(&self) -> usize {
        self.get_items().map(|id| self.doc.length(&id)).unwrap_or(0)
    }

    /// Returns true if the list is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Merges another document into this one.
    ///
    /// This is the core CRDT operation that allows conflict-free synchronization
    /// between devices. Automerge automatically resolves conflicts.
    ///
    /// # Arguments
    /// * `other` - The other document to merge
    ///
    /// # Returns
    /// Ok if merge succeeded, or an error.
    ///
    /// # Example
    /// ```
    /// let mut doc1 = ListDoc::new();
    /// doc1.add_item("milk", None, None, None)?;
    ///
    /// let mut doc2 = ListDoc::load(&doc1.save())?;
    /// doc2.add_item("eggs", None, None, None)?;
    ///
    /// doc1.merge(&mut doc2)?;
    /// assert_eq!(doc1.len(), 2);
    /// ```
    pub fn merge(&mut self, other: &mut ListDoc) -> Result<()> {
        self.doc
            .merge(&mut other.doc)
            .map_err(|e| CoreError::Crdt(e.to_string()))?;
        Ok(())
    }

    /// Merges changes from raw bytes into this document.
    ///
    /// Useful for incremental sync where only changes are transmitted.
    ///
    /// # Arguments
    /// * `bytes` - The serialized changes to merge
    ///
    /// # Returns
    /// Ok if merge succeeded, or an error.
    pub fn merge_bytes(&mut self, bytes: &[u8]) -> Result<()> {
        let mut other = ListDoc::load(bytes)?;
        self.merge(&mut other)
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
    /// list.add_item("milk", None, None, None)?;
    /// let bytes = list.save();
    /// ```
    pub fn save(&mut self) -> Vec<u8> {
        self.doc.save()
    }

    /// Loads a document from serialized bytes.
    ///
    /// # Arguments
    /// * `bytes` - The serialized document state
    ///
    /// # Returns
    /// The loaded document, or an error if bytes are invalid.
    ///
    /// # Example
    /// ```
    /// let bytes = get_bytes_from_storage();
    /// let list = ListDoc::load(&bytes)?;
    /// ```
    pub fn load(bytes: &[u8]) -> Result<Self> {
        let doc = AutoCommit::load(bytes).map_err(|e| CoreError::AutomergeLoad(e.to_string()))?;
        Ok(Self { doc })
    }
}

impl Default for ListDoc {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_and_get_items() {
        let mut doc = ListDoc::new();
        doc.add_item("milk", Some(2), Some("Dairy".to_string()), None)
            .unwrap();
        doc.add_item("bread", None, Some("Bakery".to_string()), None)
            .unwrap();

        let items = doc.get_all_items();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].name, "bread"); // Added second, but at index 0
        assert_eq!(items[1].name, "milk");
    }

    #[test]
    fn test_toggle_item() {
        let mut doc = ListDoc::new();
        let item = doc.add_item("milk", None, None, None).unwrap();
        assert!(!item.checked);

        let toggled = doc.toggle_item(&item.id).unwrap();
        assert!(toggled.checked);

        let toggled_again = doc.toggle_item(&item.id).unwrap();
        assert!(!toggled_again.checked);
    }

    #[test]
    fn test_update_item() {
        let mut doc = ListDoc::new();
        let item = doc.add_item("milk", None, None, None).unwrap();

        let updated = doc
            .update_item(
                &item.id,
                Some("whole milk".to_string()),
                Some(Some(2)),
                Some(Some("Dairy".to_string())),
                None,
            )
            .unwrap();

        assert_eq!(updated.name, "whole milk");
        assert_eq!(updated.quantity, Some(2));
        assert_eq!(updated.category, Some("Dairy".to_string()));
    }

    #[test]
    fn test_remove_item() {
        let mut doc = ListDoc::new();
        let item = doc.add_item("milk", None, None, None).unwrap();
        assert_eq!(doc.len(), 1);

        doc.remove_item(&item.id).unwrap();
        assert_eq!(doc.len(), 0);
    }

    #[test]
    fn test_save_and_load() {
        let mut doc = ListDoc::new();
        doc.add_item("milk", Some(2), None, None).unwrap();
        doc.add_item("bread", None, None, None).unwrap();

        let bytes = doc.save();
        let loaded = ListDoc::load(&bytes).unwrap();

        let items = loaded.get_all_items();
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn test_merge() {
        let mut doc1 = ListDoc::new();
        doc1.add_item("milk", None, None, None).unwrap();

        let bytes = doc1.save();
        let mut doc2 = ListDoc::load(&bytes).unwrap();
        doc2.add_item("eggs", None, None, None).unwrap();

        doc1.merge(&mut doc2).unwrap();

        // Both items should be present after merge
        assert_eq!(doc1.len(), 2);
    }
}
