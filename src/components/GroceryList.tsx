// Grocery list view - displays items with add form
import { useState } from "react";
import {
  useAppStore,
  type GroceryList as GroceryListType,
  LIST_COLORS,
  type ListColor,
} from "../store";
import { ItemRow } from "./ItemRow";
import { AddItemForm } from "./forms";
import { ColorPicker, Button } from "./ui";

interface GroceryListProps {
  list: GroceryListType;
}

export function GroceryList({ list }: GroceryListProps) {
  const updateList = useAppStore((state) => state.updateList);
  const [isEditingColor, setIsEditingColor] = useState(false);
  const uncheckedItems = list.items.filter((item) => !item.checked);
  const checkedItems = list.items.filter((item) => item.checked);

  const handleColorChange = (color: ListColor) => {
    updateList(list.id, { color });
    setIsEditingColor(false);
  };

  return (
    <div className="grocery-list">
      <header className="grocery-list-header">
        <div className="grocery-list-title-row">
          <div
            className="grocery-list-color-dot"
            style={{ backgroundColor: list.color || LIST_COLORS[0].value }}
            onClick={() => setIsEditingColor(!isEditingColor)}
            role="button"
            tabIndex={0}
            aria-label="Change list color"
            onKeyDown={(e) => {
              if (e.key === "Enter" || e.key === " ") {
                setIsEditingColor(!isEditingColor);
              }
            }}
          />
          <h2 className="grocery-list-title">{list.name}</h2>
          <Button
            variant="ghost"
            size="sm"
            onClick={() => setIsEditingColor(!isEditingColor)}
            aria-label="Edit list color"
            aria-expanded={isEditingColor}
          >
            🎨
          </Button>
        </div>
        <p className="grocery-list-meta">
          {list.items.length} items • {checkedItems.length} done
        </p>

        {/* Color picker for editing */}
        {isEditingColor && (
          <div className="grocery-list-color-editor">
            <ColorPicker
              label="Change color"
              value={list.color || LIST_COLORS[0].value}
              onChange={handleColorChange}
            />
          </div>
        )}
      </header>

      {/* Collapsible add item form */}
      <details className="add-item-section" open>
        <summary className="add-item-summary">
          <span className="add-item-label">➕ Add item</span>
        </summary>
        <AddItemForm />
      </details>

      {/* Unchecked items grouped by category */}
      <section aria-label="Items to get">
        {uncheckedItems.length === 0 ? (
          <p className="items-empty">All done! 🎉</p>
        ) : (
          <>
            {/* Group items by category */}
            {(() => {
              const grouped = uncheckedItems.reduce((acc, item) => {
                const cat = item.category || "Uncategorized";
                if (!acc[cat]) acc[cat] = [];
                acc[cat].push(item);
                return acc;
              }, {} as Record<string, typeof uncheckedItems>);

              // Sort categories, with Uncategorized last
              const categories = Object.keys(grouped).sort((a, b) => {
                if (a === "Uncategorized") return 1;
                if (b === "Uncategorized") return -1;
                return a.localeCompare(b);
              });

              return categories.map((category) => (
                <div key={category} className="item-category-group">
                  {categories.length > 1 && (
                    <h3 className="item-category-heading">{category}</h3>
                  )}
                  <ul className="items-list" role="list">
                    {grouped[category].map((item) => (
                      <ItemRow key={item.id} item={item} />
                    ))}
                  </ul>
                </div>
              ));
            })()}
          </>
        )}
      </section>

      {/* Checked items (collapsible) */}
      {checkedItems.length > 0 && (
        <details className="checked-section">
          <summary className="checked-summary">
            <span className="checked-label">
              Completed ({checkedItems.length})
            </span>
          </summary>
          <ul className="items-list items-list-checked" role="list">
            {checkedItems.map((item) => (
              <ItemRow key={item.id} item={item} />
            ))}
          </ul>
        </details>
      )}
    </div>
  );
}
