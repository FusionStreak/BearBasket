// Zustand store - central state management
// Follows local-first pattern: state syncs from Tauri commands, never remote APIs

import { create } from "zustand";
import { devtools, persist } from "zustand/middleware";

// Example: Grocery list state
export interface GroceryItem {
  id: string;
  name: string;
  quantity: number;
  checked: boolean;
  category?: string;
}

// Default categories for items
export const DEFAULT_CATEGORIES = [
  "Produce",
  "Dairy",
  "Meat",
  "Bakery",
  "Frozen",
  "Pantry",
  "Beverages",
  "Snacks",
  "Household",
  "Other",
] as const;

// Preset colors for lists
export const LIST_COLORS = [
  { name: "Blue", value: "#3b82f6" },
  { name: "Green", value: "#10b981" },
  { name: "Purple", value: "#8b5cf6" },
  { name: "Orange", value: "#f97316" },
  { name: "Pink", value: "#ec4899" },
  { name: "Teal", value: "#14b8a6" },
  { name: "Red", value: "#ef4444" },
  { name: "Yellow", value: "#eab308" },
] as const;

export type ListColor = (typeof LIST_COLORS)[number]["value"];

export interface GroceryList {
  id: string;
  name: string;
  color: ListColor;
  items: GroceryItem[];
  createdAt: string;
  updatedAt: string;
}

interface AppState {
  // Lists
  lists: GroceryList[];
  activeListId: string | null;

  // Actions
  setLists: (lists: GroceryList[]) => void;
  setActiveList: (id: string | null) => void;
  addList: (list: GroceryList) => void;
  updateList: (id: string, updates: Partial<GroceryList>) => void;
  removeList: (id: string) => void;

  // Item actions (for active list)
  addItem: (item: GroceryItem) => void;
  updateItem: (itemId: string, updates: Partial<GroceryItem>) => void;
  removeItem: (itemId: string) => void;
  toggleItem: (itemId: string) => void;
}

export const useAppStore = create<AppState>()(
  devtools(
    persist(
      (set, _get) => ({
        lists: [],
        activeListId: null,

        setLists: (lists) => set({ lists }),

        setActiveList: (id) => set({ activeListId: id }),

        addList: (list) =>
          set((state) => ({
            lists: [...state.lists, list],
          })),

        updateList: (id, updates) =>
          set((state) => ({
            lists: state.lists.map((list) =>
              list.id === id ? { ...list, ...updates } : list
            ),
          })),

        removeList: (id) =>
          set((state) => ({
            lists: state.lists.filter((list) => list.id !== id),
            activeListId: state.activeListId === id ? null : state.activeListId,
          })),

        addItem: (item) =>
          set((state) => {
            const activeList = state.lists.find(
              (l) => l.id === state.activeListId
            );
            if (!activeList) return state;

            return {
              lists: state.lists.map((list) =>
                list.id === state.activeListId
                  ? { ...list, items: [...list.items, item] }
                  : list
              ),
            };
          }),

        updateItem: (itemId, updates) =>
          set((state) => ({
            lists: state.lists.map((list) =>
              list.id === state.activeListId
                ? {
                    ...list,
                    items: list.items.map((item) =>
                      item.id === itemId ? { ...item, ...updates } : item
                    ),
                  }
                : list
            ),
          })),

        removeItem: (itemId) =>
          set((state) => ({
            lists: state.lists.map((list) =>
              list.id === state.activeListId
                ? {
                    ...list,
                    items: list.items.filter((item) => item.id !== itemId),
                  }
                : list
            ),
          })),

        toggleItem: (itemId) =>
          set((state) => ({
            lists: state.lists.map((list) =>
              list.id === state.activeListId
                ? {
                    ...list,
                    items: list.items.map((item) =>
                      item.id === itemId
                        ? { ...item, checked: !item.checked }
                        : item
                    ),
                  }
                : list
            ),
          })),
      }),
      {
        name: "bearbasket-storage", // localStorage key for persistence
      }
    ),
    { name: "BearBasket" }
  )
);

// Selector hooks for optimized re-renders
export const useActiveList = () =>
  useAppStore((state) =>
    state.lists.find((l) => l.id === state.activeListId)
  );

export const useLists = () => useAppStore((state) => state.lists);
