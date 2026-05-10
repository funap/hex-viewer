use crate::core::command::Command;
use crate::core::document::Document;
use crate::core::encoding::Encoding;
use crate::core::structure::ParseResult;
use std::cell::RefCell;
use std::cmp;
use std::collections::BTreeSet;
use std::ops::Range;
use std::sync::Arc;
use std::sync::RwLock;

pub const BYTES_PER_ROW: usize = 16;

#[derive(Default, Clone)]
pub struct SearchState {
    pub query: String,
    pub results: Vec<usize>,
    pub current_result_index: Option<usize>,
    pub is_full_search_complete: bool,
}

#[derive(Clone, Debug)]
pub enum LineMap {
    Standard { total_size: usize },
    Custom(Vec<usize>),
}

impl PartialEq for LineMap {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (LineMap::Standard { total_size: s1 }, LineMap::Standard { total_size: s2 }) => s1 == s2,
            (LineMap::Custom(v1), LineMap::Custom(v2)) => v1 == v2,
            _ => {
                if self.len() != other.len() {
                    return false;
                }
                for i in 0..self.len() {
                    if self.get(i) != other.get(i) {
                        return false;
                    }
                }
                true
            }
        }
    }
}

impl Eq for LineMap {}

impl PartialEq<Vec<usize>> for LineMap {
    fn eq(&self, other: &Vec<usize>) -> bool {
        if self.len() != other.len() {
            return false;
        }
        for i in 0..self.len() {
            if self.get(i) != Some(other[i]) {
                return false;
            }
        }
        true
    }
}

impl PartialEq<LineMap> for Vec<usize> {
    fn eq(&self, other: &LineMap) -> bool {
        other.eq(self)
    }
}

impl LineMap {
    pub fn len(&self) -> usize {
        match self {
            LineMap::Standard { total_size } => {
                if *total_size == 0 {
                    1
                } else {
                    (*total_size + BYTES_PER_ROW - 1) / BYTES_PER_ROW
                }
            }
            LineMap::Custom(vec) => vec.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn get(&self, index: usize) -> Option<usize> {
        match self {
            LineMap::Standard { .. } => {
                let len = self.len();
                if index < len {
                    Some(index * BYTES_PER_ROW)
                } else {
                    None
                }
            }
            LineMap::Custom(vec) => vec.get(index).copied(),
        }
    }

    pub fn binary_search(&self, offset: &usize) -> Result<usize, usize> {
        match self {
            LineMap::Standard { total_size } => {
                if *total_size == 0 {
                    return if *offset == 0 { Ok(0) } else { Err(1) };
                }
                let row = *offset / BYTES_PER_ROW;
                let len = self.len();
                if row < len {
                    if *offset % BYTES_PER_ROW == 0 {
                        Ok(row)
                    } else {
                        Err(row + 1)
                    }
                } else {
                    Err(len)
                }
            }
            LineMap::Custom(vec) => vec.binary_search(offset),
        }
    }
}

/// Represents the editor.
pub struct Editor {
    // Shared document containing buffer and history
    pub document: Arc<RwLock<Document>>,
    pub cursor_offset: usize,
    pub selection_start: Option<usize>,
    pub selection_end: Option<usize>,
    pub search_state: SearchState,
    pub custom_breaks: BTreeSet<usize>,
    /// 16バイト自然境界のうち、改行を抑制すべき位置を記録する。
    /// join_line() で追加され、行を16バイト超に結合するために使用。
    pub custom_joins: BTreeSet<usize>,
    /// 特定のオフセットに挿入された空行の数を記録する。
    pub empty_lines: std::collections::BTreeMap<usize, usize>,
    pub encoding: Encoding,
    pub ksy_definition: Option<crate::core::structure::KsyDefinition>,
    pub parse_result: Option<ParseResult>,
    cached_line_map: RefCell<Option<LineMap>>,
}

impl Editor {
    pub fn new(document: Arc<RwLock<Document>>) -> Self {
        Self {
            document,
            cursor_offset: 0,
            selection_start: None,
            selection_end: None,
            search_state: SearchState::default(),
            custom_breaks: BTreeSet::new(),
            custom_joins: BTreeSet::new(),
            empty_lines: std::collections::BTreeMap::new(),
            encoding: Encoding::default(),
            ksy_definition: None,
            parse_result: None,
            cached_line_map: RefCell::new(None),
        }
    }

    pub fn total_size(&self) -> usize {
        self.document.read().unwrap().buffer.len()
    }

    /// line_starts の中から、指定オフセットが属するデータ行（空行でない行）のインデックスを返す。
    /// 空行（重複エントリ）がある場合、最後の重複（データ行）を返す。
    pub fn find_line_index(offset: usize, line_starts: &LineMap) -> usize {
        match line_starts.binary_search(&offset) {
            Ok(mut idx) => {
                // 重複がある場合、最後の重複（データ行）に移動
                while idx + 1 < line_starts.len() && line_starts.get(idx + 1) == Some(offset) {
                    idx += 1;
                }
                idx
            }
            Err(idx) => idx.saturating_sub(1),
        }
    }

    /// 上方向の次のデータ行（空行をスキップ）のインデックスを返す。
    fn prev_data_line(idx: usize, line_starts: &LineMap) -> Option<usize> {
        let mut i = idx.checked_sub(1)?;
        if line_starts.is_empty() {
            return None;
        }
        // 行の長さを確認して空行をスキップ
        loop {
            let line_start = line_starts.get(i)?;
            let line_end = if i + 1 < line_starts.len() {
                line_starts.get(i + 1)?
            } else {
                return Some(i);
            };
            if line_end > line_start {
                return Some(i);
            }
            if i == 0 {
                return None;
            }
            i -= 1;
        }
    }

    /// 下方向の次のデータ行（空行をスキップ）のインデックスを返す。
    fn next_data_line(idx: usize, line_starts: &LineMap, total_size: usize) -> Option<usize> {
        let mut i = idx + 1;
        while i < line_starts.len() {
            let line_start = line_starts.get(i)?;
            let line_end = if i + 1 < line_starts.len() {
                line_starts.get(i + 1)?
            } else {
                total_size
            };
            if line_end > line_start {
                return Some(i);
            }
            i += 1;
        }
        None
    }

    pub fn value_at_cursor(&self) -> Option<u8> {
        let binding = self.document.read().unwrap();
        let buffer = &binding.buffer;
        buffer.data().get(self.cursor_offset).copied()
    }

    pub fn read_bytes_at_cursor(&self, count: usize) -> Vec<u8> {
        let binding = self.document.read().unwrap();
        let buffer = &binding.buffer;
        let data = buffer.data();
        if self.cursor_offset < data.len() {
            let end = std::cmp::min(self.cursor_offset + count, data.len());
            data[self.cursor_offset..end].to_vec()
        } else {
            Vec::new()
        }
    }

    pub fn set_encoding(&mut self, encoding: Encoding) {
        self.encoding = encoding;
    }

    pub fn selection_range(&self) -> Option<Range<usize>> {
        if let (Some(start), Some(end)) = (self.selection_start, self.selection_end) {
            let min = cmp::min(start, end);
            let max = cmp::max(start, end);
            Some(min..max)
        } else {
            None
        }
    }

    pub fn set_cursor_offset(&mut self, offset: usize) {
        let buffer_len = self.total_size();
        self.selection_start = None;
        self.selection_end = None;
        self.cursor_offset = offset.min(buffer_len);
    }

    pub fn move_left(&mut self) {
        if self.cursor_offset > 0 {
            self.cursor_offset -= 1;
            self.selection_start = None;
            self.selection_end = None;
        }
    }

    pub fn move_right(&mut self) {
        let buffer_len = self.total_size();
        if self.cursor_offset < buffer_len {
            self.cursor_offset += 1;
            self.selection_start = None;
            self.selection_end = None;
        }
    }

    pub fn move_up(&mut self) {
        let line_starts = self.line_starts();
        let current_line_idx = Self::find_line_index(self.cursor_offset, &line_starts);

        if let Some(prev_idx) = Self::prev_data_line(current_line_idx, &line_starts) {
            let current_line_start = line_starts.get(current_line_idx).unwrap();
            let offset_in_line = self.cursor_offset - current_line_start;
            let prev_line_start = line_starts.get(prev_idx).unwrap();
            let prev_line_end = line_starts.get(prev_idx + 1).unwrap();
            let prev_line_len = prev_line_end - prev_line_start;

            self.cursor_offset = prev_line_start + cmp::min(offset_in_line, prev_line_len.saturating_sub(1));
            self.selection_start = None;
            self.selection_end = None;
        }
    }

    pub fn move_down(&mut self) {
        let line_starts = self.line_starts();
        let total_size = self.total_size();
        let current_line_idx = Self::find_line_index(self.cursor_offset, &line_starts);

        if let Some(next_idx) = Self::next_data_line(current_line_idx, &line_starts, total_size) {
            let current_line_start = line_starts.get(current_line_idx).unwrap();
            let offset_in_line = self.cursor_offset - current_line_start;
            let next_line_start = line_starts.get(next_idx).unwrap();
            let next_line_end = if next_idx + 1 < line_starts.len() {
                line_starts.get(next_idx + 1).unwrap()
            } else {
                total_size
            };
            let next_line_len = next_line_end - next_line_start;

            if next_line_len > 0 {
                self.cursor_offset = next_line_start + cmp::min(offset_in_line, next_line_len - 1);
            } else {
                self.cursor_offset = next_line_start;
            }
            self.selection_start = None;
            self.selection_end = None;
        } else {
            self.cursor_offset = total_size;
            self.selection_start = None;
            self.selection_end = None;
        }
    }

    pub fn select_left(&mut self) {
        if self.cursor_offset > 0 {
            if self.selection_start.is_none() {
                self.selection_start = Some(self.cursor_offset);
            }
            self.cursor_offset -= 1;
            self.selection_end = Some(self.cursor_offset);
        }
    }

    pub fn select_right(&mut self) {
        let buffer_len = self.total_size();
        if self.cursor_offset < buffer_len {
            if self.selection_start.is_none() {
                self.selection_start = Some(self.cursor_offset);
            }
            self.cursor_offset += 1;
            self.selection_end = Some(self.cursor_offset);
        }
    }

    pub fn select_up(&mut self) {
        let line_starts = self.line_starts();
        let current_line_idx = Self::find_line_index(self.cursor_offset, &line_starts);

        if let Some(prev_idx) = Self::prev_data_line(current_line_idx, &line_starts) {
            if self.selection_start.is_none() {
                self.selection_start = Some(self.cursor_offset);
            }
            let current_line_start = line_starts.get(current_line_idx).unwrap();
            let offset_in_line = self.cursor_offset - current_line_start;
            let prev_line_start = line_starts.get(prev_idx).unwrap();
            let prev_line_end = line_starts.get(prev_idx + 1).unwrap();
            let prev_line_len = prev_line_end - prev_line_start;

            self.cursor_offset = prev_line_start + cmp::min(offset_in_line, prev_line_len.saturating_sub(1));
            self.selection_end = Some(self.cursor_offset);
        }
    }

    pub fn select_down(&mut self) {
        let line_starts = self.line_starts();
        let total_size = self.total_size();
        let current_line_idx = Self::find_line_index(self.cursor_offset, &line_starts);

        if self.selection_start.is_none() {
            self.selection_start = Some(self.cursor_offset);
        }

        if let Some(next_idx) = Self::next_data_line(current_line_idx, &line_starts, total_size) {
            let current_line_start = line_starts.get(current_line_idx).unwrap();
            let offset_in_line = self.cursor_offset - current_line_start;
            let next_line_start = line_starts.get(next_idx).unwrap();
            let next_line_end = if next_idx + 1 < line_starts.len() {
                line_starts.get(next_idx + 1).unwrap()
            } else {
                total_size
            };
            let next_line_len = next_line_end - next_line_start;

            if next_line_len > 0 {
                self.cursor_offset = next_line_start + cmp::min(offset_in_line, next_line_len - 1);
            } else {
                self.cursor_offset = next_line_start;
            }
            self.selection_end = Some(self.cursor_offset);
        } else {
            self.cursor_offset = total_size;
            self.selection_end = Some(self.cursor_offset);
        }
    }

    pub fn select_all(&mut self) {
        let buffer_len = self.total_size();
        self.selection_start = Some(0);
        self.selection_end = Some(buffer_len);
        self.cursor_offset = buffer_len;
    }

    pub fn go_to_beginning(&mut self) {
        self.cursor_offset = 0;
        self.selection_start = None;
        self.selection_end = None;
    }

    pub fn go_to_end(&mut self) {
        self.cursor_offset = self.total_size();
        self.selection_start = None;
        self.selection_end = None;
    }


    pub fn page_up(&mut self, visible_rows: usize) {
        let line_starts = self.line_starts();
        let current_line_idx = Self::find_line_index(self.cursor_offset, &line_starts);

        self.selection_start = None;
        self.selection_end = None;

        let target_line_idx = current_line_idx.saturating_sub(visible_rows);
        let current_line_start = line_starts.get(current_line_idx).unwrap();
        let offset_in_line = self.cursor_offset - current_line_start;

        let target_line_start = line_starts.get(target_line_idx).unwrap();
        let target_line_end = if target_line_idx + 1 < line_starts.len() {
            line_starts.get(target_line_idx + 1).unwrap()
        } else {
            self.total_size()
        };
        let target_line_len = target_line_end - target_line_start;

        self.cursor_offset = target_line_start + cmp::min(offset_in_line, target_line_len.saturating_sub(1));
    }

    pub fn page_down(&mut self, visible_rows: usize) {
        let line_starts = self.line_starts();
        let current_line_idx = Self::find_line_index(self.cursor_offset, &line_starts);

        self.selection_start = None;
        self.selection_end = None;

        let target_line_idx = cmp::min(current_line_idx + visible_rows, line_starts.len() - 1);
        let current_line_start = line_starts.get(current_line_idx).unwrap();
        let offset_in_line = self.cursor_offset - current_line_start;

        let target_line_start = line_starts.get(target_line_idx).unwrap();
        let target_line_end = if target_line_idx + 1 < line_starts.len() {
            line_starts.get(target_line_idx + 1).unwrap()
        } else {
            self.total_size()
        };
        let target_line_len = target_line_end - target_line_start;

        if target_line_idx == line_starts.len() - 1 && target_line_len == 0 {
            self.cursor_offset = self.total_size();
        } else {
            self.cursor_offset = target_line_start + cmp::min(offset_in_line, target_line_len.saturating_sub(1));
        }
    }

    pub fn home(&mut self) {
        self.selection_start = None;
        self.selection_end = None;
        self.cursor_offset = 0;
    }

    pub fn end(&mut self) {
        let buffer_len = self.total_size();
        self.selection_start = None;
        self.selection_end = None;
        self.cursor_offset = buffer_len;
    }

    pub fn select_page_up(&mut self, visible_rows: usize) {
        let line_starts = self.line_starts();
        let current_line_idx = Self::find_line_index(self.cursor_offset, &line_starts);

        if self.selection_start.is_none() {
            self.selection_start = Some(self.cursor_offset);
        }

        let target_line_idx = current_line_idx.saturating_sub(visible_rows);
        let current_line_start = line_starts.get(current_line_idx).unwrap();
        let offset_in_line = self.cursor_offset - current_line_start;

        let target_line_start = line_starts.get(target_line_idx).unwrap();
        let target_line_end = if target_line_idx + 1 < line_starts.len() {
            line_starts.get(target_line_idx + 1).unwrap()
        } else {
            self.total_size()
        };
        let target_line_len = target_line_end - target_line_start;

        self.cursor_offset = target_line_start + cmp::min(offset_in_line, target_line_len.saturating_sub(1));
        self.selection_end = Some(self.cursor_offset);
    }

    pub fn select_page_down(&mut self, visible_rows: usize) {
        let line_starts = self.line_starts();
        let current_line_idx = Self::find_line_index(self.cursor_offset, &line_starts);

        if self.selection_start.is_none() {
            self.selection_start = Some(self.cursor_offset);
        }

        let target_line_idx = cmp::min(current_line_idx + visible_rows, line_starts.len() - 1);
        let current_line_start = line_starts.get(current_line_idx).unwrap();
        let offset_in_line = self.cursor_offset - current_line_start;

        let target_line_start = line_starts.get(target_line_idx).unwrap();
        let target_line_end = if target_line_idx + 1 < line_starts.len() {
            line_starts.get(target_line_idx + 1).unwrap()
        } else {
            self.total_size()
        };
        let target_line_len = target_line_end - target_line_start;

        if target_line_idx == line_starts.len() - 1 && target_line_len == 0 {
            self.cursor_offset = self.total_size();
        } else {
            self.cursor_offset = target_line_start + cmp::min(offset_in_line, target_line_len.saturating_sub(1));
        }
        self.selection_end = Some(self.cursor_offset);
    }

    pub fn select_home(&mut self) {
        if self.selection_start.is_none() {
            self.selection_start = Some(self.cursor_offset);
        }
        self.cursor_offset = 0;
        self.selection_end = Some(self.cursor_offset);
    }

    pub fn select_end(&mut self) {
        let buffer_len = self.total_size();
        if self.selection_start.is_none() {
            self.selection_start = Some(self.cursor_offset);
        }
        self.cursor_offset = buffer_len;
        self.selection_end = Some(self.cursor_offset);
    }

    pub fn start_drag(&mut self, byte_pos: usize) {
        self.cursor_offset = byte_pos;
        self.selection_start = Some(byte_pos);
        self.selection_end = Some(byte_pos);
    }

    pub fn continue_drag(&mut self, byte_pos: usize) {
        self.selection_end = Some(byte_pos);
    }

    pub fn set_search_query(&mut self, query: String) {
        if self.search_state.query != query {
            self.search_state.query = query;
            self.search_state.results.clear();
            self.search_state.current_result_index = None;
            self.search_state.is_full_search_complete = false;
        }
    }

    pub fn set_search_results(&mut self, results: Vec<usize>, is_full: bool) {
        self.search_state.results = results;
        if is_full {
            self.search_state.is_full_search_complete = true;
        }
        if !self.search_state.results.is_empty() && self.search_state.current_result_index.is_none() {
            self.search_state.current_result_index = Some(0);
        }
    }

    pub fn clear_search(&mut self) {
        self.search_state.query.clear();
        self.search_state.results.clear();
        self.search_state.current_result_index = None;
        self.search_state.is_full_search_complete = false;
    }

    pub fn next_search_result(&mut self) -> Option<usize> {
        if self.search_state.results.is_empty() {
            return None;
        }

        let next_index = if let Some(index) = self.search_state.current_result_index {
            (index + 1) % self.search_state.results.len()
        } else {
            0
        };

        self.search_state.current_result_index = Some(next_index);
        Some(self.search_state.results[next_index])
    }

    pub fn line_starts(&self) -> LineMap {
        if let Some(cached) = self.cached_line_map.borrow().as_ref() {
            return cached.clone();
        }

        let map = if !self.has_custom_layout() {
            LineMap::Standard { total_size: self.total_size() }
        } else {
            let total_size = self.total_size();
            let mut starts = Vec::new();

            let mut current = 0;

            while current < total_size {
                // 空行が指定されていれば、その数だけ現在のオフセットを重複して挿入する
                if let Some(&count) = self.empty_lines.get(&current) {
                    for _ in 0..count {
                        starts.push(current);
                    }
                }

                starts.push(current);

                // Find the next custom break after current
                let next_custom_break = self.custom_breaks.range((current + 1)..).next().copied();

                // Advance in BYTES_PER_ROW increments, skipping any joined boundaries
                let mut next_pos = current + BYTES_PER_ROW;
                while self.custom_joins.contains(&next_pos) && next_pos < total_size {
                    next_pos += BYTES_PER_ROW;
                }

                match next_custom_break {
                    Some(break_pos) if break_pos < next_pos && break_pos > current => {
                        current = break_pos;
                    }
                    _ => {
                        current = next_pos;
                    }
                }
            }

            if starts.is_empty() {
                starts.push(0);
            }

            LineMap::Custom(starts)
        };

        *self.cached_line_map.borrow_mut() = Some(map.clone());
        map
    }

    pub fn add_custom_break(&mut self, offset: usize) {
        if offset < self.total_size() {
            self.custom_breaks.insert(offset);
            // Custom break と custom join が同じ位置にある場合、join を解除
            self.custom_joins.remove(&offset);
            self.cached_line_map.replace(None);
        }
    }

    pub fn remove_custom_break(&mut self, offset: usize) {
        if self.custom_breaks.remove(&offset) {
            self.cached_line_map.replace(None);
        }
    }

    pub fn toggle_custom_break(&mut self, offset: usize) {
        if self.custom_breaks.contains(&offset) {
            self.remove_custom_break(offset);
        } else {
            self.add_custom_break(offset);
        }
    }

    pub fn add_empty_line(&mut self, offset: usize) {
        if offset <= self.total_size() {
            *self.empty_lines.entry(offset).or_insert(0) += 1;
            self.cached_line_map.replace(None);
        }
    }

    pub fn remove_empty_line(&mut self, offset: usize) -> bool {
        if let Some(count) = self.empty_lines.get_mut(&offset) {
            if *count > 1 {
                *count -= 1;
            } else {
                self.empty_lines.remove(&offset);
            }
            self.cached_line_map.replace(None);
            true
        } else {
            false
        }
    }

    /// カーソルの現在行と次の行を結合する。
    /// 次の行の開始位置がCustom Breakなら削除し、
    /// 16バイト自然境界ならcustom_joinsに追加して改行を抑制する。
    pub fn join_line(&mut self) {
        let line_starts = self.line_starts();
        let current_line_idx = Self::find_line_index(self.cursor_offset, &line_starts);

        // 次の行がなければ何もしない
        if current_line_idx + 1 >= line_starts.len() {
            return;
        }

        let next_line_start = line_starts.get(current_line_idx + 1).unwrap();

        if self.custom_breaks.contains(&next_line_start) {
            // Custom Break による改行なら、その break を削除
            self.custom_breaks.remove(&next_line_start);
            self.cached_line_map.replace(None);
        } else if next_line_start % BYTES_PER_ROW == 0 {
            // 16バイト自然境界による改行なら、join として記録
            self.custom_joins.insert(next_line_start);
            self.cached_line_map.replace(None);
        }
    }

    /// 全ての Custom Break と Join をクリアし、デフォルトの16バイト表示に戻す。
    pub fn clear_all_custom_breaks(&mut self) {
        self.custom_breaks.clear();
        self.custom_joins.clear();
        self.empty_lines.clear();
        self.cached_line_map.replace(None);
    }

    pub fn has_custom_layout(&self) -> bool {
        !self.custom_breaks.is_empty() || !self.custom_joins.is_empty() || !self.empty_lines.is_empty()
    }

    pub fn custom_layout_count(&self) -> usize {
        self.custom_breaks.len() + self.custom_joins.len() + self.empty_lines.values().sum::<usize>()
    }

    pub fn prev_search_result(&mut self) -> Option<usize> {
        if self.search_state.results.is_empty() {
            return None;
        }

        let prev_index = if let Some(index) = self.search_state.current_result_index {
            if index == 0 { self.search_state.results.len() - 1 } else { index - 1 }
        } else {
            self.search_state.results.len() - 1
        };

        self.search_state.current_result_index = Some(prev_index);
        Some(self.search_state.results[prev_index])
    }

    pub fn current_search_result(&self) -> Option<usize> {
        self.search_state.current_result_index.and_then(|i| self.search_state.results.get(i).copied())
    }

    pub fn execute_command(&mut self, mut command: Box<dyn Command>) {
        command.execute(self);
        self.document.write().unwrap().history.push(command);
        self.cached_line_map.replace(None);
    }

    pub fn undo(&mut self) {
        // Need to acquire a write lock on the document to access history
        // And also we need to pop from history, then call command.undo(self)
        // command.undo might need to access document.buf, which is in the same lock if we are not careful
        // The current History implementation stores Box<dyn Command>, which is fine.
        // But if I hold the lock while calling command.undo(self), and command.undo tries to lock document again... deadlock.

        let command = {
            let mut doc = self.document.write().unwrap();
            doc.history.pop_undo()
        };

        if let Some(mut cmd) = command {
            cmd.undo(self);

            // Re-acquire lock to push redo
            self.document.write().unwrap().history.push_redo(cmd);
            self.cached_line_map.replace(None);
        }
    }

    pub fn redo(&mut self) {
        let command = {
            let mut doc = self.document.write().unwrap();
            doc.history.pop_redo()
        };

        if let Some(mut cmd) = command {
            cmd.execute(self);

            // Re-acquire lock to push undo
            self.document.write().unwrap().history.push_undo(cmd);
            self.cached_line_map.replace(None);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::command::InsertCharCommand;
    use std::sync::Arc;

    fn create_editor_with_content(content: &[u8]) -> Editor {
        let buffer = crate::core::buffer::Buffer::new(content.to_vec());
        let document = Arc::new(RwLock::new(Document::new(std::path::PathBuf::from("test"), buffer)));
        Editor::new(document)
    }

    #[test]
    fn test_initialization() {
        let editor = create_editor_with_content(b"Hello");
        assert_eq!(editor.total_size(), 5);
        assert_eq!(editor.cursor_offset, 0);
        assert!(editor.selection_start.is_none());
    }

    #[test]
    fn test_cursor_movement() {
        let mut editor = create_editor_with_content(b"123");

        // Move right
        editor.move_right();
        assert_eq!(editor.cursor_offset, 1);

        // Move left
        editor.move_left();
        assert_eq!(editor.cursor_offset, 0);

        // Boundary checks
        editor.move_left();
        assert_eq!(editor.cursor_offset, 0);

        editor.end();
        assert_eq!(editor.cursor_offset, 3);
        editor.move_right();
        assert_eq!(editor.cursor_offset, 3);
    }

    #[test]
    fn test_selection() {
        let mut editor = create_editor_with_content(b"12345");

        // Select Right
        editor.select_right();
        assert_eq!(editor.selection_start, Some(0));
        assert_eq!(editor.selection_end, Some(1));
        assert_eq!(editor.cursor_offset, 1);

        // Clear selection on move
        editor.move_right();
        assert!(editor.selection_start.is_none());
        assert!(editor.selection_end.is_none());

        // Select All
        editor.select_all();
        assert_eq!(editor.selection_start, Some(0));
        assert_eq!(editor.selection_end, Some(5)); // Corrected expectation
    }

    #[test]
    fn test_search_navigation() {
        let mut editor = create_editor_with_content(b"test match test");
        editor.search_state.results = vec![0, 11];

        // Ensure we handle no current index gracefully
        assert_eq!(editor.current_search_result(), None);

        // Next: 0 -> 11
        editor.next_search_result();
        assert_eq!(editor.current_search_result(), Some(0));
        assert_eq!(editor.cursor_offset, 0);

        editor.next_search_result();
        assert_eq!(editor.current_search_result(), Some(11));

        // Wrap around
        editor.next_search_result();
        assert_eq!(editor.current_search_result(), Some(0));

        // Prev
        editor.prev_search_result();
        assert_eq!(editor.current_search_result(), Some(11));
    }

    #[test]
    fn test_shared_document() {
        let buffer = crate::core::buffer::Buffer::new(b"".to_vec());
        let document = Arc::new(RwLock::new(Document::new(std::path::PathBuf::from("test"), buffer)));
        let mut editor1 = Editor::new(document.clone());
        let mut editor2 = Editor::new(document.clone());

        // Insert in editor1
        let cmd1 = Box::new(InsertCharCommand::new(0, b'A'));
        editor1.execute_command(cmd1);

        // Verify editor2 sees change
        assert_eq!(editor2.total_size(), 1);

        // Undo in editor2
        editor2.undo();
        assert_eq!(editor1.total_size(), 0);
    }

    #[test]
    fn test_undo_redo() {
        let mut editor = create_editor_with_content(b"");

        // Insert 'A'
        let cmd = Box::new(InsertCharCommand::new(0, b'A'));
        editor.execute_command(cmd);
        assert_eq!(editor.total_size(), 1);

        // Undo
        editor.undo();
        assert_eq!(editor.total_size(), 0);

        // Redo
        editor.redo();
        assert_eq!(editor.total_size(), 1);
    }

    #[test]
    fn test_line_starts_with_custom_breaks() {
        let mut editor = create_editor_with_content(&[0; 32]);
        // Default: 0, 16
        assert_eq!(editor.line_starts(), vec![0, 16]);

        // Add custom break at 10
        editor.add_custom_break(10);
        // Should be 0, 10, 26
        // current=0 -> push 0. next_custom=10, next_default=16. 10 < 16, so current=10.
        // current=10 -> push 10. next_custom=None, next_default=26. current=26.
        // current=26 -> push 26. next_custom=None, next_default=42. current=42 (>= 32, loop ends).
        assert_eq!(editor.line_starts(), vec![0, 10, 26]);

        // Add custom break at 5
        editor.add_custom_break(5);
        // 0, 5, 10, 26
        assert_eq!(editor.line_starts(), vec![0, 5, 10, 26]);
    }

    #[test]
    fn test_move_up_down_with_custom_breaks() {
        let mut editor = create_editor_with_content(&[0; 32]);
        editor.add_custom_break(10); // Lines: [0..10], [10..26], [26..32]

        editor.set_cursor_offset(5);
        editor.move_down();
        // Move from line 0 pos 5 to line 1 pos 5 (offset 10 + 5 = 15)
        assert_eq!(editor.cursor_offset, 15);

        editor.move_down();
        // Move from line 1 pos 5 to line 2 pos 5 (offset 26 + 5 = 31)
        assert_eq!(editor.cursor_offset, 31);

        editor.move_up();
        assert_eq!(editor.cursor_offset, 15);

        editor.move_up();
        assert_eq!(editor.cursor_offset, 5);

        // Test clamping to line length
        editor.set_cursor_offset(28); // Line 2, pos 2 (28-26)
        editor.move_up();
        // Line 1 is 16 bytes long. pos 2 is valid. 10 + 2 = 12.
        assert_eq!(editor.cursor_offset, 12);

        editor.set_cursor_offset(20); // Line 1, pos 10
        editor.move_down();
        // Line 2 is 6 bytes long. pos 10 is too far. Clamp to 5. 26 + 5 = 31.
        assert_eq!(editor.cursor_offset, 31);
    }

    #[test]
    fn test_join_line_creates_long_rows() {
        let mut editor = create_editor_with_content(&[0; 48]);
        // Default: 0, 16, 32
        assert_eq!(editor.line_starts(), vec![0, 16, 32]);

        // Join line 0 and line 1 (remove 16-byte boundary at offset 16)
        editor.set_cursor_offset(5); // On line 0
        editor.join_line();
        // Now offset 16 is in custom_joins, so line_starts should skip it
        // current=0 -> push 0. next_pos=16, but 16 is in joins, so next_pos=32. current=32.
        // current=32 -> push 32. next_pos=48, not in joins. current=48 (>= 48, loop ends).
        assert_eq!(editor.line_starts(), vec![0, 32]);
    }

    #[test]
    fn test_join_line_removes_custom_break() {
        let mut editor = create_editor_with_content(&[0; 32]);
        editor.add_custom_break(10);
        // Lines: [0..10], [10..26], [26..32]
        assert_eq!(editor.line_starts(), vec![0, 10, 26]);

        // Join line 0 with line 1 (removes custom break at 10)
        editor.set_cursor_offset(3);
        editor.join_line();
        // Custom break at 10 removed, back to default 16-byte lines
        assert_eq!(editor.line_starts(), vec![0, 16]);
    }

    #[test]
    fn test_join_line_multiple_joins() {
        let mut editor = create_editor_with_content(&[0; 64]);
        // Default: 0, 16, 32, 48
        assert_eq!(editor.line_starts(), vec![0, 16, 32, 48]);

        // Join all into one big line
        editor.set_cursor_offset(0);
        editor.join_line(); // joins 0+16 -> skip 16
        editor.join_line(); // joins 0+32 -> skip 32
        editor.join_line(); // joins 0+48 -> skip 48
        // All boundaries joined, single line
        assert_eq!(editor.line_starts(), vec![0]);
    }

    #[test]
    fn test_clear_all_custom_breaks() {
        let mut editor = create_editor_with_content(&[0; 48]);
        editor.add_custom_break(5);
        editor.add_custom_break(10);
        editor.set_cursor_offset(0);
        editor.join_line(); // join some lines

        assert!(editor.has_custom_layout());
        assert!(editor.custom_layout_count() > 0);

        editor.clear_all_custom_breaks();
        assert!(!editor.has_custom_layout());
        assert_eq!(editor.custom_layout_count(), 0);
        // Back to default
        assert_eq!(editor.line_starts(), vec![0, 16, 32]);
    }

    #[test]
    fn test_custom_break_overrides_join() {
        let mut editor = create_editor_with_content(&[0; 48]);
        // Join at 16
        editor.set_cursor_offset(0);
        editor.join_line();
        assert_eq!(editor.line_starts(), vec![0, 32]);

        // Adding a custom break at 16 should remove the join
        editor.add_custom_break(16);
        assert_eq!(editor.line_starts(), vec![0, 16, 32]);
    }
}

impl Editor {
    pub fn set_kaitai_definition(&mut self, ksy: crate::core::structure::KsyDefinition) {
        // If the definition is already the same, skip re-parsing unless necessary.
        // We compare by ID for now.
        if let Some(existing) = &self.ksy_definition {
            if existing.meta.id == ksy.meta.id {
                return;
            }
        }

        let buffer_lock = self.document.read().unwrap();
        let bytes = buffer_lock.buffer.data();
        let mut stream = std::io::Cursor::new(bytes);

        let interpreter = crate::core::structure::KaitaiInterpreter::new(ksy.clone(), &mut stream);
        let result = interpreter.parse();

        self.ksy_definition = Some(ksy);
        self.parse_result = Some(result);
    }

    pub fn clear_structure_definition(&mut self) {
        self.ksy_definition = None;
        self.parse_result = None;
    }
}
