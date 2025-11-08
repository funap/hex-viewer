//! このモジュールは、アプリケーションのエントリポイントと`HelloWorld` UIコンポーネントを定義します。
//!
//! `gpui`と`gpui_component`クレートを使用して、タブとボタンを持つシンプルなアプリケーションを
//! 作成する方法を示しています。

use gpui::{
    AnyWeakView, AppContext, Application, Context, IntoElement, ParentElement, Render, Styled,
    Window, WindowOptions, div,
};
use gpui_component::tab::{Tab, TabBar};
use gpui_component::{Root, StyledExt, button::*};
use std::rc::Rc;

/// `HelloWorld`コンポーネントの主要なアプリケーション状態。
///
/// この構造体は、現在選択されているタブの状態を保持します。
pub struct HelloWorld {
    selected_tab: usize,
    view_handle: Option<Rc<AnyWeakView>>,
}

const ACCOUNT_TAB_INDEX: usize = 0;
const PROFILE_TAB_INDEX: usize = 1;
const SETTINGS_TAB_INDEX: usize = 2;

impl Render for HelloWorld {
    /// タブバーと、選択されたタブに基づいて内容が変化するコンテンツエリア、およびボタンを含む
    /// `HelloWorld`コンポーネントをレンダリングします。
    fn render(&mut self, _: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let view = self.view_handle.clone().unwrap();
        div()
            .v_flex()
            .gap_2()
            .size_full()
            .items_center()
            .justify_center()
            .child(
                // 異なるセクション間のナビゲーションのためのTabBarコンポーネント。
                TabBar::new("tabs")
                    .selected_index(self.selected_tab)
                    .on_click(move |idx, _, cx| {
                        view.upgrade().unwrap().downcast::<Self>().unwrap().update(
                            cx,
                            |this, cx| {
                                this.selected_tab = *idx;
                                cx.notify();
                            },
                        );
                    })
                    .child(Tab::new("Account"))
                    .child(Tab::new("Profile"))
                    .child(Tab::new("Settings")),
            )
            .child(
                // `selected_tab`の状態に基づいて動的にレンダリングされるコンテンツ。
                match self.selected_tab {
                    ACCOUNT_TAB_INDEX => div().child("Account Content"),
                    PROFILE_TAB_INDEX => div().child("Profile Content"),
                    SETTINGS_TAB_INDEX => div().child("Settings Content"),
                    _ => div().child("Unknown Tab"),
                },
            )
            .child(
                // シンプルなボタンコンポーネント。
                Button::new("ok")
                    .primary()
                    .label("Let's Go!")
                    .on_click(|_, _, _| println!("Clicked!")), // ボタンクリックのイベントハンドラ。
            )
    }
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
            cx.open_window(WindowOptions::default(), |window, cx| {
                // 初期選択タブを0に設定して、新しい`HelloWorld`ビューを作成します。
                let view = cx.new(|_cx| HelloWorld {
                    selected_tab: ACCOUNT_TAB_INDEX,
                    view_handle: None,
                });
                view.update(cx, |this, _cx| {
                    this.view_handle = Some(Rc::new(view.downgrade().into()));
                });
                // ウィンドウの最初のレベルはRootコンポーネントである必要があります。
                cx.new(|cx| Root::new(view.into(), window, cx))
            })?;

            Ok::<_, anyhow::Error>(())
        })
        .detach(); // スポーンされたタスクをデタッチし、独立して実行できるようにします。
    });
}
