use std::collections::{HashMap, HashSet};

use syntect::{
    easy::HighlightLines,
    highlighting::ThemeSet,
    parsing::SyntaxSet,
};
use syntect_tui::into_span;
use arboard::Clipboard;
use color_eyre::eyre::Result;
use crossterm::event::{KeyCode, KeyEvent};
use mongo_core::{DatabaseInfo, MongoCore};
use ratatui::{
    prelude::*,
    widgets::{
        Block, Borders, Cell, Clear, List, ListItem, ListState, Paragraph, Row, Table, TableState,
        Wrap,
    },
};
use tokio::sync::mpsc::UnboundedSender;
use tui_textarea::TextArea;

use super::Component;
use crate::{action::Action, config::{Config, Connection}};

#[derive(Debug, Clone, PartialEq)]
enum ActivePane {
    Connections,
    Databases,
    Query,
    Documents,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum QueryField {
    Filter,
    Sort,
    Limit,
    Projection,
}

#[derive(Debug, Clone)]
enum PopupState {
    None,
    ConnectionManager {
        name: TextArea<'static>,
        uri: TextArea<'static>,
        is_editing_uri: bool, // Toggle focus between name/uri
    },
    QueryBuilder {
        active_field: QueryField,
    },
    #[allow(dead_code)]
    JsonViewer(String, String, usize), // json, doc_id, offset
    FieldSelector(ListState),
}

#[derive(Debug, Clone, PartialEq)]
enum ViewMode {
    Table,
    Json,
}

#[derive(Debug, Clone)]
enum TreeItem {
    Database(usize), // index in self.databases
    Collection(usize, usize), // (db_index, coll_index)
}

pub struct MongoViewer {
    action_tx: Option<UnboundedSender<Action>>,
    mongo_core: MongoCore,

    // State
    connections: Vec<Connection>,
    databases: Vec<DatabaseInfo>,
    documents: Vec<mongo_core::bson::Document>,
    selected_conn_index: Option<usize>,
    
    // Database List State
    expanded_dbs: HashSet<String>,
    tree_items: Vec<TreeItem>,
    selected_tree_index: Option<usize>,

    selected_db_index: Option<usize>, // Keeping track of context for queries
    selected_coll_index: Option<usize>,

    // UI State
    active_pane: ActivePane,
    popup_state: PopupState,
    view_mode: ViewMode,

    // Inputs
    query_input: TextArea<'static>,
    projection_input: TextArea<'static>,
    sort_input: TextArea<'static>,
    limit_input: TextArea<'static>,
    input_validation_errors: HashMap<QueryField, String>,

    // Navigation
    document_table_state: TableState,
    document_list_state: ListState,
    selected_column_index: usize,

    // Config
    visible_fields: Vec<String>,
    all_fields: Vec<String>,
    expanded_docs: HashMap<usize, bool>, // For JSON view folding
    
    // System
    clipboard: Option<Clipboard>,
    syntax_set: SyntaxSet,
    theme_set: ThemeSet,
}

impl Default for MongoViewer {
    fn default() -> Self {
        let mut query = TextArea::default();
        query.set_placeholder_text("{}");
        let mut proj = TextArea::default();
        proj.set_placeholder_text("{}");
        let mut sort = TextArea::default();
        sort.set_placeholder_text("{}");
        let mut limit = TextArea::default();
        limit.set_placeholder_text("20");

        Self {
            action_tx: None,
            mongo_core: MongoCore::new(),
            connections: vec![],
            databases: vec![],
            documents: vec![],
            selected_conn_index: None,
            expanded_dbs: HashSet::new(),
            tree_items: vec![],
            selected_tree_index: None,
            selected_db_index: None,
            selected_coll_index: None,
            active_pane: ActivePane::Connections,
            popup_state: PopupState::None,
            view_mode: ViewMode::Table,
            query_input: query,
            projection_input: proj,
            sort_input: sort,
            limit_input: limit,
            input_validation_errors: HashMap::new(),
            document_table_state: TableState::default(),
            document_list_state: ListState::default(),
            selected_column_index: 0,
            visible_fields: vec!["_id".to_string()],
            all_fields: vec![],
            expanded_docs: HashMap::new(),
            clipboard: Clipboard::new().ok(),
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme_set: ThemeSet::load_defaults(),
        }
    }
}

impl MongoViewer {
    pub fn new() -> Self {
        Self::default()
    }

    fn cycle_pane(&mut self) {
        self.active_pane = match self.active_pane {
            ActivePane::Connections => ActivePane::Databases,
            ActivePane::Databases => ActivePane::Query,
            ActivePane::Query => ActivePane::Documents,
            ActivePane::Documents => ActivePane::Connections,
        };
    }

    fn toggle_view_mode(&mut self) {
        self.view_mode = match self.view_mode {
            ViewMode::Table => ViewMode::Json,
            ViewMode::Json => ViewMode::Table,
        };
    }
    
    fn rebuild_tree_items(&mut self) {
        self.tree_items.clear();
        for (db_idx, db) in self.databases.iter().enumerate() {
            self.tree_items.push(TreeItem::Database(db_idx));
            if self.expanded_dbs.contains(&db.name) {
                for (coll_idx, _) in db.collections.iter().enumerate() {
                    self.tree_items.push(TreeItem::Collection(db_idx, coll_idx));
                }
            }
        }
    }
}

impl Component for MongoViewer {
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> Result<()> {
        self.action_tx = Some(tx);
        Ok(())
    }

    fn register_config_handler(&mut self, config: Config) -> Result<()> {
        self.connections = config.config.connections;
        Ok(())
    }

    fn handle_key_events(&mut self, key: KeyEvent) -> Result<Option<Action>> {
        if let PopupState::ConnectionManager { name, uri, is_editing_uri } = &mut self.popup_state {
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
                     // Save connection
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
        
        if let PopupState::JsonViewer(json, _, offset) = &mut self.popup_state {
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
             return Ok(None);
        }

        if let PopupState::QueryBuilder { active_field } = &mut self.popup_state {
             match key.code {
                 KeyCode::Esc => {
                     self.popup_state = PopupState::None;
                     self.input_validation_errors.clear();
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
                 KeyCode::BackTab => {
                     *active_field = match active_field {
                         QueryField::Filter => QueryField::Limit,
                         QueryField::Sort => QueryField::Filter,
                         QueryField::Projection => QueryField::Sort,
                         QueryField::Limit => QueryField::Projection,
                     };
                     return Ok(Some(Action::Render));
                 }
                 KeyCode::Enter => {
                     // Validate
                     self.input_validation_errors.clear();
                     let mut is_valid = true;

                     // Filter
                     let filter_str = self.query_input.lines().join("\n");
                     if !filter_str.trim().is_empty() {
                         if let Err(e) = serde_json::from_str::<serde_json::Value>(&filter_str) {
                              self.input_validation_errors.insert(QueryField::Filter, format!("Invalid JSON: {}", e));
                              is_valid = false;
                         }
                     }

                     // Sort
                     let sort_str = self.sort_input.lines().join("\n");
                     if !sort_str.trim().is_empty() {
                         if let Err(e) = serde_json::from_str::<serde_json::Value>(&sort_str) {
                              self.input_validation_errors.insert(QueryField::Sort, format!("Invalid JSON: {}", e));
                              is_valid = false;
                         }
                     }

                     // Projection
                     let proj_str = self.projection_input.lines().join("\n");
                     if !proj_str.trim().is_empty() {
                         if let Err(e) = serde_json::from_str::<serde_json::Value>(&proj_str) {
                              self.input_validation_errors.insert(QueryField::Projection, format!("Invalid JSON: {}", e));
                              is_valid = false;
                         }
                     }

                     // Limit
                     let limit_str = self.limit_input.lines().join("");
                     if !limit_str.trim().is_empty() {
                         if limit_str.parse::<i64>().is_err() {
                             self.input_validation_errors.insert(QueryField::Limit, "Must be a number".to_string());
                             is_valid = false;
                         }
                     }

                     if is_valid {
                         self.popup_state = PopupState::None;
                         return Ok(Some(Action::RefreshDocuments));
                     } else {
                         return Ok(Some(Action::Render));
                     }
                 }
                 _ => {
                     match active_field {
                         QueryField::Filter => { 
                             self.query_input.input(key); 
                             self.input_validation_errors.remove(&QueryField::Filter);
                         }
                         QueryField::Sort => { 
                             self.sort_input.input(key); 
                             self.input_validation_errors.remove(&QueryField::Sort);
                         }
                         QueryField::Projection => { 
                             self.projection_input.input(key); 
                             self.input_validation_errors.remove(&QueryField::Projection);
                         }
                         QueryField::Limit => { 
                             self.limit_input.input(key); 
                             self.input_validation_errors.remove(&QueryField::Limit);
                         }
                     }
                     return Ok(Some(Action::Render));
                 }
             }
        }

        if let PopupState::FieldSelector(state) = &mut self.popup_state {
             match key.code {
                 KeyCode::Esc => {
                     self.popup_state = PopupState::None;
                     return Ok(Some(Action::Render));
                 }
                 KeyCode::Down | KeyCode::Char('j') => {
                     let i = match state.selected() {
                         Some(i) => {
                             if i >= self.all_fields.len() - 1 { 0 } else { i + 1 }
                         }
                         None => 0,
                     };
                     state.select(Some(i));
                     return Ok(Some(Action::Render));
                 }
                 KeyCode::Up | KeyCode::Char('k') => {
                     let i = match state.selected() {
                         Some(i) => {
                             if i == 0 { self.all_fields.len() - 1 } else { i - 1 }
                         }
                         None => 0,
                     };
                     state.select(Some(i));
                     return Ok(Some(Action::Render));
                 }
                 KeyCode::Enter | KeyCode::Char(' ') => {
                     if let Some(i) = state.selected() {
                         if let Some(field) = self.all_fields.get(i) {
                             if self.visible_fields.contains(field) {
                                 self.visible_fields.retain(|f| f != field);
                             } else {
                                 self.visible_fields.push(field.clone());
                             }
                         }
                     }
                     return Ok(Some(Action::Render));
                 }
                 _ => {}
             }
             return Ok(None);
        }

        match key.code {
            KeyCode::Char('f') => {
                let mut state = ListState::default();
                state.select(Some(0));
                self.popup_state = PopupState::FieldSelector(state);
                return Ok(Some(Action::Render));
            }
            KeyCode::Char('c') if self.active_pane == ActivePane::Connections => {
                 let mut name = TextArea::default();
                 name.set_placeholder_text("Connection Name");
                 let mut uri = TextArea::default();
                 uri.set_placeholder_text("mongodb://localhost:27017");
                 self.popup_state = PopupState::ConnectionManager { name, uri, is_editing_uri: false };
                 return Ok(Some(Action::Render));
            }
            KeyCode::Tab => {
                self.cycle_pane();
                return Ok(Some(Action::Render));
            }
            KeyCode::Char('q') => return Ok(Some(Action::Quit)),
            KeyCode::Char('v') => {
                self.toggle_view_mode();
                return Ok(Some(Action::Render));
            }
            _ => {
                match self.active_pane {
                    ActivePane::Connections => {
                          match key.code {
                              KeyCode::Char('j') | KeyCode::Down => {
                                  if let Some(idx) = self.selected_conn_index {
                                      if idx + 1 < self.connections.len() {
                                          self.selected_conn_index = Some(idx + 1);
                                          return Ok(Some(Action::Render));
                                      }
                                  } else if !self.connections.is_empty() {
                                      self.selected_conn_index = Some(0);
                                      return Ok(Some(Action::Render));
                                  }
                              }
                              KeyCode::Char('k') | KeyCode::Up => {
                                  if let Some(idx) = self.selected_conn_index {
                                      if idx > 0 {
                                          self.selected_conn_index = Some(idx - 1);
                                          return Ok(Some(Action::Render));
                                      }
                                  }
                              }
                              KeyCode::Enter => {
                                  if let Some(idx) = self.selected_conn_index {
                                      if let Some(conn) = self.connections.get(idx) {
                                          return Ok(Some(Action::Connect(conn.uri.clone())));
                                      }
                                  }
                              }
                              _ => {}
                          }
                    }
                    ActivePane::Databases => {
                          match key.code {
                              KeyCode::Char('j') | KeyCode::Down => {
                                  if let Some(idx) = self.selected_tree_index {
                                      if idx + 1 < self.tree_items.len() {
                                          self.selected_tree_index = Some(idx + 1);
                                          return Ok(Some(Action::Render));
                                      }
                                  } else if !self.tree_items.is_empty() {
                                      self.selected_tree_index = Some(0);
                                      return Ok(Some(Action::Render));
                                  }
                              }
                              KeyCode::Char('k') | KeyCode::Up => {
                                  if let Some(idx) = self.selected_tree_index {
                                      if idx > 0 {
                                          self.selected_tree_index = Some(idx - 1);
                                          return Ok(Some(Action::Render));
                                      }
                                  }
                              }
                              KeyCode::Enter | KeyCode::Char(' ') => {
                                  if let Some(idx) = self.selected_tree_index {
                                      if let Some(item) = self.tree_items.get(idx) {
                                          match item {
                                              TreeItem::Database(db_idx) => {
                                                  if let Some(db) = self.databases.get(*db_idx) {
                                                      let name = db.name.clone();
                                                      if self.expanded_dbs.contains(&name) {
                                                          self.expanded_dbs.remove(&name);
                                                      } else {
                                                          self.expanded_dbs.insert(name);
                                                      }
                                                      self.rebuild_tree_items();
                                                      return Ok(Some(Action::Render));
                                                  }
                                              },
                                              TreeItem::Collection(db_idx, coll_idx) => {
                                                  // Select and fetch
                                                  self.selected_db_index = Some(*db_idx);
                                                  self.selected_coll_index = Some(*coll_idx);
                                                  return Ok(Some(Action::RefreshDocuments));
                                              }
                                          }
                                      }
                                  }
                              }
                              _ => {}
                          }
                    }
                    ActivePane::Documents => {
                        match key.code {
                            KeyCode::Down | KeyCode::Char('j') => {
                                let len = self.documents.len();
                                if len > 0 {
                                    if self.view_mode == ViewMode::Table {
                                        let i = match self.document_table_state.selected() {
                                            Some(i) => if i >= len - 1 { len - 1 } else { i + 1 },
                                            None => 0,
                                        };
                                        self.document_table_state.select(Some(i));
                                        // Sync list state
                                        self.document_list_state.select(Some(i));
                                    } else {
                                        let i = match self.document_list_state.selected() {
                                            Some(i) => if i >= len - 1 { len - 1 } else { i + 1 },
                                            None => 0,
                                        };
                                        self.document_list_state.select(Some(i));
                                        // Sync table state
                                        self.document_table_state.select(Some(i));
                                    }
                                    return Ok(Some(Action::Render));
                                }
                            }
                            KeyCode::Up | KeyCode::Char('k') => {
                                let len = self.documents.len();
                                if len > 0 {
                                     if self.view_mode == ViewMode::Table {
                                        let i = match self.document_table_state.selected() {
                                            Some(i) => if i == 0 { 0 } else { i - 1 },
                                            None => 0,
                                        };
                                        self.document_table_state.select(Some(i));
                                        // Sync list state
                                        self.document_list_state.select(Some(i));
                                    } else {
                                        let i = match self.document_list_state.selected() {
                                            Some(i) => if i == 0 { 0 } else { i - 1 },
                                            None => 0,
                                        };
                                        self.document_list_state.select(Some(i));
                                        // Sync table state
                                        self.document_table_state.select(Some(i));
                                    }
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
                                if let Some(idx) = self.document_table_state.selected() {
                                    if let Some(doc) = self.documents.get(idx) {
                                        let val = if let Ok(id) = doc.get_object_id("_id") {
                                            id.to_string()
                                        } else if let Some(id) = doc.get("_id") {
                                            id.to_string()
                                        } else {
                                            String::new()
                                        };
                                        
                                        if let Some(clipboard) = &mut self.clipboard {
                                            let _ = clipboard.set_text(val);
                                        }
                                    }
                                }
                            }
                            KeyCode::Char('Y') => {
                                if let Some(idx) = self.document_table_state.selected() {
                                    if let Some(doc) = self.documents.get(idx) {
                                        if let Ok(json) = serde_json::to_string_pretty(doc) {
                                            if let Some(clipboard) = &mut self.clipboard {
                                                let _ = clipboard.set_text(json);
                                            }
                                        }
                                    }
                                }
                            }
                            KeyCode::Char('p') if self.view_mode == ViewMode::Table => {
                                if let Some(idx) = self.document_table_state.selected() {
                                    if let Some(doc) = self.documents.get(idx) {
                                        if let Some(field) = self.visible_fields.get(self.selected_column_index) {
                                            let val = doc.get(field).map(|v| v.to_string()).unwrap_or_default();
                                            if let Some(clipboard) = &mut self.clipboard {
                                                let _ = clipboard.set_text(val);
                                            }
                                        }
                                    }
                                }
                            }
                            KeyCode::Char('P') if self.view_mode == ViewMode::Table => {
                                if let Some(field) = self.visible_fields.get(self.selected_column_index) {
                                    if let Some(clipboard) = &mut self.clipboard {
                                        let _ = clipboard.set_text(field.clone());
                                    }
                                }
                            }
                            KeyCode::Enter => {
                                // View full document
                                let selected_idx = if self.view_mode == ViewMode::Table {
                                    self.document_table_state.selected()
                                } else {
                                    self.document_list_state.selected()
                                };

                                if let Some(idx) = selected_idx {
                                    if let Some(doc) = self.documents.get(idx) {
                                        if let Ok(json) = serde_json::to_string_pretty(doc) {
                                            let id_str = if let Ok(id) = doc.get_object_id("_id") {
                                                id.to_string()
                                            } else if let Some(id) = doc.get("_id") {
                                                id.to_string()
                                            } else {
                                                "?".to_string()
                                            };
                                            self.popup_state = PopupState::JsonViewer(json, id_str, 0);
                                            return Ok(Some(Action::Render));
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    ActivePane::Query => {
                        match key.code {
                            KeyCode::Enter => {
                                // Open Query Builder
                                self.popup_state = PopupState::QueryBuilder { active_field: QueryField::Filter };
                                return Ok(Some(Action::Render));
                            }
                            _ => {} 
                        }
                    }
                }
            }
        }
        Ok(None)
    }


    fn update(&mut self, action: Action) -> Result<Option<Action>> {
        if let Some(action_tx) = &self.action_tx {
            let tx = action_tx.clone();
            match action {
                Action::SaveConnection(name, uri) => {
                     // TODO: Persist to config
                     self.connections.push(Connection { name, uri });
                     // Need to trigger save... but component doesn't have mutable access to Config struct properly 
                     // unless we inject a save callback or similar.
                     // For prototype, we just update in-memory.
                     // A proper way would be emitting an action that App handles to update config.
                     self.selected_conn_index = Some(self.connections.len() - 1);
                }
                Action::Connect(uri) => {
                    let mongo_core = self.mongo_core.clone();
                    let tx_clone = tx.clone();
                    tokio::spawn(async move {
                        if let Err(e) = mongo_core.connect(&uri).await {
                             let _ = tx_clone.send(Action::Error(e.to_string()));
                        } else {
                             let _ = tx_clone.send(Action::RefreshDatabases);
                        }
                    });
                }
                Action::RefreshDatabases => {
                    let mongo_core = self.mongo_core.clone();
                    let tx_clone = tx.clone();
                    tokio::spawn(async move {
                        match mongo_core.list_databases().await {
                            Ok(dbs) => {
                                let _ = tx_clone.send(Action::DatabasesLoaded(dbs));
                            },
                            Err(e) => {
                                let _ = tx_clone.send(Action::Error(e.to_string()));
                            }
                        }
                    });
                }
                Action::DatabasesLoaded(dbs) => {
                    self.databases = dbs;
                    self.active_pane = ActivePane::Databases;
                    self.rebuild_tree_items();
                    self.selected_tree_index = if !self.tree_items.is_empty() { Some(0) } else { None };
                }
                Action::SelectDatabase(idx) => {
                    // Deprecated action used by old logic, but keeping for compatibility if needed or just removing handling logic
                    self.selected_db_index = Some(idx);
                }
                Action::SelectCollection(idx) => {
                    self.selected_coll_index = Some(idx);
                    let _ = tx.send(Action::RefreshDocuments);
                }
                Action::RefreshDocuments => {
                    if let (Some(db_idx), Some(coll_idx)) = (self.selected_db_index, self.selected_coll_index) {
                         if let Some(db) = self.databases.get(db_idx) {
                             if let Some(coll) = db.collections.get(coll_idx) {
                                 let db_name = db.name.clone();
                                 let coll_name = coll.name.clone();
                                 let mongo_core = self.mongo_core.clone();
                                 let tx_clone = tx.clone();
                                 
                                 // Simple parsing for now
                                 let filter_str = self.query_input.lines().join("\n");
                                 let filter: Option<mongo_core::bson::Document> = if !filter_str.trim().is_empty() {
                                     match serde_json::from_str::<serde_json::Value>(&filter_str) {
                                         Ok(val) => match mongo_core::bson::to_document(&val) {
                                             Ok(doc) => Some(doc),
                                             Err(e) => {
                                                 let _ = tx.send(Action::Error(format!("Invalid BSON: {}", e)));
                                                 None
                                             }
                                         },
                                         Err(e) => {
                                              let _ = tx.send(Action::Error(format!("Invalid JSON: {}", e)));
                                              None
                                         }
                                     }
                                 } else {
                                     None
                                 };
                                 
                                 let projection_str = self.projection_input.lines().join("\n");
                                 let projection: Option<mongo_core::bson::Document> = if !projection_str.trim().is_empty() {
                                     match serde_json::from_str::<serde_json::Value>(&projection_str) {
                                         Ok(val) => match mongo_core::bson::to_document(&val) {
                                             Ok(doc) => Some(doc),
                                             Err(_) => None 
                                         },
                                         Err(_) => None
                                     }
                                 } else {
                                     None
                                 };

                                 let sort_str = self.sort_input.lines().join("\n");
                                 let sort: Option<mongo_core::bson::Document> = if !sort_str.trim().is_empty() {
                                     match serde_json::from_str::<serde_json::Value>(&sort_str) {
                                         Ok(val) => match mongo_core::bson::to_document(&val) {
                                             Ok(doc) => Some(doc),
                                             Err(_) => None 
                                         },
                                         Err(_) => None
                                     }
                                 } else {
                                     None
                                 };

                                 let limit_str = self.limit_input.lines().join("");
                                 let limit = limit_str.parse::<i64>().ok();
                                 let skip = None;
                                 
                                 tokio::spawn(async move {
                                     match mongo_core.find_documents(&db_name, &coll_name, filter, projection, sort, limit, skip).await {
                                         Ok(docs) => {
                                             let _ = tx_clone.send(Action::DocumentsLoaded(docs));
                                         },
                                         Err(e) => {
                                             let _ = tx_clone.send(Action::Error(e.to_string()));
                                         }
                                     }
                                 });
                             }
                         }
                    }
                }
                Action::DocumentsLoaded(docs) => {
                    self.documents = docs;
                    
                    // Reset visible fields to default
                    self.visible_fields = vec!["_id".to_string()];

                    // Update all_fields based on keys in the first few documents
                    let mut fields = std::collections::HashSet::new();
                    for doc in self.documents.iter().take(20) {
                        for k in doc.keys() {
                            fields.insert(k.clone());
                        }
                    }
                    let mut sorted_fields: Vec<String> = fields.into_iter().collect();
                    sorted_fields.sort();
                    self.all_fields = sorted_fields;
                    
                    // Add a few more fields to visible by default if available, but not too many
                    for field in self.all_fields.iter() {
                        if field != "_id" && self.visible_fields.len() < 5 {
                             self.visible_fields.push(field.clone());
                        }
                    }
                    
                    // Reset selection
                    self.document_table_state.select(if !self.documents.is_empty() { Some(0) } else { None });
                    self.document_list_state.select(if !self.documents.is_empty() { Some(0) } else { None });
                }
                _ => {}
            }
        }
        Ok(None)
    }

    fn draw(&mut self, f: &mut Frame, area: Rect) -> Result<()> {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),
                Constraint::Length(1), // Footer
            ])
            .split(area);

        let main_area = layout[0];
        let footer_area = layout[1];

        // Main Layout: Sidebar (Left) | Content (Right)
        let main_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(20), Constraint::Percentage(80)])
            .split(main_area);

        // Right Panel: Query Bar (Top) | Documents (Bottom)
        let right_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(6), Constraint::Min(0)])
            .split(main_chunks[1]);

        self.draw_sidebar(f, main_chunks[0]);
        self.draw_query_bar(f, right_chunks[0]);
        self.draw_documents(f, right_chunks[1]);
        self.draw_footer(f, footer_area);

        // Use swap to avoid borrow checker issues when we need both &mut self (for rendering stateful widgets maybe?) 
        // or access to self fields while holding a mutable reference to popup_state content.
        let mut popup = std::mem::replace(&mut self.popup_state, PopupState::None);
        
        match &mut popup {
            PopupState::JsonViewer(json, id, offset) => self.draw_json_popup(f, area, json, id, *offset),
            PopupState::ConnectionManager { name, uri, is_editing_uri } => {
                self.draw_connection_manager_popup(f, area, name, uri, *is_editing_uri)
            },
            PopupState::QueryBuilder { active_field } => {
                self.draw_query_builder_popup(f, area, active_field);
            },
            PopupState::FieldSelector(state) => {
                self.draw_field_selector(f, area, state);
            },
            PopupState::None => {}
        }
        
        self.popup_state = popup;

        Ok(())
    }
}


impl MongoViewer {
    fn draw_sidebar(&self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(30), // Connections
                Constraint::Percentage(70), // Databases Tree
            ])
            .split(area);

        // Connections List
        let conn_block = Block::default()
            .title("Connections (c: add)")
            .borders(Borders::ALL)
            .border_style(if self.active_pane == ActivePane::Connections {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            });

        let conn_items: Vec<ListItem> = self.connections
            .iter()
            .map(|conn| ListItem::new(conn.name.clone()))
            .collect();

        let mut conn_state = ListState::default();
        conn_state.select(self.selected_conn_index);

        let conn_list = List::new(conn_items)
            .block(conn_block)
            .highlight_style(Style::default().bg(Color::Blue));
        f.render_stateful_widget(conn_list, chunks[0], &mut conn_state);

        // Databases Tree
        let tree_block = Block::default()
            .title("Databases")
            .borders(Borders::ALL)
            .border_style(if self.active_pane == ActivePane::Databases {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            });

        let tree_items_widgets: Vec<ListItem> = self.tree_items.iter().map(|item| {
            match item {
                TreeItem::Database(idx) => {
                    let db = &self.databases[*idx];
                    let prefix = if self.expanded_dbs.contains(&db.name) { "v " } else { "> " };
                    ListItem::new(format!("{}{}", prefix, db.name)).style(Style::default().add_modifier(Modifier::BOLD))
                },
                TreeItem::Collection(db_idx, coll_idx) => {
                    let db = &self.databases[*db_idx];
                    let coll = &db.collections[*coll_idx];
                    ListItem::new(format!("  • {}", coll.name))
                }
            }
        }).collect();

        let mut tree_state = ListState::default();
        tree_state.select(self.selected_tree_index);

        let tree_list = List::new(tree_items_widgets)
            .block(tree_block)
            .highlight_style(Style::default().bg(Color::Blue));
        f.render_stateful_widget(tree_list, chunks[1], &mut tree_state);
    }

    fn draw_query_bar(&self, f: &mut Frame, area: Rect) {
        let block = Block::default()
            .title("Query Filter (Enter to edit)")
            .borders(Borders::ALL)
            .border_style(if self.active_pane == ActivePane::Query {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            });

        // Show a summary of the filter or "Empty"
        let filter_text = self.query_input.lines().join(" ");
        let display_text = if filter_text.trim().is_empty() {
             "{}"
        } else {
             &filter_text
        };
        
        let p = Paragraph::new(display_text)
            .block(block)
            .wrap(Wrap { trim: true });

        f.render_widget(p, area); 
    }

    fn draw_documents(&mut self, f: &mut Frame, area: Rect) {
        if self.selected_coll_index.is_none() {
             // Show help text or empty
             let block = Block::default().borders(Borders::ALL).title("Info");
             f.render_widget(Paragraph::new("Select a collection to view documents.").block(block), area);
             return;
        }

        // Breadcrumb: <conn> / <db> / <coll>
        let conn_name = self.selected_conn_index
            .and_then(|i| self.connections.get(i))
            .map(|c| c.name.as_str())
            .unwrap_or("?");
        
        let db_name = self.selected_db_index
            .and_then(|i| self.databases.get(i))
            .map(|d| d.name.as_str())
            .unwrap_or("?");

        let coll_name = if let (Some(db_idx), Some(coll_idx)) = (self.selected_db_index, self.selected_coll_index) {
             self.databases.get(db_idx)
                .and_then(|db| db.collections.get(coll_idx))
                .map(|c| c.name.as_str())
                .unwrap_or("?")
        } else {
            "?"
        };
        let breadcrumb = format!(" {} / {} / {} ", conn_name, db_name, coll_name);

        // View Mode
        let view_mode_str = if self.view_mode == ViewMode::Table { " Table " } else { " JSON " };

        // Count
        let count_str = format!(" Count: {} ", self.documents.len());
        
        // Shortcuts (only if a document is selected)
        let has_selection = self.document_table_state.selected().is_some();
        let shortcuts_str = if has_selection {
            if self.view_mode == ViewMode::Table {
                 " y: copy _id | Y: copy doc | p: copy val | P: copy key "
            } else {
                 " y: copy _id | Y: copy doc "
            }
        } else {
            ""
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(if self.active_pane == ActivePane::Documents {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            })
            .title_top(Line::from(breadcrumb).alignment(Alignment::Left))
            .title_top(Line::from(view_mode_str).alignment(Alignment::Right))
            .title_bottom(Line::from(shortcuts_str).alignment(Alignment::Left))
            .title_bottom(Line::from(count_str).alignment(Alignment::Right));
            
        if self.view_mode == ViewMode::Table {
             let header_cells = self.visible_fields.iter().enumerate().map(|(i, h)| {
                 let style = if i == self.selected_column_index && self.active_pane == ActivePane::Documents {
                     Style::default().fg(Color::Red).add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
                 } else {
                     Style::default().fg(Color::Red)
                 };
                 Cell::from(h.clone()).style(style)
             });
             let header = Row::new(header_cells).height(1).bottom_margin(1);
             
             let rows: Vec<Row> = self.documents.iter().enumerate().map(|(row_idx, doc)| {
                 let is_row_selected = self.document_table_state.selected() == Some(row_idx);
                 let cells = self.visible_fields.iter().enumerate().map(|(col_idx, field)| {
                     let val = doc.get(field).map(|v| v.to_string()).unwrap_or_default();
                     let mut cell = Cell::from(val);
                     if is_row_selected && col_idx == self.selected_column_index && self.active_pane == ActivePane::Documents {
                         cell = cell.style(Style::default().bg(Color::LightBlue).fg(Color::Black));
                     }
                     cell
                 });
                 Row::new(cells).height(1)
             }).collect();
             
             let widths = self.visible_fields.iter().map(|_| Constraint::Min(10)).collect::<Vec<_>>();
             let table = Table::new(rows, widths)
                 .header(header)
                 .block(block)
                 .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED));
                 
             f.render_stateful_widget(table, area, &mut self.document_table_state);
        } else {
             // JSON View
             let items: Vec<ListItem> = self.documents.iter().enumerate().map(|(i, doc)| {
                 let is_expanded = *self.expanded_docs.get(&i).unwrap_or(&false);
                 let content = if is_expanded {
                     serde_json::to_string_pretty(&doc).unwrap_or_default()
                 } else {
                     let mut summary = String::from("{ ");
                     if let Ok(id) = doc.get_object_id("_id") {
                         summary.push_str(&format!("_id: {}, ", id));
                     } else if let Some(id) = doc.get("_id") {
                         summary.push_str(&format!("_id: {}, ", id));
                     }


                     
                     let mut count = 0;
                     for (k, v) in doc {
                         if k == "_id" { continue; }
                         if count >= 3 { break; }
                         summary.push_str(&format!("{}: {}, ", k, v));
                         count += 1;
                     }
                     summary.push_str("... }");
                     summary
                 };
                 
                 ListItem::new(content)
             }).collect();
             
             let list = List::new(items)
                 .block(block)
                 .highlight_style(Style::default().bg(Color::Blue));
                 
             f.render_stateful_widget(list, area, &mut self.document_list_state);
        }
    }

    fn draw_json_popup(&self, f: &mut Frame, area: Rect, json: &str, doc_id: &str, offset: usize) {
        let popup_area = centered_rect(area, 80, 80);
        f.render_widget(Clear, popup_area);
        
        // Breadcrumb: <conn> / <db> / <coll> / <id>
        let conn_name = self.selected_conn_index
            .and_then(|i| self.connections.get(i))
            .map(|c| c.name.as_str())
            .unwrap_or("?");
        
        let db_name = self.selected_db_index
            .and_then(|i| self.databases.get(i))
            .map(|d| d.name.as_str())
            .unwrap_or("?");

        let coll_name = if let (Some(db_idx), Some(coll_idx)) = (self.selected_db_index, self.selected_coll_index) {
             self.databases.get(db_idx)
                .and_then(|db| db.collections.get(coll_idx))
                .map(|c| c.name.as_str())
                .unwrap_or("?")
        } else {
            "?"
        };
        let breadcrumb = format!(" {} / {} / {} / {} ", conn_name, db_name, coll_name, doc_id);

        let block = Block::default()
            .borders(Borders::ALL)
            .title_top(Line::from(breadcrumb).alignment(Alignment::Left));
            
        // Syntax Highlighting
        let syntax = self.syntax_set.find_syntax_by_extension("json").unwrap();
        let theme = &self.theme_set.themes["base16-ocean.dark"];
        let mut h = HighlightLines::new(syntax, theme);
        
        let mut spans = Vec::new();
        for line in json.lines() {
            let ranges = h.highlight_line(line, &self.syntax_set).unwrap();
            let line_spans: Vec<Span> = ranges.into_iter()
                .filter_map(|(style, content)| {
                    into_span((style, content)).ok().map(|mut span| {
                        // Force background to be transparent/reset to avoid clashing with TUI theme
                        span.style = span.style.bg(Color::Reset);
                        span
                    })
                })
                .collect();
            spans.push(Line::from(line_spans));
        }
        
        let p = Paragraph::new(spans)
            .block(block)
            .wrap(Wrap { trim: false })
            .scroll((offset as u16, 0));
        f.render_widget(p, popup_area);
    }

    fn draw_connection_manager_popup(&self, f: &mut Frame, area: Rect, name: &TextArea<'static>, uri: &TextArea<'static>, is_editing_uri: bool) {
        let popup_area = centered_rect(area, 60, 40);
        f.render_widget(Clear, popup_area);

        let block = Block::default()
            .title("New Connection (Tab to switch, Enter to save, Esc to cancel)")
            .borders(Borders::ALL);
        f.render_widget(block, popup_area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(2)
            .constraints([
                Constraint::Length(3), // Name input
                Constraint::Length(3), // URI input
                Constraint::Min(0),
            ])
            .split(popup_area);

        let mut name_widget = name.clone();
        name_widget.set_block(
            Block::default()
                .borders(Borders::ALL)
                .title("Name")
                .border_style(if !is_editing_uri { Style::default().fg(Color::Yellow) } else { Style::default() }),
        );
        
        let mut uri_widget = uri.clone();
        uri_widget.set_block(
            Block::default()
                .borders(Borders::ALL)
                .title("URI")
                .border_style(if is_editing_uri { Style::default().fg(Color::Yellow) } else { Style::default() }),
        );

        f.render_widget(&name_widget, chunks[0]);
        f.render_widget(&uri_widget, chunks[1]);
    }

    fn draw_query_builder_popup(&self, f: &mut Frame, area: Rect, active_field: &QueryField) {
        let popup_area = centered_rect(area, 70, 70);
        f.render_widget(Clear, popup_area);

        let block = Block::default()
            .title("Query Builder (Tab: Next Field | Enter: Run | Esc: Cancel)")
            .borders(Borders::ALL);
        f.render_widget(block, popup_area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(2)
            .constraints([
                Constraint::Percentage(40), // Filter
                Constraint::Percentage(20), // Sort
                Constraint::Percentage(20), // Projection
                Constraint::Length(3),      // Limit
                Constraint::Min(0),
            ])
            .split(popup_area);

        let get_style_and_title = |field: QueryField, title: &str| {
            let is_active = *active_field == field;
            let error = self.input_validation_errors.get(&field);
            
            let mut style = Style::default();
            if is_active {
                style = style.fg(Color::Yellow);
            }
            if error.is_some() {
                style = style.fg(Color::Red);
            }

            let title_text = if let Some(err) = error {
                format!("{} - Error: {}", title, err)
            } else {
                title.to_string()
            };
            
            (style, title_text)
        };

        let (style, title) = get_style_and_title(QueryField::Filter, "Filter (JSON)");
        let mut filter_widget = self.query_input.clone();
        filter_widget.set_block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(style),
        );
        f.render_widget(&filter_widget, chunks[0]);

        let (style, title) = get_style_and_title(QueryField::Sort, "Sort (JSON)");
        let mut sort_widget = self.sort_input.clone();
        sort_widget.set_block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(style),
        );
        f.render_widget(&sort_widget, chunks[1]);
        
        let (style, title) = get_style_and_title(QueryField::Projection, "Projection (JSON)");
        let mut proj_widget = self.projection_input.clone();
        proj_widget.set_block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(style),
        );
        f.render_widget(&proj_widget, chunks[2]);

        let (style, title) = get_style_and_title(QueryField::Limit, "Limit");
        let mut limit_widget = self.limit_input.clone();
        limit_widget.set_block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(style),
        );
        f.render_widget(&limit_widget, chunks[3]);
    }

    fn draw_field_selector(&self, f: &mut Frame, area: Rect, state: &mut ListState) {
        let popup_area = centered_rect(area, 60, 60);
        f.render_widget(Clear, popup_area);
        
        let block = Block::default()
            .title("Select Fields (Space/Enter to toggle)")
            .borders(Borders::ALL);
            
        let items: Vec<ListItem> = self.all_fields.iter().map(|field| {
            let is_selected = self.visible_fields.contains(field);
            let checkbox = if is_selected { "[x] " } else { "[ ] " };
            ListItem::new(format!("{}{}", checkbox, field))
        }).collect();
        
        let list = List::new(items)
            .block(block)
            .highlight_style(Style::default().bg(Color::Blue));
            
        f.render_stateful_widget(list, popup_area, state);
    }
    fn draw_footer(&self, f: &mut Frame, area: Rect) {
        let help_text = match &self.popup_state {
            PopupState::ConnectionManager { .. } => "Tab: Next Field | Enter: Save | Esc: Cancel",
            PopupState::QueryBuilder { .. } => "Tab: Next Field | Enter: Run | Esc: Cancel",
            PopupState::JsonViewer(..) => "Esc: Close",
            PopupState::FieldSelector(_) => "Space/Enter: Toggle | Esc: Close | \u{2191}/\u{2193}: Nav",
            PopupState::None => match self.active_pane {
                ActivePane::Connections => "c: New | Enter: Connect | \u{2191}/\u{2193}: Nav | Tab: Next Pane | q: Quit",
                ActivePane::Databases => "Space/Enter: Expand/Select | \u{2191}/\u{2193}: Nav | Tab: Next Pane | q: Quit",
                ActivePane::Documents => "v: View Mode | f: Fields | \u{2191}/\u{2193}: Nav | Tab: Next Pane | q: Quit",
                ActivePane::Query => "Type query... | Tab: Next Pane | q: Quit",
            },
        };

        let block = Block::default().style(Style::default().bg(Color::DarkGray).fg(Color::White));
        let paragraph = Paragraph::new(Span::styled(help_text, Style::default().add_modifier(Modifier::BOLD)))
            .block(block)
            .alignment(Alignment::Center);
        f.render_widget(paragraph, area);
    }
}

fn centered_rect(r: Rect, percent_x: u16, percent_y: u16) -> Rect {
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
