// Devices panel - shows discovered peers and sync controls
// Accessible modal/drawer for managing sync settings

import { useEffect, useCallback } from "react";
import { Dialog } from "@ark-ui/react/dialog";
import { Portal } from "@ark-ui/react/portal";
import { Button } from "../ui";
import { useSyncStore, usePeers, useSyncEnabled } from "../../store/sync";
import { useAppStore } from "../../store";
import type { PeerInfo } from "../../lib/sync-types";

interface DevicesPanelProps {
  /** Whether the panel is open */
  open: boolean;
  /** Callback when the panel should close */
  onClose: () => void;
}

export function DevicesPanel({ open, onClose }: DevicesPanelProps) {
  const enabled = useSyncEnabled();
  const peers = usePeers();
  const deviceId = useSyncStore((state) => state.deviceId);
  const isLoading = useSyncStore((state) => state.isLoading);
  const error = useSyncStore((state) => state.error);
  const lastSyncResult = useSyncStore((state) => state.lastSyncResult);

  const toggleSync = useSyncStore((state) => state.toggleSync);
  const syncWithPeer = useSyncStore((state) => state.syncWithPeer);
  const fetchPeers = useSyncStore((state) => state.fetchPeers);
  const clearError = useSyncStore((state) => state.clearError);

  const activeListId = useAppStore((state) => state.activeListId);

  // Refresh peers when panel opens
  useEffect(() => {
    if (open && enabled) {
      fetchPeers();
    }
  }, [open, enabled, fetchPeers]);

  // Clear error when panel closes
  useEffect(() => {
    if (!open) {
      clearError();
    }
  }, [open, clearError]);

  const handleSyncWithPeer = useCallback(
    async (peer: PeerInfo) => {
      if (!activeListId) {
        return;
      }
      await syncWithPeer(peer.device_id, activeListId);
    },
    [activeListId, syncWithPeer]
  );

  return (
    <Dialog.Root open={open} onOpenChange={(e) => !e.open && onClose()}>
      <Portal>
        <Dialog.Backdrop className="devices-panel-backdrop" />
        <Dialog.Positioner className="devices-panel-positioner">
          <Dialog.Content className="devices-panel">
            <Dialog.Title className="devices-panel-title">
              <span aria-hidden="true">📡</span> Devices & Sync
            </Dialog.Title>

            <Dialog.Description className="devices-panel-description">
              Sync your grocery lists with other devices on your local network.
              No internet or accounts required.
            </Dialog.Description>

            {/* Sync Toggle Section */}
            <section className="devices-panel-section">
              <div className="sync-toggle-row">
                <div className="sync-toggle-info">
                  <h3 className="sync-toggle-label">Local Sync</h3>
                  <p className="sync-toggle-description">
                    {enabled
                      ? "Discovering devices on your network..."
                      : "Turn on to find nearby devices"}
                  </p>
                </div>
                <Button
                  variant={enabled ? "secondary" : "primary"}
                  size="md"
                  onClick={toggleSync}
                  loading={isLoading}
                  aria-pressed={enabled}
                >
                  {enabled ? "Turn Off" : "Turn On"}
                </Button>
              </div>

              {enabled && deviceId && (
                <div className="device-id-display">
                  <span className="device-id-label">Your device ID:</span>
                  <code className="device-id-value">
                    {deviceId.slice(0, 8)}...
                  </code>
                </div>
              )}
            </section>

            {/* Error Display */}
            {error && (
              <div className="devices-panel-error" role="alert">
                <span className="error-icon" aria-hidden="true">
                  ⚠️
                </span>
                <span className="error-message">{error}</span>
                <button
                  className="error-dismiss"
                  onClick={clearError}
                  aria-label="Dismiss error"
                >
                  ✕
                </button>
              </div>
            )}

            {/* Success Message */}
            {lastSyncResult?.success && (
              <div className="devices-panel-success" role="status">
                <span className="success-icon" aria-hidden="true">
                  ✓
                </span>
                <span className="success-message">
                  {lastSyncResult.message}
                </span>
              </div>
            )}

            {/* Peer List Section */}
            {enabled && (
              <section className="devices-panel-section">
                <h3 className="peers-list-title">
                  Nearby Devices
                  <span className="peers-count">({peers.length})</span>
                </h3>

                {peers.length === 0 ? (
                  <div className="peers-empty">
                    <p>No devices found yet.</p>
                    <p className="peers-empty-hint">
                      Make sure other devices are on the same network and have
                      sync enabled.
                    </p>
                  </div>
                ) : (
                  <ul className="peers-list" role="list">
                    {peers.map((peer) => (
                      <PeerListItem
                        key={peer.device_id}
                        peer={peer}
                        onSync={() => handleSyncWithPeer(peer)}
                        canSync={!!activeListId}
                        isLoading={isLoading}
                      />
                    ))}
                  </ul>
                )}
              </section>
            )}

            {/* Close Button */}
            <div className="devices-panel-footer">
              <Dialog.CloseTrigger asChild>
                <Button variant="secondary" size="md">
                  Close
                </Button>
              </Dialog.CloseTrigger>
            </div>
          </Dialog.Content>
        </Dialog.Positioner>
      </Portal>
    </Dialog.Root>
  );
}

interface PeerListItemProps {
  peer: PeerInfo;
  onSync: () => void;
  canSync: boolean;
  isLoading: boolean;
}

function PeerListItem({ peer, onSync, canSync, isLoading }: PeerListItemProps) {
  const shortId = peer.device_id.slice(0, 8);

  return (
    <li className="peer-item">
      <div className="peer-info">
        <span className="peer-icon" aria-hidden="true">
          {peer.connected ? "🔗" : "💻"}
        </span>
        <div className="peer-details">
          <span className="peer-name">{peer.name}</span>
          <span className="peer-id">{shortId}</span>
        </div>
      </div>
      <Button
        variant="primary"
        size="sm"
        onClick={onSync}
        disabled={!canSync || isLoading}
        title={!canSync ? "Select a list first" : `Sync with ${peer.name}`}
      >
        Sync
      </Button>
    </li>
  );
}
