<div align="center">
  <h1>xvi</h1>
  <p><strong>A blazingly fast, modern hex editor built with Rust and GPUI.</strong></p>

  <p>
    <a href="https://github.com/rust-lang/rust"><img src="https://img.shields.io/badge/Rust-stable-orange.svg?logo=rust" alt="Rust Version" /></a>
    <a href="https://gpui.rs/"><img src="https://img.shields.io/badge/Framework-GPUI-blue.svg" alt="GPUI Framework" /></a>
  </p>
</div>

## 🚀 Overview

`xvi` is a high-performance hex editor designed to provide a modern, snappy, and extensible interface for inspecting and editing binary data. Powered by Rust and the [GPUI](https://www.gpui.rs/) framework (the same engine behind the Zed editor), `xvi` delivers a frictionless experience for reverse engineers, developers, and data analysts.

## ✨ Features

- **Side-by-Side Hex & ASCII Views:** Seamlessly inspect binary data with perfectly synchronized hex and text columns.
- **Custom Line Breaks:** Manually split lines for structural visualization of complex binary formats. Break down data your way.
- **Multi-Encoding Support:** Effortlessly toggle between ASCII, UTF-8, and UTF-16 (LE/BE) for accurate text representation.
- **Diff View:** Compare two binary files side-by-side with synchronized scrolling to easily spot modifications.
- **Integrated File Tree:** Navigate your filesystem directly within the editor workspace.
- **VI-like Keybindings:** Stay productive without leaving the home row using familiar `h`, `j`, `k`, `l` navigation, and `/` for fast searching.
- **Asynchronous & Responsive:** Built with Tokio to ensure the UI remains buttery smooth, even when handling massive files or complex diffs.

## 🛠️ Architecture

`xvi` is architected for modularity and performance:

- **`src/core/`**: The robust engine handling buffer management, search algorithms, encoding, and undo/redo history.
- **`src/ui/`**: A modern UI layer comprising dynamic panels (Editor, File Tree, Diff, Settings) and reusable components built exclusively with GPUI.
- **`src/service/`**: High-level orchestrators managing multiple documents and editors.

## 📦 Building and Running

### Prerequisites

Ensure you have the latest stable version of Rust installed. If you need to install Rust, use [Rustup](https://rustup.rs/).

### Quick Start

Navigate to the project root and run the application:

```bash
# Run the application (optionally provide a file or folder path)
cargo run [file_or_folder_path]

# Build the release binary for maximum performance
cargo build --release

# Run the test suite
cargo test

# Format and lint code
cargo fmt
cargo clippy
```

---

*Created with love and [GPUI](https://gpui.rs/).*
