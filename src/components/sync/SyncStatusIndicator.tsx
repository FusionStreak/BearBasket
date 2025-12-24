// Sync status indicator - shows sync state in the header
// Small, unobtrusive indicator with status and peer count

import { useEffect } from "react";
import { useSyncStore, useSyncEnabled, usePeers } from "../../store/sync";

interface SyncStatusIndicatorProps {
  /** Click handler to open the devices panel */
  onClick?: () => void;
}

export function SyncStatusIndicator({ onClick }: SyncStatusIndicatorProps) {
  const enabled = useSyncEnabled();
  const peers = usePeers();
  const fetchStatus = useSyncStore((state) => state.fetchStatus);
  const fetchPeers = useSyncStore((state) => state.fetchPeers);

  // Fetch initial status on mount
  useEffect(() => {
    fetchStatus();
  }, [fetchStatus]);

  // Poll for peers when sync is enabled
  useEffect(() => {
    if (!enabled) return;

    fetchPeers();
    const interval = setInterval(fetchPeers, 5000); // Poll every 5 seconds

    return () => clearInterval(interval);
  }, [enabled, fetchPeers]);

  const peerCount = peers.length;
  const statusLabel = enabled
    ? `Sync on · ${peerCount} device${peerCount !== 1 ? "s" : ""}`
    : "Sync off";

  return (
    <button
      className="sync-status-indicator"
      onClick={onClick}
      aria-label={`${statusLabel}. Click to open devices panel.`}
      title={statusLabel}
    >
      <span
        className={`sync-status-dot ${
          enabled ? "sync-status-dot--active" : ""
        }`}
        aria-hidden="true"
      />
      <span className="sync-status-text">
        {enabled ? (
          <>
            <span className="sync-status-count">{peerCount}</span>
            <span className="sync-status-label">
              {peerCount === 1 ? "device" : "devices"}
            </span>
          </>
        ) : (
          <span className="sync-status-label">Sync</span>
        )}
      </span>
    </button>
  );
}
