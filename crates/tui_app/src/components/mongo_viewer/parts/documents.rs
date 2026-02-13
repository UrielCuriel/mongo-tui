use color_eyre::eyre::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    prelude::*,
    widgets::{
        Block, BorderType, Borders, Cell, List, ListItem, ListState, Row, Table, TableState,
    },
};
// use serde_json::Value;
use std::collections::HashSet;

use super::super::{context::MongoContext, defs::ViewMode, pane_id::PaneId, registry::Pane};
use crate::action::Action;

pub struct DocumentsPane {
    id: PaneId,
    view_mode: ViewMode,
    table_state: TableState,
    list_state: ListState,
    selected_column_index: usize,
    visible_fields: Vec<String>,
    all_fields: Vec<String>,
    // expanded_docs: HashMap<usize, bool>,
}

impl DocumentsPane {
    pub fn new(id: PaneId) -> Self {
        Self {
            id,
            view_mode: ViewMode::Table,
            table_state: TableState::default(),
            list_state: ListState::default(),
            selected_column_index: 0,
            visible_fields: vec!["_id".to_string()],
            all_fields: vec![],
            // expanded_docs: HashMap::new(),
        }
    }

    fn toggle_view_mode(&mut self) {
        self.view_mode = match self.view_mode {
            ViewMode::Table => ViewMode::Json,
            ViewMode::Json => ViewMode::Table,
        };
    }
}

impl Pane for DocumentsPane {
    fn id(&self) -> PaneId {
        self.id
    }

    fn get_shortcuts(&self) -> Vec<(&'static str, &'static str)> {
        let mut s = vec![("Enter", "View"), ("j/k", "Nav")];
        if self.view_mode == ViewMode::Table {
            s.push(("h/l", "Columns"));
            s.push(("y/Y", "Copy ID/Doc"));
            s.push(("p/P", "Copy Val/Key"));
            s.push(("f", "Fields"));
        } else {
            s.push(("y/Y", "Copy ID/Doc"));
        }
        s.push(("v", "Toggle View"));
        s
    }

    fn update(&mut self, action: Action, ctx: &mut MongoContext) -> Result<Option<Action>> {
        match action {
            Action::DocumentsLoaded(_) => {
                // Reset visible fields to default
                self.visible_fields = vec!["_id".to_string()];

                // Update all_fields based on keys in the first few documents
                let mut fields = HashSet::new();
                for doc in ctx.documents.iter().take(20) {
                    for k in doc.keys() {
                        fields.insert(k.clone());
                    }
                }
                let mut sorted_fields: Vec<String> = fields.into_iter().collect();
                sorted_fields.sort();
                self.all_fields = sorted_fields;

                // Add a few more fields to visible by default if available
                for field in self.all_fields.iter() {
                    if field != "_id" && self.visible_fields.len() < 5 {
                        self.visible_fields.push(field.clone());
                    }
                }

                // Reset selection
                self.table_state.select(if !ctx.documents.is_empty() {
                    Some(0)
                } else {
                    None
                });
                self.list_state.select(if !ctx.documents.is_empty() {
                    Some(0)
                } else {
                    None
                });
            }
            Action::ToggleViewMode => {
                self.toggle_view_mode();
                return Ok(Some(Action::Render));
            }
            // Add handler for field selection if it comes via Action
            _ => {}
        }
        Ok(None)
    }

    fn handle_key_event(
        &mut self,
        key: KeyEvent,
        ctx: &mut MongoContext,
    ) -> Result<Option<Action>> {
        match key.code {
            KeyCode::Char('v') => {
                self.toggle_view_mode();
                return Ok(Some(Action::Render));
            }
            KeyCode::Char('f') => {
                return Ok(Some(Action::OpenFieldSelector));
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let len = ctx.documents.len();
                if len > 0 {
                    let i = match self.table_state.selected() {
                        Some(i) => {
                            if i >= len - 1 {
                                len - 1
                            } else {
                                i + 1
                            }
                        }
                        None => 0,
                    };
                    self.table_state.select(Some(i));
                    self.list_state.select(Some(i));
                    return Ok(Some(Action::Render));
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                let len = ctx.documents.len();
                if len > 0 {
                    let i = match self.table_state.selected() {
                        Some(i) => {
                            if i == 0 {
                                0
                            } else {
                                i - 1
                            }
                        }
                        None => 0,
                    };
                    self.table_state.select(Some(i));
                    self.list_state.select(Some(i));
                    return Ok(Some(Action::Render));
                }
            }
            KeyCode::Left | KeyCode::Char('h') if self.view_mode == ViewMode::Table => {
                if self.selected_column_index > 0 {
                    self.selected_column_index -= 1;
                    return Ok(Some(Action::Render));
                }
            }
            KeyCode::Right | KeyCode::Char('l') if self.view_mode == ViewMode::Table => {
                if self.selected_column_index < self.visible_fields.len().saturating_sub(1) {
                    self.selected_column_index += 1;
                    return Ok(Some(Action::Render));
                }
            }
            KeyCode::Char('y') => {
                if let Some(idx) = self.table_state.selected() {
                    if let Some(doc) = ctx.documents.get(idx) {
                        let val = if let Ok(id) = doc.get_object_id("_id") {
                            id.to_string()
                        } else if let Some(id) = doc.get("_id") {
                            id.to_string()
                        } else {
                            String::new()
                        };
                        if let Some(cb) = &mut ctx.clipboard {
                            let _ = cb.set_text(val);
                        }
                    }
                }
            }
            KeyCode::Char('Y') => {
                if let Some(idx) = self.table_state.selected() {
                    if let Some(doc) = ctx.documents.get(idx) {
                        if let Ok(json) = serde_json::to_string_pretty(doc) {
                            if let Some(cb) = &mut ctx.clipboard {
                                let _ = cb.set_text(json);
                            }
                        }
                    }
                }
            }
            KeyCode::Char('p') if self.view_mode == ViewMode::Table => {
                if let Some(idx) = self.table_state.selected() {
                    if let Some(doc) = ctx.documents.get(idx) {
                        if let Some(field) = self.visible_fields.get(self.selected_column_index) {
                            let val = doc.get(field).map(|v| v.to_string()).unwrap_or_default();
                            if let Some(cb) = &mut ctx.clipboard {
                                let _ = cb.set_text(val);
                            }
                        }
                    }
                }
            }
            KeyCode::Enter => {
                let selected_idx = self.table_state.selected();
                if let Some(idx) = selected_idx {
                    if let Some(doc) = ctx.documents.get(idx) {
                        if let Ok(json) = serde_json::to_string_pretty(doc) {
                            // Extract ID for title
                            let _id_str = if let Ok(id) = doc.get_object_id("_id") {
                                id.to_string()
                            } else {
                                "?".to_string()
                            };
                            return Ok(Some(Action::OpenJsonPopup(json))); // Need to pass ID too? For now just JSON
                        }
                    }
                }
            }
            _ => {}
        }
        Ok(None)
    }

    fn draw(
        &mut self,
        f: &mut Frame,
        area: Rect,
        is_active: bool,
        ctx: &MongoContext,
    ) -> Result<()> {
        let shortcuts = self.get_shortcuts();
        let shortcuts_str = shortcuts
            .iter()
            .map(|(k, v)| format!("{}: {}", k, v))
            .collect::<Vec<_>>()
            .join(" | ");

        let block = Block::default()
            .title("[4] Documents")
            .title_bottom(Line::from(shortcuts_str).alignment(Alignment::Center))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(if is_active {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            });

        if self.view_mode == ViewMode::Table {
            // Draw Table
            let header_cells = self.visible_fields.iter().enumerate().map(|(i, h)| {
                let style = if i == self.selected_column_index && is_active {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::Cyan)
                };
                Cell::from(h.as_str()).style(style)
            });
            let header = Row::new(header_cells).height(1).bottom_margin(1);

            let rows = ctx.documents.iter().map(|doc| {
                let cells = self
                    .visible_fields
                    .iter()
                    .map(|k| doc.get(k).map(|v| v.to_string()).unwrap_or_default());
                Row::new(cells)
            });

            // Widths
            let width = 100 / self.visible_fields.len().max(1) as u16;
            let constraints = vec![Constraint::Percentage(width); self.visible_fields.len()];

            let table = Table::new(rows, constraints)
                .header(header)
                .block(block)
                .row_highlight_style(Style::default().bg(Color::Blue));

            f.render_stateful_widget(table, area, &mut self.table_state);
        } else {
            // Draw JSON List
            let items: Vec<ListItem> = ctx
                .documents
                .iter()
                .map(|doc| {
                    // Simplified JSON view for list
                    let json = serde_json::to_string(doc).unwrap_or_default();
                    ListItem::new(json)
                })
                .collect();

            let list = List::new(items)
                .block(block)
                .highlight_style(Style::default().bg(Color::Blue));

            f.render_stateful_widget(list, area, &mut self.list_state);
        }

        Ok(())
    }
}
