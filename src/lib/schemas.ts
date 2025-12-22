// Zod schemas for input validation
// These schemas define the shape and validation rules for all form inputs

import { z } from "zod";

// Grocery item schema
export const groceryItemSchema = z.object({
  name: z
    .string()
    .min(1, "Item name is required")
    .max(100, "Item name must be 100 characters or less"),
  quantity: z
    .number()
    .int("Quantity must be a whole number")
    .min(1, "Quantity must be at least 1")
    .max(999, "Quantity must be 999 or less"),
  category: z
    .string()
    .max(50, "Category must be 50 characters or less")
    .optional(),
});

export type GroceryItemInput = z.infer<typeof groceryItemSchema>;

// Grocery list schema
export const groceryListSchema = z.object({
  name: z
    .string()
    .min(1, "List name is required")
    .max(50, "List name must be 50 characters or less"),
});

export type GroceryListInput = z.infer<typeof groceryListSchema>;

// Search/filter schema
export const searchSchema = z.object({
  query: z.string().max(100, "Search query too long"),
});

export type SearchInput = z.infer<typeof searchSchema>;
