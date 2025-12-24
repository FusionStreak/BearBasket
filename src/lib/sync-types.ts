// TypeScript types for sync-related API responses
// Matches the Rust types in src-tauri/src/sync_commands.rs

export interface PeerInfo {
  device_id: string;
  name: string;
  addresses: string[];
  port: number;
  connected: boolean;
}

export interface SyncStatus {
  enabled: boolean;
  device_id: string | null;
  peer_count: number;
  connection_count: number;
}

export interface SyncResult {
  success: boolean;
  list_id: string;
  message: string;
}
