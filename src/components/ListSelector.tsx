// List selector sidebar - shows all lists, allows creating new ones
import { useState } from "react";
import {
  useAppStore,
  type GroceryList,
  LIST_COLORS,
  type ListColor,
} from "../store";
import { Button, Input, ColorPicker } from "./ui";

// Helper to determine if text should be light or dark based on background
function getContrastColor(hexColor: string): string {
  const hex = hexColor.replace("#", "");
  const r = parseInt(hex.substring(0, 2), 16);
  const g = parseInt(hex.substring(2, 4), 16);
  const b = parseInt(hex.substring(4, 6), 16);
  // Perceived brightness formula
  const brightness = (r * 299 + g * 587 + b * 114) / 1000;
  return brightness > 128 ? "#1f2937" : "#ffffff";
}

interface ListSelectorProps {
  lists: GroceryList[];
  activeListId: string | null;
}

export function ListSelector({ lists, activeListId }: ListSelectorProps) {
  const setActiveList = useAppStore((state) => state.setActiveList);
  const addList = useAppStore((state) => state.addList);
  const [isCreating, setIsCreating] = useState(false);
  const [newListName, setNewListName] = useState("");
  const [newListColor, setNewListColor] = useState<ListColor>(
    LIST_COLORS[0].value
  );

  const handleCreateList = () => {
    if (!newListName.trim()) return;

    const newList: GroceryList = {
      id: crypto.randomUUID(),
      name: newListName.trim(),
      color: newListColor,
      items: [],
      createdAt: new Date().toISOString(),
      updatedAt: new Date().toISOString(),
    };

    addList(newList);
    setActiveList(newList.id);
    setNewListName("");
    setNewListColor(LIST_COLORS[0].value);
    setIsCreating(false);
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter") {
      handleCreateList();
    } else if (e.key === "Escape") {
      setIsCreating(false);
      setNewListName("");
    }
  };

  return (
    <details className="list-selector" open>
      <summary className="list-selector-header">
        <span className="list-selector-title">My Lists</span>
        <Button
          variant="primary"
          size="sm"
          onClick={(e) => {
            e.preventDefault(); // Prevent details toggle
            setIsCreating(true);
          }}
          aria-label="Create new list"
        >
          + New
        </Button>
      </summary>

      {/* New list form */}
      {isCreating && (
        <div className="new-list-form" role="form" aria-label="Create new list">
          <Input
            label="List name"
            placeholder="e.g., Weekly groceries"
            value={newListName}
            onChange={(e) => setNewListName(e.target.value)}
            onKeyDown={handleKeyDown}
            autoFocus
          />
          <ColorPicker
            label="Color"
            value={newListColor}
            onChange={setNewListColor}
          />
          <div className="new-list-actions">
            <Button variant="primary" size="sm" onClick={handleCreateList}>
              Create
            </Button>
            <Button
              variant="ghost"
              size="sm"
              onClick={() => {
                setIsCreating(false);
                setNewListName("");
                setNewListColor(LIST_COLORS[0].value);
              }}
            >
              Cancel
            </Button>
          </div>
        </div>
      )}

      {/* Lists */}
      <ul className="list-items" role="listbox" aria-label="Available lists">
        {lists.length === 0 ? (
          <li className="list-empty">
            <p>No lists yet. Create one to get started!</p>
          </li>
        ) : (
          lists.map((list) => {
            const isActive = list.id === activeListId;
            const bgColor = list.color || LIST_COLORS[0].value;
            const textColor = getContrastColor(bgColor);

            return (
              <li key={list.id}>
                <button
                  className={`list-item ${isActive ? "list-item-active" : ""}`}
                  style={{
                    backgroundColor: bgColor,
                    borderColor: bgColor,
                    color: textColor,
                    ["--list-text-color" as string]: textColor,
                  }}
                  onClick={() => setActiveList(list.id)}
                  role="option"
                  aria-selected={isActive}
                >
                  <span className="list-item-name">{list.name}</span>
                  <span
                    className="list-item-count"
                    aria-label={`${list.items.length} items`}
                    style={{
                      backgroundColor: isActive
                        ? "rgba(255,255,255,0.2)"
                        : "rgba(0,0,0,0.1)",
                    }}
                  >
                    {list.items.length}
                  </span>
                </button>
              </li>
            );
          })
        )}
      </ul>
    </details>
  );
}
