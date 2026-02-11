use super::Component;
use crate::action::Action;
use ratatui::prelude::*;
use ratatui::widgets::Paragraph;

#[derive(Default)]
pub struct FpsCounter {
    app_ticker: usize,
    render_ticker: usize,
}

impl Component for FpsCounter {
    fn update(&mut self, action: Action) -> color_eyre::Result<Option<Action>> {
        match action {
            Action::Tick => self.app_ticker += 1,
            Action::Render => self.render_ticker += 1,
            _ => {}
        }
        Ok(None)
    }

    fn draw(&mut self, f: &mut Frame, area: Rect) -> color_eyre::Result<()> {
        let rect = Rect::new(area.width - 20, 0, 20, 1);
        let text = format!(
            "App Tick: {} Render Tick: {}",
            self.app_ticker, self.render_ticker
        );
        f.render_widget(
            Paragraph::new(text).style(Style::default().fg(Color::Yellow)),
            rect,
        );
        Ok(())
    }
}
