use crate::actions::{SearchNext, SearchPrev, ToggleSearch};
use crate::core::document::Document;
use crate::core::editor::Editor;
use gpui::prelude::*;
use gpui::*;
use gpui::{ScrollWheelEvent, WeakEntity};
use gpui_component::scroll::*;
use gpui_component::{ActiveTheme, PixelsExt};
use std::cmp;
use std::ops::Range;
use std::sync::Arc;
use std::sync::RwLock;

#[allow(dead_code)]
pub enum HexViewEvent {
    Scrolled(usize),
    SelectionChanged { start: Option<usize>, end: Option<usize> },
    CursorMoved(usize),
}

actions!(
    hex_view,
    [
        MoveLeft,
        MoveRight,
        MoveUp,
        MoveDown,
        SelectLeft,
        SelectRight,
        SelectUp,
        SelectDown,
        SelectAll,
        PageUp,
        PageDown,
        Home,
        End,
        SelectPageUp,
        SelectPageDown,
        SelectHome,
        SelectEnd,
        TriggerSearch,
        TriggerSearchNext,
        TriggerSearchPrev
    ]
);

const CONTEXT: &str = "HexView";

// HexView layout constants
pub const HEADER_HEIGHT: f32 = 32.0;
pub const ROW_HEIGHT: f32 = 24.0;
pub const OFFSET_WIDTH: f32 = 80.0;
pub const HEX_BYTE_WIDTH: f32 = 22.0;
pub const HEX_GAP: f32 = 4.0;
pub const SECTION_GAP: f32 = 16.0;
pub const OFFSET_X_START: f32 = 4.0;
pub const SELECTION_PADDING: f32 = 2.0;
pub use crate::core::editor::BYTES_PER_ROW;
// pub const FONT_FAMILY: &str = "Menlo"; // Removed in favor of global Appearance

pub fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("left", MoveLeft, Some(CONTEXT)),
        KeyBinding::new("right", MoveRight, Some(CONTEXT)),
        KeyBinding::new("up", MoveUp, Some(CONTEXT)),
        KeyBinding::new("down", MoveDown, Some(CONTEXT)),
        KeyBinding::new("shift-left", SelectLeft, Some(CONTEXT)),
        KeyBinding::new("shift-right", SelectRight, Some(CONTEXT)),
        KeyBinding::new("shift-up", SelectUp, Some(CONTEXT)),
        KeyBinding::new("shift-down", SelectDown, Some(CONTEXT)),
        KeyBinding::new("cmd-a", SelectAll, Some(CONTEXT)),
        KeyBinding::new("pageup", PageUp, Some(CONTEXT)),
        KeyBinding::new("pagedown", PageDown, Some(CONTEXT)),
        KeyBinding::new("home", Home, Some(CONTEXT)),
        KeyBinding::new("end", End, Some(CONTEXT)),
        KeyBinding::new("shift-pageup", SelectPageUp, Some(CONTEXT)),
        KeyBinding::new("shift-pagedown", SelectPageDown, Some(CONTEXT)),
        KeyBinding::new("shift-home", SelectHome, Some(CONTEXT)),
        KeyBinding::new("shift-end", SelectEnd, Some(CONTEXT)),
        // Vi-like navigation
        KeyBinding::new("h", MoveLeft, Some(CONTEXT)),
        KeyBinding::new("l", MoveRight, Some(CONTEXT)),
        KeyBinding::new("k", MoveUp, Some(CONTEXT)),
        KeyBinding::new("j", MoveDown, Some(CONTEXT)),
        KeyBinding::new("shift-h", SelectLeft, Some(CONTEXT)),
        KeyBinding::new("shift-l", SelectRight, Some(CONTEXT)),
        KeyBinding::new("shift-k", SelectUp, Some(CONTEXT)),
        KeyBinding::new("shift-j", SelectDown, Some(CONTEXT)),
        // Vi-like search commands (only when HexView is focused)
        KeyBinding::new("/", TriggerSearch, Some(CONTEXT)),
        KeyBinding::new("n", TriggerSearchNext, Some(CONTEXT)),
        KeyBinding::new("shift-n", TriggerSearchPrev, Some(CONTEXT)),
        KeyBinding::new("ctrl-f", ToggleSearch, Some(CONTEXT)),
        KeyBinding::new("cmd-f", ToggleSearch, Some(CONTEXT)),
        KeyBinding::new("f3", SearchNext, Some(CONTEXT)),
        KeyBinding::new("ctrl-g", SearchNext, Some(CONTEXT)),
        KeyBinding::new("cmd-g", SearchNext, Some(CONTEXT)),
        KeyBinding::new("shift-f3", SearchPrev, Some(CONTEXT)),
        KeyBinding::new("ctrl-shift-g", SearchPrev, Some(CONTEXT)),
        KeyBinding::new("cmd-shift-g", SearchPrev, Some(CONTEXT)),
    ]);
}

#[allow(dead_code)]
pub struct HexView {
    editor: Entity<Editor>,
    focus_handle: FocusHandle,
    is_dragging: bool,
    last_bounds: Option<Bounds<Pixels>>,
    scroll_offset: usize,
    scroll_handle: ScrollHandle,
    highlights: Vec<(Range<usize>, Hsla)>,
    show_offset: bool,
    show_header: bool,
    show_ascii: bool,
    font_family_prop: SharedString,
    font_size_prop: Pixels,
}

impl EventEmitter<HexViewEvent> for HexView {}

#[allow(dead_code)]
impl HexView {
    pub fn new(editor: Entity<Editor>, cx: &mut Context<Self>) -> Self {
        cx.observe(&editor, |this, _, cx| {
            this.ensure_cursor_visible(cx);
            cx.notify();
        })
        .detach();

        Self {
            editor,
            focus_handle: cx.focus_handle(),
            is_dragging: false,
            last_bounds: None,
            scroll_offset: 0,
            scroll_handle: ScrollHandle::new(),
            highlights: Vec::new(),
            show_offset: true,
            show_header: true,
            show_ascii: true,
            font_family_prop: "Zed Sans Mono".into(),
            font_size_prop: px(14.0),
        }
    }

    pub fn font_family(mut self, font_family: impl Into<SharedString>) -> Self {
        self.font_family_prop = font_family.into();
        self
    }

    pub fn font_size(mut self, font_size: impl Into<Pixels>) -> Self {
        self.font_size_prop = font_size.into();
        self
    }

    pub fn set_font_family(&mut self, font_family: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.font_family_prop = font_family.into();
        cx.notify();
    }

    pub fn set_font_size(&mut self, font_size: impl Into<Pixels>, cx: &mut Context<Self>) {
        self.font_size_prop = font_size.into();
        cx.notify();
    }

    pub fn with_offset(mut self, show: bool) -> Self {
        self.show_offset = show;
        self
    }

    pub fn with_header(mut self, show: bool) -> Self {
        self.show_header = show;
        self
    }

    pub fn with_ascii(mut self, show: bool) -> Self {
        self.show_ascii = show;
        self
    }

    pub fn set_show_offset(&mut self, show: bool, cx: &mut Context<Self>) {
        self.show_offset = show;
        cx.notify();
    }

    pub fn set_show_header(&mut self, show: bool, cx: &mut Context<Self>) {
        self.show_header = show;
        cx.notify();
    }

    pub fn set_show_ascii(&mut self, show: bool, cx: &mut Context<Self>) {
        self.show_ascii = show;
        cx.notify();
    }

    pub fn cursor_offset(&self, cx: &App) -> usize {
        self.editor.read(cx).cursor_offset
    }

    pub fn selection_range(&self, cx: &App) -> Option<Range<usize>> {
        self.editor.read(cx).selection_range()
    }

    pub fn set_highlights(&mut self, highlights: Vec<(Range<usize>, Hsla)>, cx: &mut Context<Self>) {
        self.highlights = highlights;
        cx.notify();
    }

    pub fn set_highlight_ranges(&mut self, ranges: Vec<Range<usize>>, cx: &mut Context<Self>) {
        let theme = cx.theme();
        let highlight_color = theme.accent;
        self.highlights = ranges.into_iter().map(|range| (range, highlight_color)).collect();
        cx.notify();
    }

    pub fn scroll_to_offset(&mut self, byte_offset: usize, cx: &mut Context<Self>) {
        let row = byte_offset / BYTES_PER_ROW;
        self.set_scroll_offset(row, cx);
    }

    /// Returns the byte range of the current viewport (visible area).
    /// Returns (start_byte, end_byte) where end_byte is exclusive.
    pub fn viewport_byte_range(&self, cx: &App) -> (usize, usize) {
        let start_byte = self.scroll_offset * BYTES_PER_ROW;
        let visible_rows = self.get_visible_rows();
        let end_byte = start_byte + (visible_rows * BYTES_PER_ROW);
        let end_byte = end_byte.min(self.editor.read(cx).total_size());
        (start_byte, end_byte)
    }

    pub fn set_scroll_offset(&mut self, offset: usize, cx: &mut Context<Self>) {
        let row_height = px(ROW_HEIGHT);
        let total_rows = (self.editor.read(cx).total_size() + BYTES_PER_ROW - 1) / BYTES_PER_ROW;
        let max_offset = total_rows.saturating_sub(1);
        let new_offset = offset.min(max_offset);

        if self.scroll_offset == new_offset {
            return;
        }

        self.scroll_offset = new_offset;
        self.scroll_handle.set_offset(point(px(0.), -(self.scroll_offset as f32 * row_height)));
        cx.notify();
        cx.emit(HexViewEvent::Scrolled(self.scroll_offset));
    }

    pub fn set_cursor_offset(&mut self, offset: usize, cx: &mut Context<Self>) {
        self.editor.update(cx, |editor, _| {
            editor.set_cursor_offset(offset);
        });
        self.ensure_cursor_visible(cx);
    }

    fn ensure_cursor_visible(&mut self, cx: &mut Context<Self>) {
        let bounds = match self.last_bounds {
            Some(b) => b,
            None => return,
        };

        let header_height = px(HEADER_HEIGHT);
        let row_height = px(ROW_HEIGHT);
        let visible_height = bounds.size.height - header_height;
        let visible_rows = (visible_height / row_height).floor() as usize;

        let cursor_offset = self.editor.read(cx).cursor_offset;
        let cursor_row = cursor_offset / BYTES_PER_ROW;

        if cursor_row < self.scroll_offset {
            self.scroll_offset = cursor_row;
        } else if cursor_row >= self.scroll_offset + visible_rows {
            self.scroll_offset = cursor_row.saturating_sub(visible_rows - 1);
        }
        self.scroll_handle.set_offset(point(px(0.), -(self.scroll_offset as f32 * row_height)));
        cx.emit(HexViewEvent::Scrolled(self.scroll_offset));
    }

    fn byte_pos_from_point(&self, point: Point<Pixels>, cx: &App) -> Option<usize> {
        let bounds = self.last_bounds?;
        let header_height = if self.show_header { px(HEADER_HEIGHT) } else { px(0.) };
        let row_height = px(ROW_HEIGHT);
        let offset_width = if self.show_offset { px(OFFSET_WIDTH) } else { px(0.) };
        let hex_start_x = bounds.left() + px(OFFSET_X_START) + offset_width + px(SECTION_GAP);

        let hex_byte_width = px(HEX_BYTE_WIDTH);
        let hex_gap = px(HEX_GAP);

        if point.y < bounds.top() + header_height {
            return None;
        }

        let y_offset = point.y - bounds.top() - header_height;
        let visible_row = (y_offset / row_height).floor() as usize;

        // Add scroll offset to get the actual row in the buffer
        let row = visible_row + self.scroll_offset;

        let x_offset = point.x - hex_start_x;
        if x_offset < px(0.) {
            return None;
        }

        let byte_in_row = (x_offset / (hex_byte_width + hex_gap)).floor() as usize;
        if byte_in_row >= BYTES_PER_ROW {
            return None;
        }

        let byte_pos = row * BYTES_PER_ROW + byte_in_row;
        if byte_pos >= self.editor.read(cx).total_size() {
            return None;
        }

        Some(byte_pos)
    }

    fn on_mouse_down(&mut self, event: &MouseDownEvent, _window: &mut Window, cx: &mut Context<Self>) {
        if let Some(byte_pos) = self.byte_pos_from_point(event.position, cx) {
            self.is_dragging = true;
            self.editor.update(cx, |editor, _| {
                editor.start_drag(byte_pos);
            });
            cx.notify();
            cx.emit(HexViewEvent::SelectionChanged {
                start: Some(byte_pos),
                end: Some(byte_pos),
            });
        }
    }

    fn byte_pos_from_point_clamped(&self, point: Point<Pixels>, cx: &App) -> Option<usize> {
        let bounds = self.last_bounds?;
        let header_height = if self.show_header { px(HEADER_HEIGHT) } else { px(0.) };
        let row_height = px(ROW_HEIGHT);
        let offset_width = if self.show_offset { px(OFFSET_WIDTH) } else { px(0.) };
        let hex_start_x = bounds.left() + px(OFFSET_X_START) + offset_width + px(SECTION_GAP);

        let hex_byte_width = px(HEX_BYTE_WIDTH);
        let hex_gap = px(HEX_GAP);

        let y_offset = point.y - bounds.top() - header_height;
        let visible_row = (y_offset / row_height).floor() as i32;

        // Allow selecting above/below visible area
        let row = visible_row + self.scroll_offset as i32;
        let row = row.max(0) as usize;

        let x_offset = point.x - hex_start_x;
        let byte_in_row = (x_offset / (hex_byte_width + hex_gap)).floor() as i32;
        let byte_in_row = byte_in_row.max(0).min((BYTES_PER_ROW - 1) as i32) as usize;

        let byte_pos = row * BYTES_PER_ROW + byte_in_row;
        Some(byte_pos.min(self.editor.read(cx).total_size().saturating_sub(1)))
    }

    const SCROLL_TRIGGER_MARGIN: f32 = 32.0;

    fn on_mouse_move(&mut self, event: &MouseMoveEvent, _window: &mut Window, cx: &mut Context<Self>) {
        // Sync scroll handle to scroll offset if changed by scrollbar drag
        let row_height = px(ROW_HEIGHT);
        let handle_y = self.scroll_handle.offset().y;
        let handle_row = ((-handle_y).max(px(0.)) / row_height).round() as usize;
        let total_rows = (self.editor.read(cx).total_size() + BYTES_PER_ROW - 1) / BYTES_PER_ROW;
        if handle_row != self.scroll_offset {
            self.scroll_offset = handle_row.min(total_rows.saturating_sub(1));
            cx.notify();
            cx.emit(HexViewEvent::Scrolled(self.scroll_offset));
        }

        if self.is_dragging {
            if let Some(bounds) = self.last_bounds {
                let header_height = if self.show_header { px(HEADER_HEIGHT) } else { px(0.) };
                let top_edge = bounds.top() + header_height;

                let bottom_edge = bounds.bottom();
                let margin = px(Self::SCROLL_TRIGGER_MARGIN);

                // Auto-scroll if dragging near/outside bounds
                if event.position.y < top_edge + margin {
                    if self.scroll_offset > 0 {
                        self.scroll_offset -= 1;
                        self.scroll_handle.set_offset(point(px(0.), -(self.scroll_offset as f32 * row_height)));
                        cx.notify();
                        cx.emit(HexViewEvent::Scrolled(self.scroll_offset));
                    }
                } else if event.position.y > bottom_edge - margin {
                    let visible_rows = ((bounds.size.height - header_height) / row_height).floor() as usize;
                    let max_scroll = total_rows.saturating_sub(visible_rows);
                    if self.scroll_offset < max_scroll {
                        self.scroll_offset += 1;
                        self.scroll_handle.set_offset(point(px(0.), -(self.scroll_offset as f32 * row_height)));
                        cx.notify();
                        cx.emit(HexViewEvent::Scrolled(self.scroll_offset));
                    }
                }
            }

            if let Some(byte_pos) = self.byte_pos_from_point_clamped(event.position, cx) {
                self.editor.update(cx, |editor, _| {
                    editor.continue_drag(byte_pos);
                });
                cx.notify();
                let editor = self.editor.read(cx);
                cx.emit(HexViewEvent::SelectionChanged {
                    start: editor.selection_start,
                    end: editor.selection_end,
                });
            }
        }
    }

    fn on_mouse_up(&mut self, _event: &MouseUpEvent, _window: &mut Window, cx: &mut Context<Self>) {
        self.is_dragging = false;

        // Sync scroll handle on mouse up as well
        let row_height = px(ROW_HEIGHT);
        let handle_y = self.scroll_handle.offset().y;
        let handle_row = ((-handle_y).max(px(0.)) / row_height).round() as usize;
        let total_rows = (self.editor.read(cx).total_size() + BYTES_PER_ROW - 1) / BYTES_PER_ROW;
        if handle_row != self.scroll_offset {
            self.scroll_offset = handle_row.min(total_rows.saturating_sub(1));
            cx.notify();
            cx.emit(HexViewEvent::Scrolled(self.scroll_offset));
        } else {
            cx.notify();
        }
    }

    fn on_scroll_wheel(&mut self, event: &ScrollWheelEvent, _window: &mut Window, cx: &mut Context<Self>) {
        let row_height = px(ROW_HEIGHT);
        let total_rows = (self.editor.read(cx).total_size() + BYTES_PER_ROW - 1) / BYTES_PER_ROW;
        let max_offset = total_rows.saturating_sub(1).max(0) as i32;

        let delta_y = event.delta.pixel_delta(row_height).y.as_f32() as i32;
        let new_scroll_offset = self.scroll_offset as i32 - delta_y;

        self.scroll_offset = cmp::max(0, cmp::min(new_scroll_offset, max_offset)) as usize;
        self.scroll_handle.set_offset(point(px(0.), -(self.scroll_offset as f32 * row_height)));
        cx.notify();
        cx.emit(HexViewEvent::Scrolled(self.scroll_offset));
    }

    fn move_left(&mut self, _: &MoveLeft, _window: &mut Window, cx: &mut Context<Self>) {
        self.editor.update(cx, |editor, _| {
            editor.move_left();
        });
        self.ensure_cursor_visible(cx);
    }

    fn move_right(&mut self, _: &MoveRight, _window: &mut Window, cx: &mut Context<Self>) {
        self.editor.update(cx, |editor, _| {
            editor.move_right();
        });
        self.ensure_cursor_visible(cx);
    }

    fn move_up(&mut self, _: &MoveUp, _window: &mut Window, cx: &mut Context<Self>) {
        self.editor.update(cx, |editor, _| {
            editor.move_up();
        });
        self.ensure_cursor_visible(cx);
    }

    fn move_down(&mut self, _: &MoveDown, _window: &mut Window, cx: &mut Context<Self>) {
        self.editor.update(cx, |editor, _| {
            editor.move_down();
        });
        self.ensure_cursor_visible(cx);
    }

    fn select_left(&mut self, _: &SelectLeft, _window: &mut Window, cx: &mut Context<Self>) {
        self.editor.update(cx, |editor, _| {
            editor.select_left();
        });
        self.ensure_cursor_visible(cx);
    }

    fn select_right(&mut self, _: &SelectRight, _window: &mut Window, cx: &mut Context<Self>) {
        self.editor.update(cx, |editor, _| {
            editor.select_right();
        });
        self.ensure_cursor_visible(cx);
    }

    fn select_up(&mut self, _: &SelectUp, _window: &mut Window, cx: &mut Context<Self>) {
        self.editor.update(cx, |editor, _| {
            editor.select_up();
        });
        self.ensure_cursor_visible(cx);
    }

    fn select_down(&mut self, _: &SelectDown, _window: &mut Window, cx: &mut Context<Self>) {
        self.editor.update(cx, |editor, _| {
            editor.select_down();
        });
        self.ensure_cursor_visible(cx);
    }

    fn select_all(&mut self, _: &SelectAll, _window: &mut Window, cx: &mut Context<Self>) {
        self.editor.update(cx, |editor, _| editor.select_all());
        cx.notify();
    }

    fn get_visible_rows(&self) -> usize {
        if let Some(bounds) = self.last_bounds {
            let header_height = px(HEADER_HEIGHT);
            let row_height = px(ROW_HEIGHT);
            let visible_height = bounds.size.height - header_height;
            (visible_height / row_height).floor() as usize
        } else {
            10 // Default fallback
        }
    }

    fn page_up(&mut self, _: &PageUp, _window: &mut Window, cx: &mut Context<Self>) {
        let visible_rows = self.get_visible_rows();
        self.editor.update(cx, |editor, _| editor.page_up(visible_rows));
        self.ensure_cursor_visible(cx);
        cx.notify();
        let cursor_offset = self.editor.read(cx).cursor_offset;
        cx.emit(HexViewEvent::CursorMoved(cursor_offset));
    }

    fn page_down(&mut self, _: &PageDown, _window: &mut Window, cx: &mut Context<Self>) {
        let visible_rows = self.get_visible_rows();
        self.editor.update(cx, |editor, _| editor.page_down(visible_rows));
        self.ensure_cursor_visible(cx);
        cx.notify();
        let cursor_offset = self.editor.read(cx).cursor_offset;
        cx.emit(HexViewEvent::CursorMoved(cursor_offset));
    }

    fn home(&mut self, _: &Home, _window: &mut Window, cx: &mut Context<Self>) {
        self.editor.update(cx, |editor, _| editor.home());
        self.ensure_cursor_visible(cx);
        cx.notify();
        let cursor_offset = self.editor.read(cx).cursor_offset;
        cx.emit(HexViewEvent::CursorMoved(cursor_offset));
    }

    fn end(&mut self, _: &End, _window: &mut Window, cx: &mut Context<Self>) {
        self.editor.update(cx, |editor, _| editor.end());
        self.ensure_cursor_visible(cx);
        cx.notify();
        let cursor_offset = self.editor.read(cx).cursor_offset;
        cx.emit(HexViewEvent::CursorMoved(cursor_offset));
    }

    fn select_page_up(&mut self, _: &SelectPageUp, _window: &mut Window, cx: &mut Context<Self>) {
        let visible_rows = self.get_visible_rows();
        self.editor.update(cx, |editor, _| editor.select_page_up(visible_rows));
        self.ensure_cursor_visible(cx);
        cx.notify();
        let editor = self.editor.read(cx);
        cx.emit(HexViewEvent::SelectionChanged {
            start: editor.selection_start,
            end: editor.selection_end,
        });
    }

    fn select_page_down(&mut self, _: &SelectPageDown, _window: &mut Window, cx: &mut Context<Self>) {
        let visible_rows = self.get_visible_rows();
        self.editor.update(cx, |editor, _| editor.select_page_down(visible_rows));
        self.ensure_cursor_visible(cx);
        cx.notify();
        let editor = self.editor.read(cx);
        cx.emit(HexViewEvent::SelectionChanged {
            start: editor.selection_start,
            end: editor.selection_end,
        });
    }

    fn select_home(&mut self, _: &SelectHome, _window: &mut Window, cx: &mut Context<Self>) {
        self.editor.update(cx, |editor, _| editor.select_home());
        self.ensure_cursor_visible(cx);
        cx.notify();
        let editor = self.editor.read(cx);
        cx.emit(HexViewEvent::SelectionChanged {
            start: editor.selection_start,
            end: editor.selection_end,
        });
    }

    fn select_end(&mut self, _: &SelectEnd, _window: &mut Window, cx: &mut Context<Self>) {
        self.editor.update(cx, |editor, _| editor.select_end());
        self.ensure_cursor_visible(cx);
        cx.notify();
        let editor = self.editor.read(cx);
        cx.emit(HexViewEvent::SelectionChanged {
            start: editor.selection_start,
            end: editor.selection_end,
        });
    }

    fn trigger_search(&mut self, _: &TriggerSearch, window: &mut Window, cx: &mut Context<Self>) {
        window.dispatch_action(ToggleSearch.boxed_clone(), cx);
    }

    fn trigger_search_next(&mut self, _: &TriggerSearchNext, window: &mut Window, cx: &mut Context<Self>) {
        window.dispatch_action(SearchNext.boxed_clone(), cx);
    }

    fn trigger_search_prev(&mut self, _: &TriggerSearchPrev, window: &mut Window, cx: &mut Context<Self>) {
        window.dispatch_action(SearchPrev.boxed_clone(), cx);
    }
}

impl Focusable for HexView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for HexView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let editor = self.editor.read(cx);
        let document = editor.document.clone();
        let total_size = editor.total_size();
        let total_rows = ((total_size + BYTES_PER_ROW - 1) / BYTES_PER_ROW).max(1);

        let visible_rows = self.get_visible_rows();
        let extra_scroll_rows = visible_rows.saturating_sub(1);
        let header_height = if self.show_header { px(HEADER_HEIGHT) } else { px(0.) };
        let row_height = px(ROW_HEIGHT);
        let total_height = header_height + row_height * (total_rows + extra_scroll_rows) as f32;

        let handle_y = self.scroll_handle.offset().y;
        let handle_row = ((-handle_y).max(px(0.)) / row_height).round() as usize;
        if handle_row != self.scroll_offset {
            self.scroll_offset = handle_row.min(total_rows.saturating_sub(1));
            cx.emit(HexViewEvent::Scrolled(self.scroll_offset));
        }

        let (selection_start, selection_end, cursor_offset) = {
            let editor = self.editor.read(cx);
            (editor.selection_start, editor.selection_end, editor.cursor_offset)
        };

        div()
            .flex()
            .flex_col()
            .bg(cx.theme().background)
            .font_family(self.font_family_prop.clone())
            .size_full()
            .key_context(CONTEXT)
            .track_focus(&self.focus_handle(cx))
            .on_action(cx.listener(Self::move_left))
            .on_action(cx.listener(Self::move_right))
            .on_action(cx.listener(Self::move_up))
            .on_action(cx.listener(Self::move_down))
            .on_action(cx.listener(Self::select_left))
            .on_action(cx.listener(Self::select_right))
            .on_action(cx.listener(Self::select_up))
            .on_action(cx.listener(Self::select_down))
            .on_action(cx.listener(Self::select_all))
            .on_action(cx.listener(Self::page_up))
            .on_action(cx.listener(Self::page_down))
            .on_action(cx.listener(Self::home))
            .on_action(cx.listener(Self::end))
            .on_action(cx.listener(Self::select_page_up))
            .on_action(cx.listener(Self::select_page_down))
            .on_action(cx.listener(Self::select_home))
            .on_action(cx.listener(Self::select_end))
            .on_action(cx.listener(Self::trigger_search))
            .on_scroll_wheel(cx.listener(Self::on_scroll_wheel))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::on_mouse_down))
            .on_mouse_move(cx.listener(Self::on_mouse_move))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .child(HexViewElement {
                view: cx.entity().downgrade(),
                document,
                selection_start,
                selection_end,
                cursor_offset,
                scroll_offset: self.scroll_offset,
                focus_handle: self.focus_handle.clone(),
                highlights: self.highlights.clone(),
                show_offset: self.show_offset,
                show_header: self.show_header,
                show_ascii: self.show_ascii,
                font_family: self.font_family_prop.clone(),
                font_size: self.font_size_prop,
            })
            .child(
                div().absolute().top_0().right_0().bottom_0().w_4().child(
                    Scrollbar::vertical(&self.scroll_handle)
                        .axis(ScrollbarAxis::Vertical)
                        .scroll_size(size(px(0.), total_height)),
                ),
            )
    }
}

struct HexViewElement {
    view: WeakEntity<HexView>,
    document: Arc<RwLock<Document>>,
    selection_start: Option<usize>,
    selection_end: Option<usize>,
    cursor_offset: usize,
    scroll_offset: usize,
    focus_handle: FocusHandle,
    highlights: Vec<(Range<usize>, Hsla)>,
    show_offset: bool,
    show_header: bool,
    show_ascii: bool,
    font_family: SharedString,
    font_size: Pixels,
}

struct PrepaintState {
    data_lines: Vec<DataLine>,
    selection_quads: Vec<PaintQuad>,
    cursor: Option<PaintQuad>,
    header: HeaderParts,
}

struct HeaderParts {
    offset: ShapedLine,
    hex_bytes: Vec<ShapedLine>,
    ascii: ShapedLine,
}

struct DataLine {
    offset_line: ShapedLine,
    hex_lines: Vec<ShapedLine>,
    ascii_line: ShapedLine,
}

impl IntoElement for HexViewElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for HexViewElement {
    type RequestLayoutState = ();
    type PrepaintState = PrepaintState;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        // Ensure at least one line is shown, even for empty buffer
        let buffer_len = self.document.read().unwrap().buffer.len();
        let line_count = ((buffer_len + BYTES_PER_ROW - 1) / BYTES_PER_ROW).max(1);
        let header_height = if self.show_header { px(HEADER_HEIGHT) } else { px(0.) };

        let row_height = px(ROW_HEIGHT);
        let total_height = header_height + row_height * line_count as f32;

        let mut style = Style::default();
        style.size.width = relative(1.).into();
        style.size.height = total_height.into();

        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let document = self.document.read().unwrap();
        let buffer = &document.buffer;
        let selection_start = self.selection_start;
        let selection_end = self.selection_end;
        let highlights = &self.highlights;
        let text_style = TextStyle {
            font_family: self.font_family.clone(),
            font_size: gpui::AbsoluteLength::Pixels(self.font_size),
            ..window.text_style()
        };
        let font_size = self.font_size;

        let theme = cx.theme();
        let offset_color = theme.muted_foreground;
        let hex_byte_color = theme.foreground;
        let hex_null_color = theme.muted_foreground;
        let ascii_printable_color = theme.foreground;
        let ascii_non_printable_color = theme.muted_foreground;
        let selection_bg_color = theme.secondary;

        // Ensure at least one line is shown, even for empty buffer
        let line_count = ((buffer.len() + BYTES_PER_ROW - 1) / BYTES_PER_ROW).max(1);
        let header_height = if self.show_header { px(HEADER_HEIGHT) } else { px(0.) };

        let row_height = px(ROW_HEIGHT);

        let scroll_offset = self.scroll_offset;
        let visible_height = bounds.size.height - header_height;
        let visible_rows = (visible_height / row_height).ceil() as usize + 1;
        let start_row = scroll_offset;
        let end_row = (scroll_offset + visible_rows).min(line_count);

        let mut data_lines = Vec::new();
        let mut selection_quads = Vec::new();

        let offset_width = if self.show_offset { px(OFFSET_WIDTH) } else { px(0.) };
        let hex_start_x = bounds.left() + px(OFFSET_X_START) + offset_width + px(SECTION_GAP);

        let hex_byte_width = px(HEX_BYTE_WIDTH);
        let hex_gap = px(HEX_GAP);

        let (min_sel, max_sel) = if let (Some(start), Some(end)) = (selection_start, selection_end) {
            if start <= end { (start, end) } else { (end, start) }
        } else {
            (usize::MAX, usize::MIN)
        };

        for i in start_row..end_row {
            let offset = i * 16;
            let chunk = buffer.get_range(offset, 16);
            let row_index = i - start_row;
            let y_pos = bounds.top() + header_height + row_height * row_index as f32;

            // Draw highlights
            for (range, color) in highlights {
                let range_start = range.start;
                let range_end = range.end;

                // Check if this line has any overlap with the highlight range
                let line_start = offset;
                let line_end = offset + 16; // Exclusive end for the line

                if line_start < range_end && line_end > range_start {
                    let start_in_line = cmp::max(line_start, range_start) - line_start;
                    let end_in_line = cmp::min(line_end, range_end) - line_start;

                    let x_start = hex_start_x + (hex_byte_width + hex_gap) * start_in_line as f32;
                    let x_end = hex_start_x + (hex_byte_width + hex_gap) * end_in_line as f32 - hex_gap;
                    // Adjust width to cover the gap if it's a continuous range within the line
                    let width = x_end - x_start;

                    selection_quads.push(fill(
                        Bounds::new(point(x_start - px(SELECTION_PADDING), y_pos), size(width + px(SELECTION_PADDING), row_height)),
                        *color,
                    ));
                }
            }

            // Draw continuous selection background for this line
            let line_start = offset;
            let line_end = offset + 15;
            if line_start <= max_sel && line_end >= min_sel {
                let start_in_line = cmp::max(line_start, min_sel) - line_start;
                let end_in_line = cmp::min(line_end, max_sel) - line_start;

                let x_start = hex_start_x + (hex_byte_width + hex_gap) * start_in_line as f32;
                let x_end = hex_start_x + (hex_byte_width + hex_gap) * end_in_line as f32 + hex_byte_width;
                let width = x_end - x_start;

                selection_quads.push(fill(
                    Bounds::new(point(x_start - px(SELECTION_PADDING), y_pos), size(width + px(SELECTION_PADDING), row_height)),
                    selection_bg_color,
                ));
            }

            let offset_str = format!("{:08x}", offset);
            let offset_run = TextRun {
                len: offset_str.len(),
                font: text_style.font(),
                color: offset_color.into(),
                background_color: None,
                underline: None,
                strikethrough: None,
            };
            let offset_line = if self.show_offset {
                window.text_system().shape_line(offset_str.into(), font_size, &[offset_run], None)
            } else {
                window.text_system().shape_line("".into(), font_size, &[], None)
            };

            let mut hex_lines = Vec::new();
            for (_byte_idx, byte) in chunk.iter().enumerate() {
                let color = if *byte == 0 { hex_null_color } else { hex_byte_color };

                let hex_str = format!("{:02x}", byte);
                let hex_run = TextRun {
                    len: hex_str.len(),
                    font: text_style.font(),
                    color: color.into(),
                    background_color: None,
                    underline: None,
                    strikethrough: None,
                };
                let hex_line = window.text_system().shape_line(hex_str.into(), font_size, &[hex_run], None);
                hex_lines.push(hex_line);
            }

            let mut ascii_str = String::new();
            let mut ascii_runs = Vec::new();
            let mut current_run_start = 0;
            let mut current_color = if let Some(first) = chunk.first() {
                if *first >= 32 && *first <= 126 {
                    ascii_printable_color
                } else {
                    ascii_non_printable_color
                }
            } else {
                ascii_non_printable_color
            };

            for (byte_idx, byte) in chunk.iter().enumerate() {
                let (char_str, color) = if *byte >= 32 && *byte <= 126 {
                    ((*byte as char).to_string(), ascii_printable_color)
                } else {
                    (".".to_string(), ascii_non_printable_color)
                };

                if byte_idx > 0 && color != current_color {
                    ascii_runs.push(TextRun {
                        len: ascii_str.len() - current_run_start,
                        font: text_style.font(),
                        color: current_color.into(),
                        background_color: None,
                        underline: None,
                        strikethrough: None,
                    });
                    current_run_start = ascii_str.len();
                    current_color = color;
                }

                ascii_str.push_str(&char_str);
            }

            if !ascii_str.is_empty() {
                ascii_runs.push(TextRun {
                    len: ascii_str.len() - current_run_start,
                    font: text_style.font(),
                    color: current_color.into(),
                    background_color: None,
                    underline: None,
                    strikethrough: None,
                });
            }

            let ascii_line = if self.show_ascii {
                window.text_system().shape_line(ascii_str.into(), font_size, &ascii_runs, None)
            } else {
                window.text_system().shape_line("".into(), font_size, &[], None)
            };

            data_lines.push(DataLine {
                offset_line,
                hex_lines,
                ascii_line,
            });
        }

        let cursor = {
            let cursor_offset = self.cursor_offset;
            let focus_handle = self.focus_handle.clone();

            // Show cursor if focused, even for empty buffer
            if focus_handle.is_focused(window) {
                let cursor_row = if buffer.len() > 0 { cursor_offset / BYTES_PER_ROW } else { 0 };
                let byte_in_row = if buffer.len() > 0 { cursor_offset % BYTES_PER_ROW } else { 0 };

                if cursor_row >= start_row && cursor_row < end_row {
                    let visible_cursor_row = cursor_row - start_row;
                    let y_pos = bounds.top() + header_height + row_height * visible_cursor_row as f32;
                    let cursor_x = hex_start_x + (hex_byte_width + hex_gap) * byte_in_row as f32;

                    Some(fill(
                        Bounds::new(point(cursor_x, y_pos), size(hex_byte_width, row_height)),
                        theme.accent.opacity(0.3),
                    ))
                } else {
                    None
                }
            } else {
                None
            }
        };

        let header = {
            let header_color = theme.foreground;
            let font = text_style.font();

            let offset_run = TextRun {
                len: 6,
                font: font.clone(),
                color: header_color.into(),
                background_color: None,
                underline: None,
                strikethrough: None,
            };
            let offset = if self.show_offset {
                window.text_system().shape_line("Offset".into(), font_size, &[offset_run], None)
            } else {
                window.text_system().shape_line("".into(), font_size, &[], None)
            };

            let mut hex_bytes = Vec::new();
            for i in 0..16 {
                let s = format!("+{:X}", i);
                let run = TextRun {
                    len: s.len(),
                    font: font.clone(),
                    color: header_color.into(),
                    background_color: None,
                    underline: None,
                    strikethrough: None,
                };
                hex_bytes.push(window.text_system().shape_line(s.into(), font_size, &[run], None));
            }

            let ascii_run = TextRun {
                len: 5,
                font: font.clone(),
                color: header_color.into(),
                background_color: None,
                underline: None,
                strikethrough: None,
            };
            let ascii = if self.show_ascii {
                window.text_system().shape_line("ASCII".into(), font_size, &[ascii_run], None)
            } else {
                window.text_system().shape_line("".into(), font_size, &[], None)
            };

            HeaderParts { offset, hex_bytes, ascii }
        };

        PrepaintState {
            data_lines,
            selection_quads,
            cursor,
            header,
        }
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let header_height = if self.show_header { px(HEADER_HEIGHT) } else { px(0.) };
        let row_height = px(ROW_HEIGHT);
        let offset_width = if self.show_offset { px(OFFSET_WIDTH) } else { px(0.) };

        let hex_start_x = bounds.left() + px(OFFSET_X_START) + offset_width + px(SECTION_GAP);
        let hex_byte_width = px(HEX_BYTE_WIDTH);
        let hex_gap = px(HEX_GAP);
        let ascii_start_x = hex_start_x + (hex_byte_width + hex_gap) * BYTES_PER_ROW as f32 + px(SECTION_GAP);

        let theme = cx.theme();
        let bg_color = theme.background;
        let border_color = theme.border;
        let accent_color = theme.accent;

        window.paint_quad(fill(bounds, bg_color));

        window.paint_quad(fill(
            Bounds::new(
                point(bounds.left() + px(OFFSET_X_START), bounds.top() + header_height - px(1.)),
                size(bounds.size.width - px(OFFSET_X_START), px(1.)),
            ),
            border_color,
        ));

        // Paint header
        if self.show_header {
            let header_y = bounds.top();
            if self.show_offset {
                prepaint
                    .header
                    .offset
                    .paint(point(bounds.left() + px(OFFSET_X_START), header_y), header_height, window, cx)
                    .ok();
            }

            for (i, hex_header) in prepaint.header.hex_bytes.iter().enumerate() {
                let x_pos = hex_start_x + (hex_byte_width + hex_gap) * i as f32;
                hex_header.paint(point(x_pos, header_y), header_height, window, cx).ok();
            }

            if self.show_ascii {
                prepaint.header.ascii.paint(point(ascii_start_x, header_y), header_height, window, cx).ok();
            }
        }

        for selection_quad in prepaint.selection_quads.drain(..) {
            window.paint_quad(selection_quad);
        }

        for (i, data_line) in prepaint.data_lines.iter().enumerate() {
            let y_pos = bounds.top() + header_height + row_height * i as f32;

            if self.show_offset {
                data_line
                    .offset_line
                    .paint(point(bounds.left() + px(OFFSET_X_START), y_pos), row_height, window, cx)
                    .ok();
            }

            for (byte_idx, hex_line) in data_line.hex_lines.iter().enumerate() {
                let x_pos = hex_start_x + (hex_byte_width + hex_gap) * byte_idx as f32;
                hex_line.paint(point(x_pos, y_pos), row_height, window, cx).ok();
            }

            if self.show_ascii {
                data_line.ascii_line.paint(point(ascii_start_x, y_pos), row_height, window, cx).ok();
            }
        }

        // Draw cursor last so it's always visible on top of highlights and selection
        if let Some(cursor_quad) = prepaint.cursor.take() {
            // Get cursor bounds before moving cursor_quad
            let cursor_bounds = cursor_quad.bounds;

            // Draw cursor background
            window.paint_quad(cursor_quad);

            // Draw cursor border for better visibility
            // Top border
            window.paint_quad(fill(Bounds::new(cursor_bounds.origin, size(cursor_bounds.size.width, px(2.))), accent_color));
            // Bottom border
            window.paint_quad(fill(
                Bounds::new(
                    point(cursor_bounds.origin.x, cursor_bounds.origin.y + cursor_bounds.size.height - px(2.)),
                    size(cursor_bounds.size.width, px(2.)),
                ),
                accent_color,
            ));
            // Left border
            window.paint_quad(fill(
                Bounds::new(
                    point(cursor_bounds.origin.x - px(SELECTION_PADDING), cursor_bounds.origin.y),
                    size(px(2.), cursor_bounds.size.height),
                ),
                accent_color,
            ));
            // Right border
            window.paint_quad(fill(
                Bounds::new(
                    point(cursor_bounds.origin.x + cursor_bounds.size.width - px(2.), cursor_bounds.origin.y),
                    size(px(2.), cursor_bounds.size.height),
                ),
                accent_color,
            ));
        }

        self.view
            .update(cx, |view, _cx| {
                view.last_bounds = Some(bounds);
            })
            .ok();
    }
}
