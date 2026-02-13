use color_eyre::eyre::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    prelude::*,
    widgets::{Block, BorderType, Borders, List, ListItem, ListState},
};

use super::super::{context::MongoContext, pane_id::PaneId, registry::Pane};
use crate::action::Action;

pub struct ConnectionsPane {
    id: PaneId,
    list_state: ListState,
}

impl ConnectionsPane {
    pub fn new(id: PaneId) -> Self {
        Self {
            id,
            list_state: ListState::default(),
        }
    }
}

impl Pane for ConnectionsPane {
    fn id(&self) -> PaneId {
        self.id
    }

    fn get_shortcuts(&self) -> Vec<(&'static str, &'static str)> {
        vec![
            ("c", "Add"),
            ("Enter", "Connect"),
            ("j/k", "Nav"),
            ("Del", "Remove"),
        ]
    }

    fn handle_key_event(
        &mut self,
        key: KeyEvent,
        ctx: &mut MongoContext,
    ) -> Result<Option<Action>> {
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                if let Some(idx) = ctx.selected_connection {
                    if idx + 1 < ctx.connections.len() {
                        ctx.selected_connection = Some(idx + 1);
                        self.list_state.select(ctx.selected_connection);
                        return Ok(Some(Action::Render));
                    }
                } else if !ctx.connections.is_empty() {
                    ctx.selected_connection = Some(0);
                    self.list_state.select(ctx.selected_connection);
                    return Ok(Some(Action::Render));
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if let Some(idx) = ctx.selected_connection {
                    if idx > 0 {
                        ctx.selected_connection = Some(idx - 1);
                        self.list_state.select(ctx.selected_connection);
                        return Ok(Some(Action::Render));
                    }
                }
            }
            KeyCode::Enter => {
                if let Some(idx) = ctx.selected_connection {
                    if let Some(conn) = ctx.connections.get(idx) {
                        return Ok(Some(Action::Connect(conn.uri.clone())));
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
            .title("[1] Connections")
            .title_bottom(Line::from(shortcuts_str).alignment(Alignment::Center))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(if is_active {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            });

        let items: Vec<ListItem> = ctx
            .connections
            .iter()
            .map(|conn| ListItem::new(conn.name.clone()))
            .collect();

        // Sync state just in case
        let mut state = self.list_state.clone();
        state.select(ctx.selected_connection);

        let list = List::new(items)
            .block(block)
            .highlight_style(Style::default().bg(Color::Blue));

        f.render_stateful_widget(list, area, &mut state);
        Ok(())
    }
}
