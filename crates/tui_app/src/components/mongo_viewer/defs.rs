use ratatui::widgets::{ListState, TableState};
// use std::collections::HashMap;
use tui_textarea::TextArea;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum QueryField {
    Filter,
    Sort,
    Limit,
    Projection,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ViewMode {
    Table,
    Json,
}

#[derive(Debug, Default, Clone)]
pub struct PaginationState {
    pub current_page: usize,
    pub total_count: Option<u64>,
}

#[derive(Debug, Clone)]
pub enum PopupState {
    None,
    ConnectionManager {
        name: Box<TextArea<'static>>,
        uri: Box<TextArea<'static>>,
        is_editing_uri: bool,
    },
    QueryBuilder {
        active_field: QueryField,
    },
    JsonViewer(String, String, usize), // json, doc_id, offset
    FieldSelector(ListState, Vec<String>, Vec<String>), // State, All, Visible
    Help(TableState),
    Error(String),
}
