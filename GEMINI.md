# xvw

`xvw` is a high-performance hex editor built with Rust and the [GPUI](https://www.gpui.rs/) framework (the same framework used by the Zed editor). It provides a modern, fast, and extensible interface for inspecting and editing binary data.

## Project Overview

- **Main Technologies:** Rust, GPUI, Tokio (for async tasks), Serde (for serialization).
- **Architecture:**
    - `src/core/`: Contains the core logic for the editor.
        - `buffer.rs`: Simple byte buffer management.
        - `editor.rs`: Manages cursor, selection, line layout, and custom breaks.
        - `document.rs`: Represents a file or data stream being edited.
        - `search.rs`: Implementation of search algorithms (hex/text).
        - `history.rs`: Undo/Redo management.
        - `encoding.rs`: Support for various character encodings (ASCII, UTF-8, UTF-16).
    - `src/service/`: High-level services like `EditorService` for managing multiple documents and editors.
    - `src/ui/`: All UI-related code.
        - `workspace.rs`: The main window container, managing panels and docks.
        - `panels/`: Specific UI panels like `EditorPanel`, `FileTreePanel`, `DiffPanel`, and `SettingsPanel`.
        - `components/`: Reusable UI components like `HexView`, `StatusBar`, `SearchBar`, and `Toolbar`.
    - `src/app_state.rs`: Global application state accessible throughout the app.
    - `src/actions.rs`: Definitions of user actions that can be triggered via menus or keybindings.
- **Key Features:**
    - **Hex & ASCII Views:** Side-by-side inspection of binary data.
    - **Custom Line Breaks:** Users can manually split lines for better visualization of structured binary data.
    - **Multi-Encoding Support:** Toggle between ASCII, UTF-8, and UTF-16 (LE/BE).
    - **Diff View:** Compare two binary files side-by-side with synchronized scrolling.
    - **File Tree:** Navigate the filesystem directly within the editor.
    - **VI-like Bindings:** Support for `h`, `j`, `k`, `l` navigation, and `/` for search.

## Building and Running

### Prerequisites

- Rust (latest stable version recommended)

### Commands

- **Run the application:** `cargo run [file_or_folder_path]`
- **Build for production:** `cargo build --release`
- **Run tests:** `cargo test`
- **Format code:** `cargo fmt`
- **Lint code:** `cargo clippy`

## Development Conventions

- **UI Framework:** Strictly follow GPUI patterns. Use `cx.new`, `cx.observe`, and `cx.subscribe` for reactive state management.
- **Actions:** Define new user interactions in `src/actions.rs` and bind them in `src/ui/workspace.rs` or specific components.
- **Styling:** Use the theme-aware styling provided by `gpui` and the local `theme.rs`. Prefer `cx.theme()` for colors and spacing.
- **Asynchronous Tasks:** Use `cx.spawn` or `cx.spawn_in` for long-running operations like file I/O or diff computation to keep the UI responsive.
- **Testing:** Add unit tests for core logic in `src/core/` and integration tests where appropriate.
