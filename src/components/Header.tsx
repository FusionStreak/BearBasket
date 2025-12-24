// App header with navigation and actions
import { useState } from "react";
import { Dialog } from "@ark-ui/react/dialog";
import { Portal } from "@ark-ui/react/portal";
import { Button } from "./ui";
import { SyncStatusIndicator } from "./sync/SyncStatusIndicator";
import { DevicesPanel } from "./sync/DevicesPanel";

export function Header() {
  const [menuOpen, setMenuOpen] = useState(false);
  const [devicesPanelOpen, setDevicesPanelOpen] = useState(false);

  return (
    <header className="app-header">
      <div className="header-content">
        {/* Logo / App name */}
        <h1 className="app-title">
          <span aria-hidden="true">🐻</span> BearBasket
        </h1>

        {/* Sync status indicator */}
        <SyncStatusIndicator onClick={() => setDevicesPanelOpen(true)} />

        {/* Mobile menu toggle */}
        <button
          className="menu-toggle"
          onClick={() => setMenuOpen(!menuOpen)}
          aria-expanded={menuOpen}
          aria-controls="mobile-menu"
          aria-label={menuOpen ? "Close menu" : "Open menu"}
        >
          <span className="menu-icon" aria-hidden="true">
            {menuOpen ? "✕" : "☰"}
          </span>
        </button>

        {/* Desktop nav */}
        <nav className="header-nav desktop-only" aria-label="Main navigation">
          <Button
            variant="ghost"
            size="sm"
            onClick={() => setDevicesPanelOpen(true)}
          >
            Devices
          </Button>
          <Button variant="ghost" size="sm">
            Settings
          </Button>
        </nav>
      </div>

      {/* Mobile menu (slide-down) */}
      <Dialog.Root open={menuOpen} onOpenChange={(e) => setMenuOpen(e.open)}>
        <Portal>
          <Dialog.Backdrop className="mobile-menu-backdrop" />
          <Dialog.Positioner className="mobile-menu-positioner">
            <Dialog.Content className="mobile-menu" id="mobile-menu">
              <Dialog.Title className="visually-hidden">Menu</Dialog.Title>
              <Dialog.Description className="visually-hidden">
                App navigation menu
              </Dialog.Description>
              <nav className="mobile-nav" aria-label="Mobile navigation">
                <Button
                  variant="ghost"
                  size="lg"
                  className="mobile-nav-item"
                  onClick={() => {
                    setMenuOpen(false);
                    setDevicesPanelOpen(true);
                  }}
                >
                  Sync Devices
                </Button>
                <Button variant="ghost" size="lg" className="mobile-nav-item">
                  Settings
                </Button>
                <Button variant="ghost" size="lg" className="mobile-nav-item">
                  Help
                </Button>
              </nav>
              <Dialog.CloseTrigger asChild>
                <Button
                  variant="secondary"
                  size="md"
                  className="mobile-menu-close"
                >
                  Close
                </Button>
              </Dialog.CloseTrigger>
            </Dialog.Content>
          </Dialog.Positioner>
        </Portal>
      </Dialog.Root>

      {/* Devices panel */}
      <DevicesPanel
        open={devicesPanelOpen}
        onClose={() => setDevicesPanelOpen(false)}
      />
    </header>
  );
}
