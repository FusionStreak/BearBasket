// Sync store - manages sync state and peer discovery
// Follows local-first pattern: all sync is peer-to-peer, no servers

import { create } from "zustand";
import { devtools } from "zustand/middleware";
import { invoke } from "@tauri-apps/api/core";
import type { PeerInfo, SyncStatus, SyncResult } from "../lib/sync-types";

interface SyncState {
  // Sync status
  enabled: boolean;
  deviceId: string | null;
  
  // Discovered peers
  peers: PeerInfo[];
  
  // UI state
  isLoading: boolean;
  error: string | null;
  lastSyncResult: SyncResult | null;
  
  // Actions
  fetchStatus: () => Promise<void>;
  fetchPeers: () => Promise<void>;
  toggleSync: () => Promise<void>;
  syncWithPeer: (peerId: string, listId: string) => Promise<SyncResult>;
  clearError: () => void;
}

export const useSyncStore = create<SyncState>()(
  devtools(
    (set, get) => ({
      // Initial state
      enabled: false,
      deviceId: null,
      peers: [],
      isLoading: false,
      error: null,
      lastSyncResult: null,

      fetchStatus: async () => {
        try {
          const status = await invoke<SyncStatus>("get_sync_status");
          set({
            enabled: status.enabled,
            deviceId: status.device_id,
          });
        } catch (error) {
          set({ error: String(error) });
        }
      },

      fetchPeers: async () => {
        try {
          const peers = await invoke<PeerInfo[]>("get_peers");
          set({ peers });
        } catch (error) {
          set({ error: String(error) });
        }
      },

      toggleSync: async () => {
        set({ isLoading: true, error: null });
        try {
          const status = await invoke<SyncStatus>("toggle_sync");
          set({
            enabled: status.enabled,
            deviceId: status.device_id,
            isLoading: false,
          });
          
          // If we just enabled sync, start polling for peers
          if (status.enabled) {
            get().fetchPeers();
          } else {
            set({ peers: [] });
          }
        } catch (error) {
          set({ error: String(error), isLoading: false });
        }
      },

      syncWithPeer: async (peerId: string, listId: string) => {
        set({ isLoading: true, error: null });
        try {
          const result = await invoke<SyncResult>("sync_with_peer", {
            peerDeviceId: peerId,
            listId,
          });
          set({ lastSyncResult: result, isLoading: false });
          return result;
        } catch (error) {
          const errorMsg = String(error);
          set({ error: errorMsg, isLoading: false });
          return {
            success: false,
            list_id: listId,
            message: errorMsg,
          };
        }
      },

      clearError: () => set({ error: null }),
    }),
    { name: "BearBasket-Sync" }
  )
);

// Selector hooks for optimized re-renders
export const useSyncEnabled = () => useSyncStore((state) => state.enabled);
export const usePeers = () => useSyncStore((state) => state.peers);
export const useSyncLoading = () => useSyncStore((state) => state.isLoading);
