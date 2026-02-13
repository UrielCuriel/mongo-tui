// use std::rc::Rc;
// use std::cell::RefCell;
use color_eyre::eyre::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, BorderType, Clear, List, ListItem, ListState, Paragraph, Row, Table, TableState, Wrap},
};
use tokio::sync::mpsc::UnboundedSender;
// use tracing::{info, error};
use tui_textarea::TextArea;
use syntect::{
    easy::HighlightLines,
    highlighting::ThemeSet,
    parsing::SyntaxSet,
    util::LinesWithEndings,
};
use syntect_tui::into_span;
use lazy_static::lazy_static;

use super::Component;
use crate::{
    action::Action,
    config::Config,
};

pub mod defs;
pub mod context;
pub mod pane_id;
pub mod registry;
pub mod parts;

use defs::{PopupState, QueryField};
use context::MongoContext;
use pane_id::PaneId;
use registry::PaneRegistry;
use parts::{
    connections::ConnectionsPane,
    databases::DatabasesPane,
    documents::DocumentsPane,
    query::QueryPane,
};

lazy_static! {
    static ref SYNTAX_SET: SyntaxSet = SyntaxSet::load_defaults_newlines();
    static ref THEME_SET: ThemeSet = ThemeSet::load_defaults();
}

pub struct MongoViewer {
    context: MongoContext,
    registry: PaneRegistry,
    popup_state: PopupState,
    
    // IDs for direct access/switching
    conn_pane_id: PaneId,
    db_pane_id: PaneId,
    query_pane_id: PaneId,
    doc_pane_id: PaneId,

    // Loading State
    is_loading: bool,
    loading_frame: usize,
}

impl Default for MongoViewer {
    fn default() -> Self {
        let mut registry = PaneRegistry::new();
        let context = MongoContext::new();
        
        // Create Panes
        let conn_pane_id = PaneId::new();
        let db_pane_id = PaneId::new();
        let query_pane_id = PaneId::new();
        let doc_pane_id = PaneId::new();
        
        registry.register(ConnectionsPane::new(conn_pane_id));
        registry.register(DatabasesPane::new(db_pane_id));
        registry.register(QueryPane::new(query_pane_id));
        registry.register(DocumentsPane::new(doc_pane_id));
        
        // Set initial active
        registry.set_active(conn_pane_id);

        Self {
            context,
            registry,
            popup_state: PopupState::None,
            conn_pane_id,
            db_pane_id,
            query_pane_id,
            doc_pane_id,
            is_loading: false,
            loading_frame: 0,
        }
    }
}

impl MongoViewer {
    pub fn new() -> Self {
        Self::default()
    }
    
    fn get_global_shortcuts(&self) -> Vec<(&'static str, &'static str)> {
        vec![
            ("q", "Quit"),
            ("?", "Help"),
            ("Tab", "Cycle"),
        ]
    }

    fn handle_popup_events(&mut self, key: KeyEvent) -> Result<Option<Action>> {
        match &mut self.popup_state {
            PopupState::Error(_) => {
                if let KeyCode::Esc | KeyCode::Enter = key.code {
                    self.popup_state = PopupState::None;
                    return Ok(Some(Action::Render));
                }
                return Ok(None);
            }
            PopupState::ConnectionManager { name, uri, is_editing_uri } => {
                match key.code {
                    KeyCode::Esc => {
                        self.popup_state = PopupState::None;
                        return Ok(Some(Action::Render));
                    }
                    KeyCode::Tab => {
                        *is_editing_uri = !*is_editing_uri;
                        return Ok(Some(Action::Render));
                    }
                    KeyCode::Enter => {
                         let n = name.lines().join("");
                         let u = uri.lines().join("");
                         if !n.is_empty() && !u.is_empty() {
                             self.popup_state = PopupState::None;
                             return Ok(Some(Action::SaveConnection(n, u)));
                         }
                    }
                    _ => {
                        if *is_editing_uri {
                            uri.input(key);
                        } else {
                            name.input(key);
                        }
                        return Ok(Some(Action::Render));
                    }
                }
            }
             PopupState::JsonViewer(_, _, offset) => {
                match key.code {
                    KeyCode::Esc => {
                        self.popup_state = PopupState::None;
                        return Ok(Some(Action::Render));
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        *offset = offset.saturating_add(1);
                        return Ok(Some(Action::Render));
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        *offset = offset.saturating_sub(1);
                        return Ok(Some(Action::Render));
                    }
                    _ => {}
                }
            }
             PopupState::Help(state) => {
                 match key.code {
                     KeyCode::Esc | KeyCode::Char('?') => {
                         self.popup_state = PopupState::None;
                         return Ok(Some(Action::Render));
                     }
                     KeyCode::Down | KeyCode::Char('j') => {
                         let i = match state.selected() {
                             Some(i) => i + 1,
                             None => 0,
                         };
                         state.select(Some(i));
                         return Ok(Some(Action::Render));
                     }
                     KeyCode::Up | KeyCode::Char('k') => {
                         let i = match state.selected() {
                             Some(i) => if i == 0 { 0 } else { i - 1 },
                             None => 0,
                         };
                         state.select(Some(i));
                         return Ok(Some(Action::Render));
                     }
                     _ => {}
                 }
            }
            PopupState::QueryBuilder { active_field } => {
                match key.code {
                    KeyCode::Esc => {
                        self.popup_state = PopupState::None;
                        self.context.input_validation_errors.clear();
                        return Ok(Some(Action::Render));
                    }
                    KeyCode::Tab => {
                        *active_field = match active_field {
                            QueryField::Filter => QueryField::Sort,
                            QueryField::Sort => QueryField::Projection,
                            QueryField::Projection => QueryField::Limit,
                            QueryField::Limit => QueryField::Filter,
                        };
                        return Ok(Some(Action::Render));
                    }
                    KeyCode::Enter => {
                         // Simplify validation: just trigger refresh
                         self.popup_state = PopupState::None;
                         self.context.pagination.current_page = 0; // Reset pagination
                         return Ok(Some(Action::RefreshDocuments));
                    }
                     _ => {
                        match active_field {
                            QueryField::Filter => { self.context.query_input.input(key); }
                            QueryField::Sort => { self.context.sort_input.input(key); }
                            QueryField::Projection => { self.context.projection_input.input(key); }
                            QueryField::Limit => { self.context.limit_input.input(key); }
                        }
                        return Ok(Some(Action::Render));
                    }
                }
            }
            PopupState::FieldSelector(state, all_fields, visible_fields) => {
                 match key.code {
                     KeyCode::Esc => {
                         self.popup_state = PopupState::None;
                         return Ok(Some(Action::Render));
                     }
                     KeyCode::Down | KeyCode::Char('j') => {
                         let i = match state.selected() {
                             Some(i) => if i >= all_fields.len().saturating_sub(1) { all_fields.len().saturating_sub(1) } else { i + 1 },
                             None => 0,
                         };
                         state.select(Some(i));
                         return Ok(Some(Action::Render));
                     }
                     KeyCode::Up | KeyCode::Char('k') => {
                         let i = match state.selected() {
                             Some(i) => if i == 0 { 0 } else { i - 1 },
                             None => 0,
                         };
                         state.select(Some(i));
                         return Ok(Some(Action::Render));
                     }
                     KeyCode::Enter | KeyCode::Char(' ') => {
                         if let Some(i) = state.selected() {
                             if let Some(field) = all_fields.get(i) {
                                 // Clone visible_fields to modify
                                 let mut new_visible = visible_fields.clone();
                                 if new_visible.contains(field) {
                                     new_visible.retain(|f| f != field);
                                 } else {
                                     new_visible.push(field.clone());
                                 }
                                 
                                 // Update the popup state with the new visible fields
                                 *visible_fields = new_visible.clone();
                                 
                                 // Dispatch action to update the main view
                                 return Ok(Some(Action::UpdateVisibleFields(new_visible)));
                             }
                         }
                         return Ok(Some(Action::Render));
                     }
                     _ => {}
                 }
                 return Ok(Some(Action::Render));
            }
            _ => {}
        }
        Ok(None)
    }

    // Popup Drawing Methods
    fn draw_error_popup(&self, f: &mut Frame, area: Rect, msg: &str) {
        let block = Block::default().title("Error").borders(Borders::ALL).style(Style::default().fg(Color::Red));
        let paragraph = Paragraph::new(msg).block(block).wrap(Wrap { trim: true });
        let area = centered_rect(60, 20, area);
        f.render_widget(Clear, area);
        f.render_widget(paragraph, area);
    }

    fn draw_connection_manager_popup(&self, f: &mut Frame, area: Rect, name: &TextArea, uri: &TextArea, is_editing_uri: bool) {
        let area = centered_rect(60, 40, area);
        f.render_widget(Clear, area);
        let block = Block::default().title("New Connection").borders(Borders::ALL);
        f.render_widget(block.clone(), area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(2)
            .constraints([Constraint::Length(3), Constraint::Length(3), Constraint::Min(1)])
            .split(area);

        let name_block = Block::default().borders(Borders::ALL).title("Name");
        let name_style = if !is_editing_uri { Style::default().fg(Color::Yellow) } else { Style::default() };
        let mut name_widget = name.clone();
        name_widget.set_block(name_block);
        name_widget.set_style(name_style);
        f.render_widget(&name_widget, chunks[0]);

        let uri_block = Block::default().borders(Borders::ALL).title("URI");
        let uri_style = if is_editing_uri { Style::default().fg(Color::Yellow) } else { Style::default() };
        let mut uri_widget = uri.clone();
        uri_widget.set_block(uri_block);
        uri_widget.set_style(uri_style);
        f.render_widget(&uri_widget, chunks[1]);
        
        let help = Paragraph::new("Tab: Switch | Enter: Save | Esc: Cancel").alignment(Alignment::Center);
        f.render_widget(help, chunks[2]);
    }

    fn draw_query_builder_popup(&self, f: &mut Frame, area: Rect, active_field: &QueryField) {
        let area = centered_rect(80, 80, area);
        f.render_widget(Clear, area);
        let block = Block::default().title("Query Builder").borders(Borders::ALL);
        f.render_widget(block, area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Percentage(40), // Filter
                Constraint::Percentage(20), // Sort
                Constraint::Percentage(20), // Projection
                Constraint::Length(3),      // Limit
                Constraint::Length(1),      // Help
            ])
            .split(area);
            
        let draw_input = |f: &mut Frame, chunk: Rect, title: &str, input: &TextArea, is_active: bool| {
             let mut widget = input.clone();
             widget.set_block(Block::default().borders(Borders::ALL).title(title));
             if is_active {
                 widget.set_style(Style::default().fg(Color::Yellow));
             }
             f.render_widget(&widget, chunk);
        };

        draw_input(f, chunks[0], "Filter (JSON)", &self.context.query_input, *active_field == QueryField::Filter);
        draw_input(f, chunks[1], "Sort (JSON)", &self.context.sort_input, *active_field == QueryField::Sort);
        draw_input(f, chunks[2], "Projection (JSON)", &self.context.projection_input, *active_field == QueryField::Projection);
        draw_input(f, chunks[3], "Limit (Number)", &self.context.limit_input, *active_field == QueryField::Limit);
        
        let help = Paragraph::new("Tab: Cycle | Enter: Apply | Esc: Cancel").alignment(Alignment::Center);
        f.render_widget(help, chunks[4]);
    }

    fn draw_json_popup(&self, f: &mut Frame, area: Rect, json: &str, title: &str, offset: usize) {
        let area = centered_rect(80, 80, area);
        f.render_widget(Clear, area);
        let block = Block::default().title(format!("JSON View: {}", title)).borders(Borders::ALL);
        
        let syntax = SYNTAX_SET.find_syntax_by_extension("json").unwrap_or_else(|| SYNTAX_SET.find_syntax_plain_text());
        // base16-ocean.dark is usually available in defaults, otherwise fall back to first available
        let theme = THEME_SET.themes.get("base16-ocean.dark").unwrap_or_else(|| &THEME_SET.themes.values().next().unwrap());
        let mut h = HighlightLines::new(syntax, theme);

        let lines: Vec<Line> = LinesWithEndings::from(json)
            .map(|line| {
                let ranges: Vec<(syntect::highlighting::Style, &str)> = h.highlight_line(line, &SYNTAX_SET).unwrap_or_default();
                let spans: Vec<Span> = ranges.into_iter()
                    .filter_map(|(style, content)| {
                        into_span((style, content)).ok().map(|mut span| {
                            span.style.bg = None; // Remove background color to adapt to terminal
                            span
                        })
                    })
                    .collect();
                Line::from(spans)
            })
            .collect();

        let paragraph = Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false })
            .scroll((offset as u16, 0));
        f.render_widget(paragraph, area);
    }
    
    fn draw_help_popup(&self, f: &mut Frame, area: Rect, state: &mut TableState) {
        let area = centered_rect(70, 70, area);
        f.render_widget(Clear, area);
        let block = Block::default().title("Help (Scroll: j/k)").borders(Borders::ALL);
        
        let mut rows = vec![];
        
        // Global
        rows.push(Row::new(vec!["Global", "q", "Quit"]));
        rows.push(Row::new(vec!["Global", "?", "Help"]));
        rows.push(Row::new(vec!["Global", "Tab", "Cycle Pane"]));
        rows.push(Row::new(vec!["Global", "1-4", "Switch Pane"]));

        // Panes
        let pane_shortcuts = self.registry.get_all_shortcuts();
        for (pane_name, shortcuts) in pane_shortcuts {
             for (key, action) in shortcuts {
                 rows.push(Row::new(vec![pane_name, key, action]));
             }
        }

        let table = Table::new(rows, [Constraint::Percentage(30), Constraint::Percentage(20), Constraint::Percentage(50)])
            .header(Row::new(vec!["Context", "Key", "Action"]).style(Style::default().add_modifier(Modifier::BOLD)))
            .block(block)
            .row_highlight_style(Style::default().bg(Color::Blue));
            
        f.render_stateful_widget(table, area, state);
    }
    fn draw_field_selector_popup(&self, f: &mut Frame, area: Rect, state: &mut ListState, all_fields: &[String], visible_fields: &[String]) {
        let area = centered_rect(50, 60, area);
        f.render_widget(Clear, area);
        let block = Block::default().title("Select Fields").borders(Borders::ALL);
        
        let items: Vec<ListItem> = all_fields
            .iter()
            .map(|field| {
                let is_selected = visible_fields.contains(field);
                let text = if is_selected {
                    format!("[x] {}", field)
                } else {
                    format!("[ ] {}", field)
                };
                ListItem::new(text).style(if is_selected {
                    Style::default().fg(Color::Cyan)
                } else {
                    Style::default()
                })
            })
            .collect();

        let list = List::new(items)
            .block(block)
            .highlight_style(Style::default().bg(Color::Blue));
            
        f.render_stateful_widget(list, area, state);
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

impl Component for MongoViewer {
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> Result<()> {
        self.context.action_tx = Some(tx);
        Ok(())
    }

    fn register_config_handler(&mut self, config: Config) -> Result<()> {
        self.context.connections = config.config.connections;
        Ok(())
    }

    fn handle_key_events(&mut self, key: KeyEvent) -> Result<Option<Action>> {
        // 1. Handle Popups first
        if !matches!(self.popup_state, PopupState::None) {
            return self.handle_popup_events(key);
        }

        // 2. Global Shortcuts
        match key.code {
            KeyCode::Char('q') => return Ok(Some(Action::Quit)),
            KeyCode::Char('?') => {
                 let mut state = TableState::default();
                 state.select(Some(0));
                 self.popup_state = PopupState::Help(state);
                 return Ok(Some(Action::Render));
            }
            KeyCode::Char('c') if self.registry.active_pane_id() == Some(self.conn_pane_id) => {
                 return Ok(Some(Action::OpenConnectionManager));
            }
            KeyCode::Tab => {
                self.registry.cycle_next();
                return Ok(Some(Action::Render));
            }
            KeyCode::Char('1') => {
                self.registry.set_active(self.conn_pane_id);
                return Ok(Some(Action::Render));
            }
             KeyCode::Char('2') => {
                self.registry.set_active(self.db_pane_id);
                return Ok(Some(Action::Render));
            }
             KeyCode::Char('3') => {
                self.registry.set_active(self.query_pane_id);
                return Ok(Some(Action::Render));
            }
             KeyCode::Char('4') => {
                self.registry.set_active(self.doc_pane_id);
                return Ok(Some(Action::Render));
            }
            _ => {}
        }

        // 3. Active Pane
        let result = self.registry.handle_key_event(key, &mut self.context)?;
        if let Some(action) = result {
            // Handle internal actions immediately
            match action {
                Action::OpenConnectionManager => {
                     let mut name = TextArea::default();
                     name.set_placeholder_text("Connection Name");
                     let mut uri = TextArea::default();
                     uri.set_placeholder_text("mongodb://localhost:27017");
                     self.popup_state = PopupState::ConnectionManager { name, uri, is_editing_uri: false };
                     return Ok(Some(Action::Render));
                }
                Action::OpenQueryBuilder => {
                    self.popup_state = PopupState::QueryBuilder { active_field: QueryField::Filter };
                     return Ok(Some(Action::Render));
                }
                Action::OpenJsonPopup(json, title) => {
                     self.popup_state = PopupState::JsonViewer(json, title, 0);
                     return Ok(Some(Action::Render));
                }
            Action::OpenFieldSelector(all_fields, visible_fields) => {
                 let mut state = ListState::default();
                 state.select(Some(0));
                 self.popup_state = PopupState::FieldSelector(state, all_fields.clone(), visible_fields.clone());
                 return Ok(Some(Action::Render));
            }
                _ => return Ok(Some(action))
            }
        }
        
        Ok(None)
    }

    fn update(&mut self, action: Action) -> Result<Option<Action>> {
        match &action {
            Action::Tick => {
                if self.is_loading {
                     self.loading_frame = self.loading_frame.wrapping_add(1);
                }
            }
            Action::SaveConnection(name, uri) => {
                self.context.connections.push(crate::config::Connection { name: name.clone(), uri: uri.clone() });
                self.context.selected_connection = Some(self.context.connections.len() - 1);
            }
            Action::Connect(uri) => {
                self.is_loading = true;
                let mongo_core = self.context.mongo_core.clone();
                let tx = self.context.action_tx.clone();
                let uri = uri.clone();
                tokio::spawn(async move {
                    if let Some(tx) = tx {
                         if let Err(e) = mongo_core.connect(&uri).await {
                             let _ = tx.send(Action::Error(e.to_string()));
                         } else {
                             let _ = tx.send(Action::RefreshDatabases);
                         }
                    }
                });
            }
            Action::RefreshDatabases => {
                self.is_loading = true;
                let mongo_core = self.context.mongo_core.clone();
                let tx = self.context.action_tx.clone();
                tokio::spawn(async move {
                    if let Some(tx) = tx {
                        match mongo_core.list_databases().await {
                            Ok(dbs) => { let _ = tx.send(Action::DatabasesLoaded(dbs)); }
                            Err(e) => { let _ = tx.send(Action::Error(e.to_string())); }
                        }
                    }
                });
            }
             Action::DatabasesLoaded(dbs) => {
                self.is_loading = false;
                self.context.databases = dbs.clone();
                self.registry.set_active(self.db_pane_id);
            }
             Action::RefreshDocuments => {
                 if let (Some(db_idx), Some(coll_idx)) = (self.context.selected_db_index, self.context.selected_coll_index) {
                     if let Some(db) = self.context.databases.get(db_idx) {
                         if let Some(coll) = db.collections.get(coll_idx) {
                             self.is_loading = true;
                             let db_name = db.name.clone();
                             let coll_name = coll.name.clone();
                             let mongo_core = self.context.mongo_core.clone();
                             let tx = self.context.action_tx.clone();
                             
                             let filter_str = self.context.query_input.lines().join("\n");
                             let sort_str = self.context.sort_input.lines().join("\n");
                             let proj_str = self.context.projection_input.lines().join("\n");
                             let limit_str = self.context.limit_input.lines().join("");
                             let current_page = self.context.pagination.current_page;
                             
                             // ... parsing logic (simplified here) ...
                             // Ideally move parsing to context helper or util
                             
                             tokio::spawn(async move {
                                  if let Some(tx) = tx {
                                      let limit = limit_str.parse::<i64>().unwrap_or(10);
                                      let skip = (current_page as i64 * limit) as u64;

                                      let filter = if !filter_str.trim().is_empty() {
                                          serde_json::from_str::<serde_json::Value>(&filter_str).ok().and_then(|v| mongo_core::bson::to_document(&v).ok())
                                      } else { None };
                                      let sort = if !sort_str.trim().is_empty() {
                                          serde_json::from_str::<serde_json::Value>(&sort_str).ok().and_then(|v| mongo_core::bson::to_document(&v).ok())
                                      } else { None };
                                       let proj = if !proj_str.trim().is_empty() {
                                          serde_json::from_str::<serde_json::Value>(&proj_str).ok().and_then(|v| mongo_core::bson::to_document(&v).ok())
                                      } else { None };
                                      
                                      let filter_clone_for_count = filter.clone();

                                      match mongo_core.find_documents(&db_name, &coll_name, filter, proj, sort, Some(limit), Some(skip)).await {
                                          Ok(docs) => { 
                                              // Fetch count
                                              match mongo_core.count_documents(&db_name, &coll_name, filter_clone_for_count).await {
                                                  Ok(count) => { let _ = tx.send(Action::DocumentsLoaded(docs, count)); }
                                                  Err(e) => { let _ = tx.send(Action::Error(e.to_string())); }
                                              }
                                          }
                                          Err(e) => { let _ = tx.send(Action::Error(e.to_string())); }
                                      }
                                  }
                             });
                         }
                     }
                 }
             }
             Action::DocumentsLoaded(docs, count) => {
                 self.is_loading = false;
                 self.context.documents = docs.clone();
                 self.context.pagination.total_count = Some(*count);
                 self.registry.set_active(self.doc_pane_id);
             }
             Action::NextPage => {
                 if let Some(total) = self.context.pagination.total_count {
                     let limit = self.context.limit_input.lines().join("").parse::<usize>().unwrap_or(10);
                     let current = self.context.pagination.current_page;
                     let max_pages = (total as usize + limit - 1) / limit;
                     if current + 1 < max_pages {
                         self.context.pagination.current_page += 1;
                         return Ok(Some(Action::RefreshDocuments));
                     }
                 }
             }
             Action::PreviousPage => {
                 if self.context.pagination.current_page > 0 {
                     self.context.pagination.current_page -= 1;
                     return Ok(Some(Action::RefreshDocuments));
                 }
             }
             Action::Error(msg) => {
                 self.is_loading = false;
                 self.popup_state = PopupState::Error(msg.clone());
             }
            _ => {}
        }

        self.registry.update_all(action, &mut self.context)?;
        Ok(None)
    }

    fn draw(&mut self, f: &mut Frame, area: Rect) -> Result<()> {
        let global_shortcuts = self.get_global_shortcuts();
        let global_shortcuts_str = global_shortcuts
            .iter()
            .map(|(k, v)| format!("{}: {}", k, v))
            .collect::<Vec<_>>()
            .join(" | ");

        let mut global_block = Block::default()
            .title(" Mongo TUI ")
            .title_alignment(Alignment::Center)
            .title_bottom(Line::from(global_shortcuts_str).alignment(Alignment::Center))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded);
        
        if self.is_loading {
             let spinner = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
             let frame = self.loading_frame / 5 % spinner.len();
             let text = format!(" Loading {} ", spinner[frame]);
             global_block = global_block.title_bottom(Line::from(text).style(Style::default().fg(Color::Cyan)).alignment(Alignment::Left));
        }

        f.render_widget(global_block.clone(), area);
        let inner_area = global_block.inner(area);

        let main_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(20), Constraint::Percentage(80)])
            .split(inner_area);

        let right_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(6), Constraint::Min(0)])
            .split(main_chunks[1]);

         let sidebar_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
            .split(main_chunks[0]);
            
         let active_pane_id = self.registry.active_pane_id();

         if let Some(pane) = self.registry.get_pane(self.conn_pane_id) {
             let is_active = active_pane_id == Some(self.conn_pane_id);
             pane.draw(f, sidebar_chunks[0], is_active, &self.context)?;
         }
         if let Some(pane) = self.registry.get_pane(self.db_pane_id) {
             let is_active = active_pane_id == Some(self.db_pane_id);
             pane.draw(f, sidebar_chunks[1], is_active, &self.context)?;
         }
         
         if let Some(pane) = self.registry.get_pane(self.query_pane_id) {
             let is_active = active_pane_id == Some(self.query_pane_id);
             pane.draw(f, right_chunks[0], is_active, &self.context)?;
         }
         if let Some(pane) = self.registry.get_pane(self.doc_pane_id) {
             let is_active = active_pane_id == Some(self.doc_pane_id);
             pane.draw(f, right_chunks[1], is_active, &self.context)?;
         }
         
         // Use swap to handle popup state mutable borrow
         let mut popup = std::mem::replace(&mut self.popup_state, PopupState::None);
         
         match &mut popup {
             PopupState::ConnectionManager { name, uri, is_editing_uri } => 
                 self.draw_connection_manager_popup(f, area, name, uri, *is_editing_uri),
             PopupState::QueryBuilder { active_field } => 
                 self.draw_query_builder_popup(f, area, active_field),
             PopupState::JsonViewer(json, title, offset) => 
                 self.draw_json_popup(f, area, json, title, *offset),
             PopupState::Help(state) => 
                 self.draw_help_popup(f, area, state),
              PopupState::Error(msg) => 
                  self.draw_error_popup(f, area, msg),
              PopupState::FieldSelector(state, all_fields, visible_fields) =>
                  self.draw_field_selector_popup(f, area, state, all_fields, visible_fields),
             _ => {}
         }
         
         self.popup_state = popup;
         
        Ok(())
    }
}
