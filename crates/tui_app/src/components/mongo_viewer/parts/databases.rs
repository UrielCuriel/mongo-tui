use color_eyre::eyre::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    prelude::*,
    widgets::{Block, BorderType, Borders, List, ListItem, ListState},
};
use std::collections::HashSet;

use super::super::{context::MongoContext, pane_id::PaneId, registry::Pane};
use crate::action::Action;

#[derive(Debug, Clone)]
enum TreeItem {
    Database(usize),          // index in ctx.databases
    Collection(usize, usize), // (db_index, coll_index)
}

pub struct DatabasesPane {
    id: PaneId,
    expanded_dbs: HashSet<String>,
    tree_items: Vec<TreeItem>,
    selected_tree_index: Option<usize>,
    list_state: ListState,
}

impl DatabasesPane {
    pub fn new(id: PaneId) -> Self {
        Self {
            id,
            expanded_dbs: HashSet::new(),
            tree_items: vec![],
            selected_tree_index: None,
            list_state: ListState::default(),
        }
    }

    fn rebuild_tree_items(&mut self, ctx: &MongoContext) {
        self.tree_items.clear();
        for (db_idx, db) in ctx.databases.iter().enumerate() {
            self.tree_items.push(TreeItem::Database(db_idx));
            if self.expanded_dbs.contains(&db.name) {
                for (coll_idx, _) in db.collections.iter().enumerate() {
                    self.tree_items.push(TreeItem::Collection(db_idx, coll_idx));
                }
            }
        }
    }
}

impl Pane for DatabasesPane {
    fn id(&self) -> PaneId {
        self.id
    }

    fn get_shortcuts(&self) -> Vec<(&'static str, &'static str)> {
        vec![("Enter", "Select/Expand"), ("j/k", "Nav")]
    }

    fn update(&mut self, action: Action, ctx: &mut MongoContext) -> Result<Option<Action>> {
        match action {
            Action::DatabasesLoaded(_) => {
                self.rebuild_tree_items(ctx);
                if !self.tree_items.is_empty() {
                    self.selected_tree_index = Some(0);
                    self.list_state.select(Some(0));
                } else {
                    self.selected_tree_index = None;
                    self.list_state.select(None);
                }
            }
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
            KeyCode::Char('j') | KeyCode::Down => {
                if let Some(idx) = self.selected_tree_index {
                    if idx + 1 < self.tree_items.len() {
                        self.selected_tree_index = Some(idx + 1);
                        self.list_state.select(self.selected_tree_index);
                        return Ok(Some(Action::Render));
                    }
                } else if !self.tree_items.is_empty() {
                    self.selected_tree_index = Some(0);
                    self.list_state.select(Some(0));
                    return Ok(Some(Action::Render));
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if let Some(idx) = self.selected_tree_index {
                    if idx > 0 {
                        self.selected_tree_index = Some(idx - 1);
                        self.list_state.select(self.selected_tree_index);
                        return Ok(Some(Action::Render));
                    }
                }
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                if let Some(idx) = self.selected_tree_index {
                    if let Some(item) = self.tree_items.get(idx) {
                        match item {
                            TreeItem::Database(db_idx) => {
                                if let Some(db) = ctx.databases.get(*db_idx) {
                                    let name = db.name.clone();
                                    if self.expanded_dbs.contains(&name) {
                                        self.expanded_dbs.remove(&name);
                                    } else {
                                        self.expanded_dbs.insert(name);
                                    }
                                    self.rebuild_tree_items(ctx);
                                    return Ok(Some(Action::Render));
                                }
                            }
                            TreeItem::Collection(db_idx, coll_idx) => {
                                ctx.selected_db_index = Some(*db_idx);
                                ctx.selected_coll_index = Some(*coll_idx);
                                return Ok(Some(Action::RefreshDocuments));
                            }
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
            .title("[2] Databases")
            .title_bottom(Line::from(shortcuts_str).alignment(Alignment::Center))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(if is_active {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            });

        let items: Vec<ListItem> = self
            .tree_items
            .iter()
            .map(|item| match item {
                TreeItem::Database(idx) => {
                    if let Some(db) = ctx.databases.get(*idx) {
                        let prefix = if self.expanded_dbs.contains(&db.name) {
                            "v "
                        } else {
                            "> "
                        };
                        ListItem::new(format!("{}{}", prefix, db.name))
                            .style(Style::default().add_modifier(Modifier::BOLD))
                    } else {
                        ListItem::new("?")
                    }
                }
                TreeItem::Collection(db_idx, coll_idx) => {
                    if let Some(db) = ctx.databases.get(*db_idx) {
                        if let Some(coll) = db.collections.get(*coll_idx) {
                            ListItem::new(format!("  • {}", coll.name))
                        } else {
                            ListItem::new("  ?")
                        }
                    } else {
                        ListItem::new("  ?")
                    }
                }
            })
            .collect();

        // Sync state
        let mut state = self.list_state.clone();
        state.select(self.selected_tree_index);

        let list = List::new(items)
            .block(block)
            .highlight_style(Style::default().bg(Color::Blue));

        f.render_stateful_widget(list, area, &mut state);
        Ok(())
    }
}
