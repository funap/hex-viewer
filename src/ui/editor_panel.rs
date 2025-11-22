use crate::data::file_buffer::FileBuffer;
use gpui::ScrollWheelEvent;
use gpui::*;
use gpui_component::dock::{Panel, PanelEvent};
use gpui_component::scroll::*;
use gpui_component::{ActiveTheme, PixelsExt};
use std::cmp;
use std::sync::Arc;

actions!(
    editor_panel,
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
        SelectEnd
    ]
);

const CONTEXT: &str = "EditorPanel";

pub(crate) fn init(cx: &mut App) {
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
    ]);
}
pub struct EditorPanel {
    buffer: Arc<FileBuffer>,
    focus_handle: FocusHandle,
    selection_start: Option<usize>,
    selection_end: Option<usize>,
    is_dragging: bool,
    last_bounds: Option<Bounds<Pixels>>,
    cursor_offset: usize,
    scroll_offset: usize,
    scrollbar_state: ScrollbarState,
    scroll_handle: ScrollHandle,
}

impl EditorPanel {
    pub fn new(buffer: Arc<FileBuffer>, cx: &mut Context<Self>) -> Self {
        Self {
            buffer,
            focus_handle: cx.focus_handle(),
            selection_start: None,
            selection_end: None,
            is_dragging: false,
            last_bounds: None,
            cursor_offset: 0,
            scroll_offset: 0,
            scrollbar_state: ScrollbarState::default(),
            scroll_handle: ScrollHandle::new(),
        }
    }

    fn cursor_offset(&self) -> usize {
        self.cursor_offset
    }

    fn ensure_cursor_visible(&mut self) {
        let bounds = match self.last_bounds {
            Some(b) => b,
            None => return,
        };

        let header_height = px(32.);
        let row_height = px(24.);
        let visible_height = bounds.size.height - header_height;
        let visible_rows = (visible_height / row_height).floor() as usize;

        let cursor_row = self.cursor_offset / 16;

        if cursor_row < self.scroll_offset {
            self.scroll_offset = cursor_row;
        } else if cursor_row >= self.scroll_offset + visible_rows {
            self.scroll_offset = cursor_row.saturating_sub(visible_rows - 1);
        }
        self.scroll_handle
            .set_offset(point(px(0.), -(self.scroll_offset as f32 * row_height)));
    }

    fn byte_pos_from_point(&self, point: Point<Pixels>) -> Option<usize> {
        let bounds = self.last_bounds?;
        let header_height = px(32.);
        let row_height = px(24.);
        let offset_width = px(96.);
        let hex_start_x = bounds.left() + offset_width + px(16.);
        let hex_byte_width = px(22.);
        let hex_gap = px(4.);

        if point.y < bounds.top() + header_height {
            return None;
        }

        let y_offset = point.y - bounds.top() - header_height;
        let row = (y_offset / row_height).floor() as usize;

        let x_offset = point.x - hex_start_x;
        if x_offset < px(0.) {
            return None;
        }

        let byte_in_row = (x_offset / (hex_byte_width + hex_gap)).floor() as usize;
        if byte_in_row >= 16 {
            return None;
        }

        let byte_pos = row * 16 + byte_in_row;
        if byte_pos >= self.buffer.len() {
            return None;
        }

        Some(byte_pos)
    }

    fn on_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(byte_pos) = self.byte_pos_from_point(event.position) {
            self.is_dragging = true;
            self.cursor_offset = byte_pos;
            self.selection_start = Some(byte_pos);
            self.selection_end = Some(byte_pos);
            cx.notify();
        }
    }

    fn on_mouse_move(
        &mut self,
        event: &MouseMoveEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // Sync scroll handle to scroll offset if changed by scrollbar drag
        let row_height = px(24.);
        let handle_y = self.scroll_handle.offset().y;
        let handle_row = ((-handle_y).max(px(0.)) / row_height).round() as usize;
        let total_rows = (self.buffer.len() + 15) / 16;
        if handle_row != self.scroll_offset {
            self.scroll_offset = handle_row.min(total_rows.saturating_sub(1));
            cx.notify();
        }

        if self.is_dragging {
            if let Some(byte_pos) = self.byte_pos_from_point(event.position) {
                self.selection_end = Some(byte_pos);
                cx.notify();
            }
        }
    }

    fn on_mouse_up(&mut self, _event: &MouseUpEvent, _window: &mut Window, cx: &mut Context<Self>) {
        self.is_dragging = false;

        // Sync scroll handle on mouse up as well
        let row_height = px(24.);
        let handle_y = self.scroll_handle.offset().y;
        let handle_row = ((-handle_y).max(px(0.)) / row_height).round() as usize;
        let total_rows = (self.buffer.len() + 15) / 16;
        if handle_row != self.scroll_offset {
            self.scroll_offset = handle_row.min(total_rows.saturating_sub(1));
            cx.notify();
        } else {
            cx.notify();
        }
    }

    fn on_scroll_wheel(
        &mut self,
        event: &ScrollWheelEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // Determine visible rows based on current bounds
        let bounds = match self.last_bounds {
            Some(b) => b,
            None => return,
        };
        let header_height = px(32.);
        let row_height = px(24.);
        let visible_height = bounds.size.height - header_height;
        let visible_rows = (visible_height / row_height).floor() as usize;
        let total_rows = (self.buffer.len() + 15) / 16;
        let max_offset = total_rows.saturating_sub(visible_rows) as i32;

        // Scroll delta Y: positive means scroll down (content moves up)
        let delta_y = event.delta.pixel_delta(row_height).y.as_f32() as i32;

        let new_scroll_offset = self.scroll_offset as i32 - delta_y;

        self.scroll_offset = cmp::max(0, cmp::min(new_scroll_offset, max_offset)) as usize;
        self.scroll_handle
            .set_offset(point(px(0.), -(self.scroll_offset as f32 * row_height)));
        cx.notify();
    }

    fn move_left(&mut self, _: &MoveLeft, _window: &mut Window, cx: &mut Context<Self>) {
        self.selection_start = None;
        self.selection_end = None;
        if self.cursor_offset > 0 {
            self.cursor_offset -= 1;
            self.ensure_cursor_visible();
            cx.notify();
        }
    }

    fn move_right(&mut self, _: &MoveRight, _window: &mut Window, cx: &mut Context<Self>) {
        self.selection_start = None;
        self.selection_end = None;
        if self.cursor_offset < self.buffer.len().saturating_sub(1) {
            self.cursor_offset += 1;
            self.ensure_cursor_visible();
            cx.notify();
        }
    }

    fn move_up(&mut self, _: &MoveUp, _window: &mut Window, cx: &mut Context<Self>) {
        self.selection_start = None;
        self.selection_end = None;
        if self.cursor_offset >= 16 {
            self.cursor_offset -= 16;
            self.ensure_cursor_visible();
            cx.notify();
        }
    }

    fn move_down(&mut self, _: &MoveDown, _window: &mut Window, cx: &mut Context<Self>) {
        self.selection_start = None;
        self.selection_end = None;
        let new_offset = self.cursor_offset + 16;
        if new_offset < self.buffer.len() {
            self.cursor_offset = new_offset;
            self.ensure_cursor_visible();
            cx.notify();
        }
    }

    fn select_left(&mut self, _: &SelectLeft, _window: &mut Window, cx: &mut Context<Self>) {
        if self.selection_start.is_none() {
            self.selection_start = Some(self.cursor_offset);
        }
        if self.cursor_offset > 0 {
            self.cursor_offset -= 1;
            self.selection_end = Some(self.cursor_offset);
            self.ensure_cursor_visible();
            cx.notify();
        }
    }

    fn select_right(&mut self, _: &SelectRight, _window: &mut Window, cx: &mut Context<Self>) {
        if self.selection_start.is_none() {
            self.selection_start = Some(self.cursor_offset);
        }
        if self.cursor_offset < self.buffer.len().saturating_sub(1) {
            self.cursor_offset += 1;
            self.selection_end = Some(self.cursor_offset);
            self.ensure_cursor_visible();
            cx.notify();
        }
    }

    fn select_up(&mut self, _: &SelectUp, _window: &mut Window, cx: &mut Context<Self>) {
        if self.selection_start.is_none() {
            self.selection_start = Some(self.cursor_offset);
        }
        if self.cursor_offset >= 16 {
            self.cursor_offset -= 16;
            self.selection_end = Some(self.cursor_offset);
            self.ensure_cursor_visible();
            cx.notify();
        }
    }

    fn select_down(&mut self, _: &SelectDown, _window: &mut Window, cx: &mut Context<Self>) {
        if self.selection_start.is_none() {
            self.selection_start = Some(self.cursor_offset);
        }
        let new_offset = self.cursor_offset + 16;
        if new_offset < self.buffer.len() {
            self.cursor_offset = new_offset;
            self.selection_end = Some(self.cursor_offset);
            self.ensure_cursor_visible();
            cx.notify();
        }
    }

    fn select_all(&mut self, _: &SelectAll, _window: &mut Window, cx: &mut Context<Self>) {
        self.selection_start = Some(0);
        self.selection_end = Some(self.buffer.len().saturating_sub(1));
        self.cursor_offset = self.buffer.len().saturating_sub(1);
        cx.notify();
    }

    fn get_visible_rows(&self) -> usize {
        if let Some(bounds) = self.last_bounds {
            let header_height = px(32.);
            let row_height = px(24.);
            let visible_height = bounds.size.height - header_height;
            (visible_height / row_height).floor() as usize
        } else {
            10 // Default fallback
        }
    }

    fn page_up(&mut self, _: &PageUp, _window: &mut Window, cx: &mut Context<Self>) {
        self.selection_start = None;
        self.selection_end = None;
        let visible_rows = self.get_visible_rows();
        let move_amount = visible_rows * 16;
        if self.cursor_offset >= move_amount {
            self.cursor_offset -= move_amount;
        } else {
            self.cursor_offset = 0;
        }
        self.ensure_cursor_visible();
        cx.notify();
    }

    fn page_down(&mut self, _: &PageDown, _window: &mut Window, cx: &mut Context<Self>) {
        self.selection_start = None;
        self.selection_end = None;
        let visible_rows = self.get_visible_rows();
        let move_amount = visible_rows * 16;
        let new_offset = self.cursor_offset + move_amount;
        if new_offset < self.buffer.len() {
            self.cursor_offset = new_offset;
        } else {
            self.cursor_offset = self.buffer.len().saturating_sub(1);
        }
        self.ensure_cursor_visible();
        cx.notify();
    }

    fn home(&mut self, _: &Home, _window: &mut Window, cx: &mut Context<Self>) {
        self.selection_start = None;
        self.selection_end = None;
        self.cursor_offset = 0;
        self.ensure_cursor_visible();
        cx.notify();
    }

    fn end(&mut self, _: &End, _window: &mut Window, cx: &mut Context<Self>) {
        self.selection_start = None;
        self.selection_end = None;
        self.cursor_offset = self.buffer.len().saturating_sub(1);
        self.ensure_cursor_visible();
        cx.notify();
    }

    fn select_page_up(&mut self, _: &SelectPageUp, _window: &mut Window, cx: &mut Context<Self>) {
        if self.selection_start.is_none() {
            self.selection_start = Some(self.cursor_offset);
        }
        let visible_rows = self.get_visible_rows();
        let move_amount = visible_rows * 16;
        if self.cursor_offset >= move_amount {
            self.cursor_offset -= move_amount;
        } else {
            self.cursor_offset = 0;
        }
        self.selection_end = Some(self.cursor_offset);
        self.ensure_cursor_visible();
        cx.notify();
    }

    fn select_page_down(
        &mut self,
        _: &SelectPageDown,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.selection_start.is_none() {
            self.selection_start = Some(self.cursor_offset);
        }
        let visible_rows = self.get_visible_rows();
        let move_amount = visible_rows * 16;
        let new_offset = self.cursor_offset + move_amount;
        if new_offset < self.buffer.len() {
            self.cursor_offset = new_offset;
        } else {
            self.cursor_offset = self.buffer.len().saturating_sub(1);
        }
        self.selection_end = Some(self.cursor_offset);
        self.ensure_cursor_visible();
        cx.notify();
    }

    fn select_home(&mut self, _: &SelectHome, _window: &mut Window, cx: &mut Context<Self>) {
        if self.selection_start.is_none() {
            self.selection_start = Some(self.cursor_offset);
        }
        self.cursor_offset = 0;
        self.selection_end = Some(self.cursor_offset);
        self.ensure_cursor_visible();
        cx.notify();
    }

    fn select_end(&mut self, _: &SelectEnd, _window: &mut Window, cx: &mut Context<Self>) {
        if self.selection_start.is_none() {
            self.selection_start = Some(self.cursor_offset);
        }
        self.cursor_offset = self.buffer.len().saturating_sub(1);
        self.selection_end = Some(self.cursor_offset);
        self.ensure_cursor_visible();
        cx.notify();
    }
}

impl EventEmitter<PanelEvent> for EditorPanel {}

impl Focusable for EditorPanel {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Panel for EditorPanel {
    fn panel_name(&self) -> &'static str {
        "EditorPanel"
    }

    fn title(&self, _window: &Window, _cx: &App) -> AnyElement {
        "Hex Editor".into_any_element()
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

impl Render for EditorPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let header_height = px(32.);
        let row_height = px(24.);
        let total_rows = (self.buffer.len() + 15) / 16;

        let extra_height = if let Some(bounds) = self.last_bounds {
            let visible_height = bounds.size.height - header_height;
            let ratio = visible_height / row_height;
            visible_height - row_height * ratio.floor()
        } else {
            px(0.)
        };

        let total_height = header_height + row_height * total_rows as f32 + extra_height;

        let handle_y = self.scroll_handle.offset().y;
        let handle_row = ((-handle_y).max(px(0.)) / row_height).round() as usize;
        if handle_row != self.scroll_offset {
            self.scroll_offset = handle_row.min(total_rows.saturating_sub(1));
        }

        div()
            .flex()
            .flex_col()
            .bg(cx.theme().background)
            .font_family("Menlo")
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
            .on_scroll_wheel(cx.listener(Self::on_scroll_wheel))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::on_mouse_down))
            .on_mouse_move(cx.listener(Self::on_mouse_move))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .child(HexViewElement {
                panel: cx.entity(),
                scroll_offset: self.scroll_offset,
            })
            .child(
                div().absolute().top_0().right_0().bottom_0().w_4().child(
                    Scrollbar::vertical(&self.scrollbar_state, &self.scroll_handle)
                        .axis(ScrollbarAxis::Vertical)
                        .scroll_size(size(px(0.), total_height)),
                ),
            )
    }
}

struct HexViewElement {
    panel: Entity<EditorPanel>,
    scroll_offset: usize,
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
        let panel = self.panel.read(cx);
        let line_count = (panel.buffer.len() + 15) / 16;
        let header_height = px(32.);
        let row_height = px(24.);
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
        let panel = self.panel.read(cx);
        let buffer = panel.buffer.clone();
        let selection_start = panel.selection_start;
        let selection_end = panel.selection_end;

        let text_style = window.text_style();
        let font_size = text_style.font_size.to_pixels(window.rem_size());

        let theme = cx.theme();
        let offset_color = theme.info;
        let hex_byte_color = theme.primary;
        let hex_null_color = theme.muted;
        let ascii_printable_color = theme.foreground;
        let ascii_non_printable_color = theme.muted;
        let selection_bg_color = theme.secondary;

        let line_count = (buffer.len() + 15) / 16;
        let header_height = px(32.);
        let row_height = px(24.);

        let scroll_offset = self.scroll_offset;
        let visible_height = bounds.size.height - header_height;
        let visible_rows = (visible_height / row_height).ceil() as usize + 1;
        let start_row = scroll_offset;
        let end_row = (scroll_offset + visible_rows).min(line_count);

        let mut data_lines = Vec::new();
        let mut selection_quads = Vec::new();

        let offset_width = px(96.);
        let hex_start_x = bounds.left() + offset_width + px(16.);
        let hex_byte_width = px(22.);
        let hex_gap = px(4.);

        let (min_sel, max_sel) = if let (Some(start), Some(end)) = (selection_start, selection_end)
        {
            if start <= end {
                (start, end)
            } else {
                (end, start)
            }
        } else {
            (usize::MAX, usize::MIN)
        };

        for i in start_row..end_row {
            let offset = i * 16;
            let chunk = buffer.get_range(offset, 16);
            let row_index = i - start_row;
            let y_pos = bounds.top() + header_height + row_height * row_index as f32;

            // Draw continuous selection background for this line
            let line_start = offset;
            let line_end = offset + 15;
            if line_start <= max_sel && line_end >= min_sel {
                let start_in_line = cmp::max(line_start, min_sel) - line_start;
                let end_in_line = cmp::min(line_end, max_sel) - line_start;

                let x_start = hex_start_x + (hex_byte_width + hex_gap) * start_in_line as f32;
                let x_end =
                    hex_start_x + (hex_byte_width + hex_gap) * end_in_line as f32 + hex_byte_width;
                let width = x_end - x_start;

                selection_quads.push(fill(
                    Bounds::new(point(x_start, y_pos), size(width, row_height)),
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
            let offset_line =
                window
                    .text_system()
                    .shape_line(offset_str.into(), font_size, &[offset_run], None);

            let mut hex_lines = Vec::new();
            for (_byte_idx, byte) in chunk.iter().enumerate() {
                let color = if *byte == 0 {
                    hex_null_color
                } else {
                    hex_byte_color
                };

                let hex_str = format!("{:02x}", byte);
                let hex_run = TextRun {
                    len: hex_str.len(),
                    font: text_style.font(),
                    color: color.into(),
                    background_color: None,
                    underline: None,
                    strikethrough: None,
                };
                let hex_line =
                    window
                        .text_system()
                        .shape_line(hex_str.into(), font_size, &[hex_run], None);
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

            let ascii_line =
                window
                    .text_system()
                    .shape_line(ascii_str.into(), font_size, &ascii_runs, None);

            data_lines.push(DataLine {
                offset_line,
                hex_lines,
                ascii_line,
            });
        }

        let cursor = {
            let cursor_offset = panel.cursor_offset();
            let focus_handle = panel.focus_handle.clone();

            if focus_handle.is_focused(window) && cursor_offset < buffer.len() {
                let cursor_row = cursor_offset / 16;
                let byte_in_row = cursor_offset % 16;
                // Adjust for current scroll offset
                let visible_cursor_row = if cursor_row >= panel.scroll_offset {
                    cursor_row - panel.scroll_offset
                } else {
                    0
                };
                let y_pos = bounds.top() + header_height + row_height * visible_cursor_row as f32;
                let cursor_x = hex_start_x + (hex_byte_width + hex_gap) * byte_in_row as f32;

                Some(fill(
                    Bounds::new(point(cursor_x, y_pos), size(hex_byte_width, row_height)),
                    theme.accent,
                ))
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
            let offset =
                window
                    .text_system()
                    .shape_line("Offset".into(), font_size, &[offset_run], None);

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
                hex_bytes.push(
                    window
                        .text_system()
                        .shape_line(s.into(), font_size, &[run], None),
                );
            }

            let ascii_run = TextRun {
                len: 5,
                font: font.clone(),
                color: header_color.into(),
                background_color: None,
                underline: None,
                strikethrough: None,
            };
            let ascii =
                window
                    .text_system()
                    .shape_line("ASCII".into(), font_size, &[ascii_run], None);

            HeaderParts {
                offset,
                hex_bytes,
                ascii,
            }
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
        let header_height = px(32.);
        let row_height = px(24.);
        let offset_width = px(96.);
        let hex_start_x = bounds.left() + offset_width + px(16.);
        let hex_byte_width = px(22.);
        let hex_gap = px(4.);
        let ascii_start_x = hex_start_x + (hex_byte_width + hex_gap) * 16.0 + px(16.);

        let theme = cx.theme();
        let bg_color = theme.background;
        let border_color = theme.border;

        window.paint_quad(fill(bounds, bg_color));

        window.paint_quad(fill(
            Bounds::new(
                point(bounds.left(), bounds.top() + header_height - px(1.)),
                size(bounds.size.width, px(1.)),
            ),
            border_color,
        ));

        // Paint header
        let header_y = bounds.top();
        prepaint
            .header
            .offset
            .paint(point(bounds.left(), header_y), header_height, window, cx)
            .ok();

        for (i, hex_header) in prepaint.header.hex_bytes.iter().enumerate() {
            let x_pos = hex_start_x + (hex_byte_width + hex_gap) * i as f32;
            hex_header
                .paint(point(x_pos, header_y), header_height, window, cx)
                .ok();
        }

        prepaint
            .header
            .ascii
            .paint(point(ascii_start_x, header_y), header_height, window, cx)
            .ok();

        for selection_quad in prepaint.selection_quads.drain(..) {
            window.paint_quad(selection_quad);
        }

        if let Some(cursor) = prepaint.cursor.take() {
            window.paint_quad(cursor);
        }

        for (i, data_line) in prepaint.data_lines.iter().enumerate() {
            let y_pos = bounds.top() + header_height + row_height * i as f32;

            data_line
                .offset_line
                .paint(point(bounds.left(), y_pos), row_height, window, cx)
                .ok();

            for (byte_idx, hex_line) in data_line.hex_lines.iter().enumerate() {
                let x_pos = hex_start_x + (hex_byte_width + hex_gap) * byte_idx as f32;
                hex_line
                    .paint(point(x_pos, y_pos), row_height, window, cx)
                    .ok();
            }

            data_line
                .ascii_line
                .paint(point(ascii_start_x, y_pos), row_height, window, cx)
                .ok();
        }

        self.panel.update(cx, |panel, _cx| {
            panel.last_bounds = Some(bounds);
        });
    }
}
