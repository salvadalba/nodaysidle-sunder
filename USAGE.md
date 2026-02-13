# Sunder — Usage Guide

A local-first markdown research hub with automatic latent link discovery.

---

## Getting Started

### Launch

Open **Sunder** from `/Applications`, Launchpad, or Spotlight (`Cmd + Space` → type "Sunder").

You'll land on the **empty state** — a dark canvas with the Sunder mark and a "New Note" button.

### Development Mode

```bash
cd /Volumes/omarchyuser/projekti/nodaysidle-sunder
npm run tauri dev
```

This hot-reloads both the frontend (Vite) and backend (Rust) as you make changes.

---

## Interface Layout

Sunder has four zones:

```
┌──────────┬─────────────────────────┬──────────┐
│          │        Top Bar          │          │
│          ├─────────────────────────┤          │
│          │                         │  Latent  │
│ Sidebar  │       Editor            │  Links   │
│  (notes) │       (markdown)        │  Panel   │
│          │                         │          │
│          │                         │          │
└──────────┴─────────────────────────┴──────────┘
```

### 1. Sidebar (left)

The sidebar lists all your notes sorted by last updated.

- **Search**: Type in the search bar to filter notes by title or content
- **Navigate**: Arrow keys (`Up`/`Down`) to move, `Enter` to select, `Esc` to clear
- **New note**: Click the `+` button in the sidebar header
- **Active note**: Highlighted with an amber indicator line

### 2. Top Bar

A minimal command strip at the top:

- **Sidebar toggle** (left icon): Show/hide the sidebar
- **Breadcrumb**: Shows "Sunder / Note" when a note is open
- **Graph button** (right): Toggles the graph view (coming in a future update)

### 3. Editor (center)

The main writing area for your notes.

- **Title**: Displayed in large Bricolage Grotesque font at the top
- **Metadata**: Word count and last-updated date below the title
- **Editor**: Plain text/markdown editor (CodeMirror integration coming soon)
- **Auto-save**: Changes are saved automatically after 1 second of inactivity
  - "Saving..." appears in amber while debouncing
  - "Saved" appears in teal when complete
- **Delete**: Trash icon in the top-right of the note header

### 4. Latent Links Panel (right)

This panel appears ~800ms after you select a note, showing semantically related notes.

- **Pulsing amber dot**: Indicates the system is actively discovering connections
- **Similarity score**: Each link shows a percentage and a radial arc gauge
  - Amber (>80%): Strong semantic match
  - Violet (>65%): Moderate match
  - Gray (<65%): Weak but notable connection
- **Click to navigate**: Click any latent link to jump to that note
- The panel auto-hides when no note is selected

---

## Current Status

This is **Task 1** of the Sunder build — the foundation. Here's what works now vs. what's coming:

| Feature | Status |
|---------|--------|
| App scaffold & build | Done |
| SQLite database with WAL mode | Done |
| Note CRUD (create, read, update, delete) | Backend done, UI uses mock data |
| FTS5 full-text search | Backend done, not wired to UI |
| Settings service | Backend done |
| Sidebar with search & keyboard nav | Done (mock data) |
| Markdown editor | Textarea placeholder (CodeMirror coming) |
| Auto-save with debounce | Done (not yet wired to IPC) |
| Latent links panel | Done (mock data) |
| Vector embeddings & semantic search | Coming (Task 2) |
| IPC wiring (frontend ↔ backend) | Coming (Task 3) |
| File watcher & vault sync | Coming (Task 4) |
| Graph visualization | Coming (Task 5) |

---

## Keyboard Shortcuts (planned)

| Shortcut | Action |
|----------|--------|
| `Cmd + B` | Toggle sidebar |
| `Cmd + N` | New note |
| `Cmd + G` | Toggle graph view |
| `Cmd + K` | Command palette (future) |
| `Esc` | Clear search / deselect |

---

## Data Storage

All data is stored locally on your machine:

- **Database**: SQLite with WAL mode in your app data directory
- **No cloud**: Nothing leaves your device
- **No accounts**: No sign-up required

The database location is:
```
~/Library/Application Support/com.sunder.app/sunder.db
```

---

## Rebuilding

To rebuild and reinstall:

```bash
cd /Volumes/omarchyuser/projekti/nodaysidle-sunder
npm run tauri build
cp -R src-tauri/target/release/bundle/macos/Sunder.app /Applications/
```
