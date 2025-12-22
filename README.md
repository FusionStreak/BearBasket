# BearBasket

**BearBasket** is a **local-first, privacy-respecting grocery list app**.
No accounts. No servers. No cloud. Your data lives on your devices and syncs only when *you* decide.

Built with **Rust**, **TypeScript**, and **Tauri**, BearBasket focuses on simplicity, accessibility, and long-term reliability.

---

## Key features

* **Local-first**
  Everything works offline. Your grocery lists are always available.

* **No infrastructure required**
  BearBasket does not rely on any servers, relays, or cloud services — neither ours nor yours.

* **Peer sharing without accounts**
  Share grocery lists using:

  * Local network sync (same Wi-Fi)
  * QR code or file export/import

* **Multiple grocery lists**
  Create one list per store, trip, or household.

* **Accessible by design**
  Keyboard-friendly, screen-reader-friendly, and touch-friendly UI.

* **Safe, modern tech stack**

  * Rust for correctness and data integrity
  * TypeScript for a responsive UI
  * CRDT-based merging for conflict-free sync

---

## What BearBasket is *not*

To keep the project honest and sustainable:

* No accounts
* No push notifications
* No always-online remote sync
* No telemetry or analytics

If two devices aren’t connected (same network or manual exchange), they don’t magically sync — and that’s by design.

---

## Architecture overview

BearBasket follows a **local-first, zero-infrastructure architecture**.

```txt
┌────────────┐        ┌────────────┐
│   Device A │◀──────▶│   Device B │
│            │  LAN   │            │
│  SQLite +  │  Sync  │  SQLite +  │
│   CRDT     │        │   CRDT     │
└────────────┘        └────────────┘
        ▲                    ▲
        │                    │
        └── QR / File Export ┘
```

### Core principles

* Each grocery list is an independent **CRDT document**
* All data is stored locally (SQLite)
* Sync is **peer-to-peer only**
* No central authority or server

---

## Tech stack

### Desktop app

* **Tauri** (lightweight desktop shell)
* **React + TypeScript** (UI)
* **Vite** (frontend tooling)

### Core logic

* **Rust**
* **Automerge** (CRDT)
* **SQLite** (local persistence)

### Sync (optional, local only)

* mDNS (local network discovery)
* Direct peer-to-peer connections (TCP / future QUIC)

---

## Repository layout

```txt
.
├── apps/
│   └── desktop/        # Tauri UI (TypeScript / React)
├── crates/
│   ├── core/           # Rust core (CRDT, DB, commands)
│   └── sync/           # LAN discovery & sync protocol
├── src-tauri/          # Tauri Rust entrypoint
└── Cargo.toml          # Workspace
```

---

## Getting started (development)

### Prerequisites

* **Rust** (stable)
* **Node.js** (LTS)
* **pnpm** or **npm**
* Platform build tools (see Tauri docs)

### Run the desktop app

```sh
pnpm install
pnpm tauri dev
```

---

## How syncing works

BearBasket supports **three zero-infrastructure sync methods**:

1. **Local network sync**
   Devices on the same Wi-Fi automatically discover each other and exchange changes.

2. **QR code sharing**
   Export a grocery list as a QR code and import it on another device.

3. **File export/import**
   Share a `.bearbasket` file via USB, email, or any file transfer method.

All sync methods use **conflict-free merging** — no “last write wins” surprises.

---

## Privacy & security

* All data is stored locally
* No external network connections are required
* Optional encryption can be layered on top of exports
* No tracking, ads, or analytics

---

## Project status

BearBasket is **early-stage** and evolving.

Current focus:

* Core data model
* Reliable local persistence
* Export/import & LAN sync
* Accessibility polish

Expect breaking changes until `v1.0`.

---

## Contributing

Contributions are very welcome — especially:

* Accessibility improvements
* UI/UX polish
* Documentation
* Testing
* Sync robustness

Please read `CONTRIBUTING.md` before submitting PRs.

Guiding principles:

* No servers
* No accounts
* No dark patterns
* Keep it simple

---

## 📜 License

BearBasket is licensed under the **AGPL-3.0-or-later**.

If you want to use BearBasket or its components in a proprietary product, you must comply with the AGPL.

---

## Why “BearBasket”?

Because bears know how to:

* gather food efficiently
* remember where it’s stored
* and mind their own business
