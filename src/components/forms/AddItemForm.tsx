// Form for adding new grocery items

import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { groceryItemSchema, type GroceryItemInput } from "../../lib/schemas";
import { Button, Input } from "../ui";
import { useAppStore, DEFAULT_CATEGORIES } from "../../store";

interface AddItemFormProps {
  onSuccess?: () => void;
}

export function AddItemForm({ onSuccess }: AddItemFormProps) {
  const addItem = useAppStore((state) => state.addItem);

  const {
    register,
    handleSubmit,
    reset,
    formState: { errors, isSubmitting },
  } = useForm<GroceryItemInput>({
    resolver: zodResolver(groceryItemSchema),
    defaultValues: {
      name: "",
      quantity: 1,
      category: "",
    },
  });

  const onSubmit = async (data: GroceryItemInput) => {
    // In real app, this would call a Tauri command first
    // const result = await invoke("add_item", { listId, item: data });

    addItem({
      id: crypto.randomUUID(),
      name: data.name,
      quantity: data.quantity,
      category: data.category || undefined,
      checked: false,
    });

    reset();
    onSuccess?.();
  };

  return (
    <form onSubmit={handleSubmit(onSubmit)} className="add-item-form">
      <Input
        label="Item name"
        placeholder="e.g., Milk"
        error={errors.name?.message}
        {...register("name")}
      />

      <Input
        label="Qty"
        type="number"
        min={1}
        max={999}
        error={errors.quantity?.message}
        {...register("quantity", { valueAsNumber: true })}
      />

      <div className="input-group">
        <label htmlFor="category" className="input-label">
          Category
        </label>
        <select id="category" className="input" {...register("category")}>
          <option value="">None</option>
          {DEFAULT_CATEGORIES.map((cat) => (
            <option key={cat} value={cat}>
              {cat}
            </option>
          ))}
        </select>
      </div>

      <Button type="submit" loading={isSubmitting}>
        Add
      </Button>
    </form>
  );
}
