use gpui::Application;
use gpui_component_assets::Assets;

mod actions;
mod app_state;
mod appearance;
mod model;
mod service;
mod theme;
mod ui;

use ui::workspace::Workspace;

fn main() {
    let rt = tokio::runtime::Builder::new_multi_thread().enable_all().build().unwrap();
    let _guard = rt.enter();

    // Parse command line arguments
    let args: Vec<String> = std::env::args().collect();

    let app = Application::new().with_assets(Assets);

    app.run(move |cx| {
        app_state::AppState::init(cx);
        appearance::init(cx);

        gpui_component::init(cx);
        theme::init(cx);
        ui::workspace::init(cx);
        ui::file_tree_panel::init(cx);
        ui::editor_panel::init(cx);
        ui::diff_panel::init(cx);

        // Parse command line arguments (skip the first one which is the program name)
        let mut files_to_open = Vec::new();
        let mut folder_to_open = None;

        for arg in args.iter().skip(1) {
            let path = std::path::PathBuf::from(arg);
            if path.is_file() {
                files_to_open.push(path);
            } else if path.is_dir() {
                // Use the last directory as the folder to open
                folder_to_open = Some(path);
            } else {
                eprintln!("Warning: Path does not exist: {}", path.display());
            }
        }

        Workspace::open_window(cx, files_to_open, folder_to_open).detach();
    });
}
