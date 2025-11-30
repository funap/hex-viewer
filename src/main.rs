use gpui::Application;
use gpui_component_assets::Assets;

mod actions;
mod app_state;
mod keybindings;
mod model;
mod service;
mod theme;
mod ui;

use ui::workspace::Workspace;

fn main() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    let _guard = rt.enter();

    // Parse command line arguments
    let args: Vec<String> = std::env::args().collect();
    let path_arg = args.get(1).map(|s| std::path::PathBuf::from(s));

    let app = Application::new().with_assets(Assets);

    app.run(move |cx| {
        app_state::AppState::init(cx);

        keybindings::init(cx);
        gpui_component::init(cx);
        theme::init(cx);
        ui::file_tree_panel::init(cx);
        ui::editor_panel::init(cx);

        // Determine what to open based on command line argument
        let (file_to_open, folder_to_open) = if let Some(path) = path_arg {
            if path.is_file() {
                (Some(path), None)
            } else if path.is_dir() {
                (None, Some(path))
            } else {
                eprintln!("Warning: Path does not exist: {}", path.display());
                (None, None)
            }
        } else {
            // No argument: open empty buffer
            (None, None)
        };

        Workspace::new_local(cx, file_to_open, folder_to_open).detach();
    });
}
