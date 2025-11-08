//! このモジュールは、アプリケーションのエントリポイントと`Workspace` UIコンポーネントを定義します。
//!
//! `gpui`と`gpui_component`クレートを使用して、タブとボタンを持つシンプルなアプリケーションを
//! 作成する方法を示しています。

use gpui::{
    App, AppContext, Application, Context, Entity, EventEmitter, FocusHandle, Focusable,
    IntoElement, ParentElement, Render, SharedString, Window, WindowOptions, div,
};
use gpui_component::Root;
use gpui_component::dock::{DockArea, DockItem, Panel, PanelEvent};
use gpui_component::input::{Input, InputState};
use std::sync::Arc;

/// `Workspace`コンポーネントの主要なアプリケーション状態。
///
/// この構造体は、現在選択されているタブの状態を保持します。
pub struct Workspace {
    dock_area: Entity<DockArea>,
}

impl Render for Workspace {
    /// タブバーと、選択されたタブに基づいて内容が変化するコンテンツエリア、およびボタンを含む
    /// `Workspace`コンポーネントをレンダリングします。
    fn render(&mut self, _: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        self.dock_area.clone()
    }
}

/// A simple panel for demonstration purposes.
pub struct MyPanel {
    title: SharedString,
    focus_handle: FocusHandle,
}

impl MyPanel {
    pub fn new(title: impl Into<SharedString>, cx: &mut Context<Self>) -> Self {
        Self {
            title: title.into(),
            focus_handle: cx.focus_handle(),
        }
    }
}

impl Render for MyPanel {
    fn render(&mut self, _: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().child(format!("Content for {}", self.title))
    }
}

impl EventEmitter<PanelEvent> for MyPanel {}

impl Focusable for MyPanel {
    fn focus_handle(&self, _cx: &App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Panel for MyPanel {
    fn panel_name(&self) -> &'static str {
        "MyPanel"
    }

    fn title(&self, _window: &Window, _cx: &App) -> gpui::AnyElement {
        self.title.clone().into_any_element()
    }

    fn closable(&self, _cx: &App) -> bool {
        true
    }

    fn zoomable(&self, _cx: &App) -> Option<gpui_component::dock::PanelControl> {
        Some(gpui_component::dock::PanelControl::Both)
    }

    fn visible(&self, _cx: &App) -> bool {
        true
    }

    fn set_active(&mut self, _active: bool, _window: &mut Window, _cx: &mut App) {}

    fn set_zoomed(&mut self, _zoomed: bool, _window: &mut Window, _cx: &mut App) {}
}

/// A panel for displaying a code editor.
pub struct EditorPanel {
    editor: Entity<InputState>,
    focus_handle: FocusHandle,
}

impl EditorPanel {
    pub fn new(editor: Entity<InputState>, cx: &mut Context<Self>) -> Self {
        Self {
            editor,
            focus_handle: cx.focus_handle(),
        }
    }
}

impl Render for EditorPanel {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        Input::new(&self.editor).h_full() // Full height
    }
}

impl EventEmitter<PanelEvent> for EditorPanel {}

impl Focusable for EditorPanel {
    fn focus_handle(&self, _cx: &App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Panel for EditorPanel {
    fn panel_name(&self) -> &'static str {
        "EditorPanel"
    }

    fn title(&self, _window: &Window, _cx: &App) -> gpui::AnyElement {
        SharedString::from("Editor").into_any_element()
    }

    fn closable(&self, _cx: &App) -> bool {
        true
    }

    fn zoomable(&self, _cx: &App) -> Option<gpui_component::dock::PanelControl> {
        Some(gpui_component::dock::PanelControl::Both)
    }

    fn visible(&self, _cx: &App) -> bool {
        true
    }

    fn set_active(&mut self, _active: bool, _window: &mut Window, _cx: &mut App) {}

    fn set_zoomed(&mut self, _zoomed: bool, _window: &mut Window, _cx: &mut App) {}
}

fn main() {
    // 新しいGPUIアプリケーションインスタンスを作成します。
    let app = Application::new();

    // アプリケーションを実行します。
    app.run(move |cx| {
        // GPUIコンポーネントの機能を使用する前に、これを呼び出す必要があります。
        gpui_component::init(cx);

        // メインウィンドウを開く非同期タスクをスポーンします。
        cx.spawn(async move |cx| {
            // デフォルトオプションで新しいウィンドウを開きます。
            cx.open_window(WindowOptions::default(), |window_ctx, cx| {
                let dock_area_entity =
                    cx.new(|cx| DockArea::new("main_dock_area", None, window_ctx, cx));

                dock_area_entity.update(cx, |dock_area, cx| {
                    let panel1 = cx.new(|cx| MyPanel::new("Panel 1", cx));
                    let panel2 = cx.new(|cx| MyPanel::new("Panel 2", cx));
                    let panel3 = cx.new(|cx| MyPanel::new("Panel 3", cx));

                    let code_editor_state = cx.new(|cx| {
                        InputState::new(window_ctx, cx)
                            .code_editor("rust") // Language for syntax highlighting
                            .line_number(true) // Show line numbers
                            .searchable(true) // Enable search functionality
                            .default_value("fn main() {\n    println!(\"Hello, world!\");\n}")
                    });
                    let editor_panel = cx.new(|cx| EditorPanel::new(code_editor_state, cx));

                    dock_area.set_left_dock(
                        DockItem::tabs(
                            vec![Arc::new(panel1)],
                            None,
                            &dock_area_entity.downgrade(),
                            window_ctx,
                            cx,
                        ),
                        None,
                        true,
                        window_ctx,
                        cx,
                    );

                    dock_area.set_bottom_dock(
                        DockItem::tabs(
                            vec![Arc::new(panel2)],
                            None,
                            &dock_area_entity.downgrade(),
                            window_ctx,
                            cx,
                        ),
                        None,
                        true,
                        window_ctx,
                        cx,
                    );

                    dock_area.set_right_dock(
                        DockItem::tabs(
                            vec![Arc::new(panel3)],
                            None,
                            &dock_area_entity.downgrade(),
                            window_ctx,
                            cx,
                        ),
                        None,
                        true,
                        window_ctx,
                        cx,
                    );

                    dock_area.set_center(DockItem::panel(Arc::new(editor_panel)), window_ctx, cx);
                });

                // 新しい`Workspace`ビューを作成します。
                let view = cx.new(|_cx| Workspace {
                    dock_area: dock_area_entity,
                });
                // ウィンドウの最初のレベルはRootコンポーネントである必要があります。
                cx.new(|cx| Root::new(view.into(), window_ctx, cx))
            })?;
            Ok::<_, anyhow::Error>(())
        })
        .detach(); // スポーンされたタスクをデタッチし、独立して実行できるようにします。
    });
}
