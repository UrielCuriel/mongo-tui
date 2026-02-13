use color_eyre::eyre::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    prelude::*,
    widgets::{Block, BorderType, Borders, Paragraph},
};

use super::super::{context::MongoContext, pane_id::PaneId, registry::Pane};
use crate::action::Action;

pub struct QueryPane {
    id: PaneId,
}

impl QueryPane {
    pub fn new(id: PaneId) -> Self {
        Self { id }
    }
}

impl Pane for QueryPane {
    fn id(&self) -> PaneId {
        self.id
    }

    fn name(&self) -> &'static str {
        "Query"
    }

    fn get_shortcuts(&self) -> Vec<(&'static str, &'static str)> {
        vec![("Enter", "Edit")]
    }

    fn handle_key_event(
        &mut self,
        key: KeyEvent,
        _ctx: &mut MongoContext,
    ) -> Result<Option<Action>> {
        if key.code == KeyCode::Enter {
            // Signal to open the Query Builder popup
            return Ok(Some(Action::OpenQueryBuilder));
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
            .title("[3] Query")
            .title_bottom(Line::from(shortcuts_str).alignment(Alignment::Center))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(if is_active {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            });

        // Display current query summary
        let filter_line = ctx.query_input.lines().join("");
        let sort_line = ctx.sort_input.lines().join("");
        let limit_line = ctx.limit_input.lines().join("");

        let text = vec![
            Line::from(vec![
                Span::styled("Filter: ", Style::default().fg(Color::Cyan)),
                Span::raw(if filter_line.is_empty() {
                    "{}"
                } else {
                    &filter_line
                }),
            ]),
            Line::from(vec![
                Span::styled("Sort: ", Style::default().fg(Color::Cyan)),
                Span::raw(if sort_line.is_empty() {
                    "{}"
                } else {
                    &sort_line
                }),
                Span::raw(" | "),
                Span::styled("Limit: ", Style::default().fg(Color::Cyan)),
                Span::raw(if limit_line.is_empty() {
                    "10"
                } else {
                    &limit_line
                }),
            ]),
        ];

        let paragraph = Paragraph::new(text).block(block);
        f.render_widget(paragraph, area);
        Ok(())
    }
}
