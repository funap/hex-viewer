use crate::data::file_buffer::FileBuffer;
use gpui::*;
use gpui_component::dock::{Panel, PanelEvent};
use std::sync::Arc;

pub struct EditorPanel {
    buffer: Arc<FileBuffer>,
    focus_handle: FocusHandle,
    selection_start: Option<usize>,
    selection_end: Option<usize>,
    is_dragging: bool,
    last_bounds: Option<Bounds<Pixels>>,
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
        }
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
        if self.is_dragging {
            if let Some(byte_pos) = self.byte_pos_from_point(event.position) {
                self.selection_end = Some(byte_pos);
                cx.notify();
            }
        }
    }

    fn on_mouse_up(&mut self, _event: &MouseUpEvent, _window: &mut Window, cx: &mut Context<Self>) {
        self.is_dragging = false;
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
        div()
            .flex()
            .flex_col()
            .bg(rgb(0x1e1e1e))
            .font_family("Menlo")
            .size_full()
            .track_focus(&self.focus_handle(cx))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::on_mouse_down))
            .on_mouse_move(cx.listener(Self::on_mouse_move))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .child(HexViewElement { panel: cx.entity() })
    }
}

struct HexViewElement {
    panel: Entity<EditorPanel>,
}

struct PrepaintState {
    data_lines: Vec<DataLine>,
    selection_quads: Vec<PaintQuad>,
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

        let offset_color = rgb(0x858585);
        let hex_byte_color = rgb(0x9cdcfe);
        let hex_null_color = rgb(0x505050);
        let ascii_printable_color = rgb(0xce9178);
        let ascii_non_printable_color = rgb(0x505050);
        let selection_bg_color = rgb(0x264f78);

        let line_count = (buffer.len() + 15) / 16;
        let header_height = px(32.);
        let row_height = px(24.);

        let mut data_lines = Vec::new();
        let mut selection_quads = Vec::new();

        let offset_width = px(96.);
        let hex_start_x = bounds.left() + offset_width + px(16.);
        let hex_byte_width = px(22.);
        let hex_gap = px(4.);

        for i in 0..line_count {
            let offset = i * 16;
            let chunk = buffer.get_range(offset, 16);
            let y_pos = bounds.top() + header_height + row_height * i as f32;

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
            for (byte_idx, byte) in chunk.iter().enumerate() {
                let byte_pos = offset + byte_idx;
                let is_selected = if let (Some(start), Some(end)) = (selection_start, selection_end)
                {
                    let (min_pos, max_pos) = if start <= end {
                        (start, end)
                    } else {
                        (end, start)
                    };
                    byte_pos >= min_pos && byte_pos <= max_pos
                } else {
                    false
                };

                let color = if *byte == 0 {
                    hex_null_color
                } else {
                    hex_byte_color
                };

                if is_selected {
                    let x_pos = hex_start_x + (hex_byte_width + hex_gap) * byte_idx as f32;
                    selection_quads.push(fill(
                        Bounds::new(point(x_pos, y_pos), size(hex_byte_width, row_height)),
                        selection_bg_color,
                    ));
                }

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
            let mut current_color = ascii_non_printable_color;

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

        PrepaintState {
            data_lines,
            selection_quads,
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

        let bg_color = rgb(0x1e1e1e);
        let border_color = rgb(0x333333);

        window.paint_quad(fill(bounds, bg_color));

        window.paint_quad(fill(
            Bounds::new(
                point(bounds.left(), bounds.top() + header_height - px(1.)),
                size(bounds.size.width, px(1.)),
            ),
            border_color,
        ));

        for selection_quad in prepaint.selection_quads.drain(..) {
            window.paint_quad(selection_quad);
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
