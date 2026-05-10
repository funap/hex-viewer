# xvi - Agent Guidelines

## Project Overview
- **Purpose**: High-performance hex editor built with Rust + GPUI framework
- **Architecture**: `src/core/` (engine), `src/ui/` (GPUI components), `src/service/` (orchestration)
- **Entry point**: `src/main.rs:16` - initializes GPUI app with Tokio runtime

## Commands
```bash
cargo run [file_or_folder_path]  # Run application
cargo build --release            # Production build
cargo test                       # Run 28 unit tests
cargo fmt                        # Format (max_width: 160)
cargo clippy                     # Lint (50+ warnings acceptable)
```

## Key Architecture Notes
- **GPUI reactive pattern**: Use `cx.new`, `cx.observe`, `cx.spawn` for state management
- **Async**: Tokio multi-thread runtime initialized in `main.rs`; use `cx.spawn_in` for I/O
- **Line endings**: Native (configured in `.serena/project.yml`)
- **Rust edition**: 2024 (see `Cargo.toml`)

## Structure Parsing
- Binary structure definitions use **Kaitai Struct** format (`.ksy` YAML files)
- Runtime loading via **Action: Load Structure Definition** (`cmd-shift-s`)
- See `docs/structure-definition-spec.md` for supported features

## Testing
- 28 unit tests in `src/core/` modules (buffer, editor, diff, search)
- No integration/UI tests present
- Tests cover: cursor movement, encoding, diff computation, search navigation

## Conventions
- **Actions**: Define in `src/actions.rs`, bind in `src/main.rs` keymap
- **Theming**: Use `cx.theme()` for colors; themes in `themes/` directory
- **Panels**: Initialized in `main.rs`; see `src/ui/panels/` for implementations

## Gotchas
- Do not commit `.serena/`, `.vscode/`, or `target/` (gitignored)
- GPUI-specific patterns differ from standard React/Elm architectures
- Custom line breaks override automatic joining (see `src/core/editor.rs`)
