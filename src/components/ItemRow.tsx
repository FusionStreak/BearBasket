// Individual grocery item row with checkbox and inline editing
import { useState, useRef, useEffect } from "react";
import { Checkbox } from "@ark-ui/react/checkbox";
import { useAppStore, type GroceryItem, DEFAULT_CATEGORIES } from "../store";
import { Button } from "./ui";

interface ItemRowProps {
  item: GroceryItem;
}

export function ItemRow({ item }: ItemRowProps) {
  const toggleItem = useAppStore((state) => state.toggleItem);
  const removeItem = useAppStore((state) => state.removeItem);
  const updateItem = useAppStore((state) => state.updateItem);

  const [isEditing, setIsEditing] = useState(false);
  const [editName, setEditName] = useState(item.name);
  const [editQuantity, setEditQuantity] = useState(item.quantity);
  const [editCategory, setEditCategory] = useState(item.category || "");

  const nameInputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (isEditing && nameInputRef.current) {
      nameInputRef.current.focus();
      nameInputRef.current.select();
    }
  }, [isEditing]);

  const handleSave = () => {
    if (!editName.trim()) {
      setEditName(item.name);
      setIsEditing(false);
      return;
    }

    updateItem(item.id, {
      name: editName.trim(),
      quantity: editQuantity,
      category: editCategory || undefined,
    });
    setIsEditing(false);
  };

  const handleCancel = () => {
    setEditName(item.name);
    setEditQuantity(item.quantity);
    setEditCategory(item.category || "");
    setIsEditing(false);
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter") {
      handleSave();
    } else if (e.key === "Escape") {
      handleCancel();
    }
  };

  const handleLabelClick = (e: React.MouseEvent) => {
    // Prevent checkbox toggle when clicking the label area
    e.preventDefault();
    e.stopPropagation();
    setIsEditing(true);
  };

  if (isEditing) {
    return (
      <li className="item-row item-row-editing">
        <div className="item-edit-form">
          <input
            ref={nameInputRef}
            type="text"
            className="input item-edit-name"
            value={editName}
            onChange={(e) => setEditName(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder="Item name"
            aria-label="Item name"
          />
          <div className="item-edit-row">
            <div className="item-edit-field">
              <label className="item-edit-label">Qty</label>
              <input
                type="number"
                className="input item-edit-quantity"
                value={editQuantity}
                onChange={(e) =>
                  setEditQuantity(Math.max(1, parseInt(e.target.value) || 1))
                }
                onKeyDown={handleKeyDown}
                min={1}
                max={999}
                aria-label="Quantity"
              />
            </div>
            <div className="item-edit-field item-edit-field-grow">
              <label className="item-edit-label">Category</label>
              <select
                className="input item-edit-category"
                value={editCategory}
                onChange={(e) => setEditCategory(e.target.value)}
                aria-label="Category"
              >
                <option value="">No category</option>
                {DEFAULT_CATEGORIES.map((cat) => (
                  <option key={cat} value={cat}>
                    {cat}
                  </option>
                ))}
              </select>
            </div>
          </div>
          <div className="item-edit-actions">
            <Button variant="primary" size="sm" onClick={handleSave}>
              Save
            </Button>
            <Button variant="ghost" size="sm" onClick={handleCancel}>
              Cancel
            </Button>
          </div>
        </div>
      </li>
    );
  }

  return (
    <li className="item-row">
      <Checkbox.Root
        checked={item.checked}
        onCheckedChange={() => toggleItem(item.id)}
        className="item-checkbox-root"
      >
        <Checkbox.Control className="item-checkbox">
          <Checkbox.Indicator className="item-checkbox-indicator">
            ✓
          </Checkbox.Indicator>
        </Checkbox.Control>
        <Checkbox.HiddenInput />
      </Checkbox.Root>

      {/* Clickable label area for editing */}
      <button
        type="button"
        className={`item-label-button ${
          item.checked ? "item-label-checked" : ""
        }`}
        onClick={handleLabelClick}
        aria-label={`Edit ${item.name}`}
      >
        <span className="item-name">{item.name}</span>
        {item.quantity > 1 && (
          <span
            className="item-quantity"
            aria-label={`quantity ${item.quantity}`}
          >
            ×{item.quantity}
          </span>
        )}
      </button>

      <Button
        variant="ghost"
        size="sm"
        className="item-delete"
        onClick={() => removeItem(item.id)}
        aria-label={`Remove ${item.name}`}
      >
        <span aria-hidden="true">🗑</span>
      </Button>
    </li>
  );
}
