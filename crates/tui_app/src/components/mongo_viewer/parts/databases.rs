use color_eyre::eyre::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    prelude::*,
    widgets::{Block, BorderType, Borders},
};
use tui_tree_widget::{Tree, TreeItem, TreeState};

use super::super::{context::MongoContext, pane_id::PaneId, registry::Pane};
use crate::action::Action;

pub struct DatabasesPane {
    id: PaneId,
    state: TreeState<String>,
    tree_items: Vec<TreeItem<'static, String>>,
}

impl DatabasesPane {
    pub fn new(id: PaneId) -> Self {
        Self {
            id,
            state: TreeState::default(),
            tree_items: vec![],
        }
    }

    fn rebuild_tree_items(&mut self, ctx: &MongoContext) {
        let mut items = vec![];
        for db in ctx.databases.iter() {
            let mut children = vec![];
            for coll in db.collections.iter() {
                // Use a composite ID: "db_name:coll_name" for uniqueness and stability
                let id = format!("{}:{}", db.name, coll.name);
                children.push(TreeItem::new_leaf(id, coll.name.clone()));
            }

            // Use db.name for DB ID
            let id = db.name.clone();
            items.push(
                TreeItem::new(id, db.name.clone(), children).expect("Failed to create tree item"),
            );
        }
        self.tree_items = items;
    }
}

impl Pane for DatabasesPane {
    fn id(&self) -> PaneId {
        self.id
    }

    fn name(&self) -> &'static str {
        "Databases"
    }

    fn get_shortcuts(&self) -> Vec<(&'static str, &'static str)> {
        vec![("Enter", "Select/Expand"), ("j/k", "Nav")]
    }

    fn update(&mut self, action: Action, ctx: &mut MongoContext) -> Result<Option<Action>> {
        if let Action::DatabasesLoaded(_) = action {
            self.rebuild_tree_items(ctx);
            // Optionally expand the first one or restore state
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
                self.state.key_down();
                return Ok(Some(Action::Render));
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.state.key_up();
                return Ok(Some(Action::Render));
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                let selected = self.state.selected();
                if selected.is_empty() {
                    // No selection or root?
                    self.state.toggle_selected();
                    return Ok(Some(Action::Render));
                }

                let last_id = selected.last().unwrap();
                // If ID contains ':', it's a collection: "db_name:coll_name"
                if last_id.contains(':') {
                    let parts: Vec<&str> = last_id.split(':').collect();
                    if parts.len() == 2 {
                        let db_name = parts[0];
                        let coll_name = parts[1];

                        // Find indices
                        if let Some(db_idx) = ctx.databases.iter().position(|d| d.name == db_name) {
                            if let Some(coll_idx) = ctx.databases[db_idx]
                                .collections
                                .iter()
                                .position(|c| c.name == coll_name)
                            {
                                ctx.selected_db_index = Some(db_idx);
                                ctx.selected_coll_index = Some(coll_idx);
                                ctx.pagination.current_page = 0; // Reset pagination
                                return Ok(Some(Action::RefreshDocuments));
                            }
                        }
                    }
                } else {
                    // It's a database, just toggle expand/collapse
                    self.state.toggle_selected();
                    return Ok(Some(Action::Render));
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
        _ctx: &MongoContext,
    ) -> Result<()> {
        // Show subset
        let shortcuts_str = "Space/Enter: Expand/Select";

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

        let tree = Tree::new(&self.tree_items)
            .expect("all item identifiers are unique")
            .block(block)
            .highlight_style(Style::default().fg(Color::Black).bg(Color::Blue));

        f.render_stateful_widget(tree, area, &mut self.state);
        Ok(())
    }
}
