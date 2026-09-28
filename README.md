# 🚀 Explor

A **modern**, **rounded**, **ultra-fast** file manager designed for **keyboard-first navigation**, built in **Rust** with **Libadwaita** and **GTK4**.

Specially crafted for modern Wayland environments like **Hyprland** (Omarchy), Sway, GNOME, or any Wayland/X11 window manager.

---

## ✨ Key Feature

- **📊 Non-Blocking Multi-Process Sidebar Progress Indicator**:
  - **Concurrent Multi-Task Support**: Run and track **multiple background operations at once** (e.g., extract a large archive while copying files or compressing folders). Each task features its own progress bar, percentage label, real-time speed metrics (MB/s), and independent cancel/dismiss buttons.
  - Operations run in the background **without modal dialogs blocking UI interaction**, allowing seamless folder browsing and tab navigation during long tasks.
  - **Device-Style Design (Expanded Sidebar)**: Rendered under `ACTIVE OPERATIONS`, styled after device disk usage meters with distinct task icons, numerical percentages, live speeds, and cancel (`✕`) buttons.
  - **Collapsed Sidebar Mode (Dock)**:
    - **Unified Badge Indicator**: Displays total running tasks in the compact dock icon.
    - **Mouse Hover**: Hovering over the indicator pops open a detailed *Popover* listing all active operations with individual progress bars.
    - **Single-Key Toggle (<kbd>b</kbd>)**: Press <kbd>b</kbd> at any time to toggle the progress popover without touching the mouse.
  - **Independent Cancel Button**: Cancels only the specific task without interrupting others.
  - **Individual "✓ Dismiss" Button**: Upon completion, shows status and a **"✓ Dismiss"** button (or <kbd>Enter</kbd>) to clear the card.
  - **System Notifications**: Native desktop notifications (`notify-send`) and in-app toasts on task success or failure.
- **🗜️ Fast ZIP Compression (<kbd>z</kbd>)**:
  - Compress any selected file or folder directly to `.zip` with the <kbd>z</kbd> key, from the action bubble, or via the command palette.
- **📑 Complete Floating Tabs System**:
  - Open directories or bookmarks in a **new tab** using <kbd>Ctrl+T</kbd>, the action bubble, or the command palette.
  - **Floating Tab Bubble**: When multiple tabs are open, they are neatly displayed in a translucent capsule above the bottom action bar.
  - **Ultra-Fast Tab Navigation**:
    - <kbd>Ctrl + 1</kbd> ... <kbd>Ctrl + 9</kbd> to jump straight to tabs 1 through 9.
    - <kbd>Ctrl + Tab</kbd> / <kbd>Ctrl + Shift + Tab</kbd> to cycle between tabs.
    - <kbd>Ctrl + W</kbd> or `✕` button to close the active tab.
    - `+` button in the bubble to create a new tab with one click.
- **🖼️ Dual Views: List or Grid (Cards)**:
  - Toggle instantly between traditional **list view** and **grid view** with the single <kbd>v</kbd> key (or from Settings / Command Palette).
  - In grid mode, items are presented as cards with enlarged icons, centered filenames, and metadata.
- **🎛️ Fully Customizable Keyboard Shortcuts**:
  - Edit any action shortcut by clicking its badge or via Settings / Command Palette.
  - **Collision Prevention & Alerts**: If an assigned shortcut conflicts with an existing action, Explor blocks the duplicate assignment and shows a warning describing the conflict.
  - **Full or Individual Reset**: One-click reset to factory defaults or individual revert buttons per action.
  - Real-time synchronization with command palette and action bar badges, persisted in `~/.config/explor/shortcuts.json`.
- **🪟 Window Opacity / Transparency Control**:
  - Under **Appearance & Borders** in Settings (<kbd>Ctrl+,</kbd>), adjust background opacity from 30% translucent to 100% opaque.
  - Native Wayland compositor support (Hyprland, Sway, etc.) enabling blur and glassmorphism styling.
- **🎨 System Icon Theme Selector**:
  - In Settings (<kbd>Ctrl+,</kbd>), choose from installed system icon themes (`Papirus`, `Adwaita`, `Breeze`, `Yaru`, etc.).
  - Applied instantly across the interface without restarting the app, saved in `~/.config/explor/settings.json`.
- **🔍 Dynamic Zoom Control (<kbd>Ctrl + +</kbd> / <kbd>Ctrl + -</kbd>)**:
  - Scale icons and file elements in list and grid views (from 60% to 200%).
  - <kbd>Ctrl + 0</kbd> resets zoom to default (100%).
- **📱 Mobile & External Device Detection**: Live detection of mobile devices (Android MTP), USB flash drives, and external disks via GIO VolumeMonitor and GVfs without duplicates.
- **💾 Full Sidebar Navigation**: Fluidly navigate between bookmarks and storage devices using <kbd>j</kbd>/<kbd>k</kbd> and open with <kbd>Enter</kbd>.
- **📋 Compact Path Bar with Integrated Copy Button**:
  - Clean centered horizontal pill bar.
  - One-click or <kbd>Ctrl+Shift+C</kbd> to copy current path to clipboard.
- **🎨 Clean & Minimalist Top Bar**:
  - Header bar focused on efficient navigation without cluttered buttons.
  - Toggle dotfiles with <kbd>Ctrl+H</kbd>.
- **🗃️ Collapsible Sidebar**: Switch between full sidebar and 58px compact dock with <kbd>Ctrl+B</kbd>.
- **⚙️ Full Settings Dialog (`Ctrl+,`)**:
  - **System icon theme**: Dropdown with all detected icon packages.
  - **Corner roundness**: Real-time slider from **square corners (0%)** to **fully rounded (100%)**.
  - **Compact navbar**: Narrow layout mode for smaller screens.
  - **Grid view**: Persisted view preference.
  - **Show preview pane**: Toggle side preview pane.
  - **Auto-hide navbar**: Reveal on hover or toggle with <kbd>F10</kbd>.
  - **Compact sidebar**: Start with collapsed dock.
  - **Show hidden files**: Show dotfiles by default.
- **⌨️ Unified Command Palette (`Ctrl+P`)**: Fast search modal for commands, shortcuts, and directory paths.
- **🌗 Omarchy Theme Sync**: Automatically inherits active Omarchy color themes (`Aamis`, `Ash`, etc.).
- **🫧 Floating Action Bubble**: Context-aware floating capsule at the bottom for common file operations.
- **📦 Fast Archive Extraction**: Extract `.zip`, `.tar.gz`, `.tar.xz`, `.7z`, `.rar`, etc., with <kbd>e</kbd>.
- **⚡ Safe External Storage Operations**: Copy, cut, paste, and delete operations compatible with MTP, Android, and external drives.

---

## ⌨️ Keyboard Shortcuts Reference

### 📑 Tabs
| Shortcut | Action |
|---|---|
| `Ctrl + T` | **Open selected folder or bookmark in new tab** |
| `Ctrl + W` | **Close active tab** |
| `Ctrl + 1` ... `Ctrl + 9` | **Jump directly to tab 1 to 9** |
| `Ctrl + Tab` | **Next tab** |
| `Ctrl + Shift + Tab` | **Previous tab** |

### 🧭 Navigation
| Shortcut | Action |
|---|---|
| `j` | Select next item |
| `k` | Select previous item |
| `Enter` | Open file/folder (or enter bookmark/device) |
| `Backspace` | Go to parent directory (`..`) |
| `←` | Focus sidebar |
| `→` | Focus file list |
| `g` | Go to first item |
| `G` | Go to last item |
| `Alt + ←` | History back |
| `Alt + →` | History forward |
| `Ctrl + L` | Edit path manually |
| `Ctrl + Shift + C` | **Copy current folder path to clipboard** |

### 🖼️ Views & Zoom
| Shortcut | Action |
|---|---|
| `v` | **Toggle between List and Grid view** |
| `Ctrl + +` | **Zoom In** |
| `Ctrl + -` | **Zoom Out** |
| `Ctrl + 0` | **Reset Zoom (100%)** |

### 🛠️ File Operations (Action Bubble)
| Shortcut | Action |
|---|---|
| `y` | Copy selected file (Yank) |
| `x` | Cut selected file |
| `p` | Paste file into current folder |
| `e` | **Extract archive** (`.zip`, `.tar.*`, `.7z`, etc.) |
| `z` | **Compress to ZIP** (selected file or folder) |
| `d` | Move to trash |
| `r` | Rename selected file |
| `a` | Create new file |
| `Shift + A` | Create new folder |
| `o` | Open terminal in current folder |

### 🔍 Panels & Settings
| Shortcut | Action |
|---|---|
| `Ctrl + P` | **Open Command Palette** |
| `Ctrl + ,` | **Open Settings** |
| `Space` | **Toggle side preview pane** |
| `F10` | **Toggle top navigation bar** |
| `Ctrl + B` | **Toggle sidebar (collapse/expand)** |
| `b` | **Show operations progress (when sidebar is collapsed)** |
| `Ctrl + F` | Activate real-time search filter |
| `Ctrl + H` | Toggle hidden files |
| `Esc` | Cancel search / close dialogs / focus list |

---

## 🚀 Building and Running

### Development Mode:
```bash
cargo run
```

### Production Build (Optimized):
```bash
cargo build --release
```
The binary will be generated at `target/release/explor`.
