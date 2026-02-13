use color_eyre::eyre::Result;
use crossterm::event::KeyEvent;
use ratatui::layout::Rect;
use ratatui::Frame;
use std::collections::HashMap;

use super::context::MongoContext;
use super::pane_id::PaneId;
use crate::action::Action;

pub trait Pane {
    fn id(&self) -> PaneId;
    fn name(&self) -> &'static str;
    fn handle_key_event(&mut self, key: KeyEvent, ctx: &mut MongoContext)
        -> Result<Option<Action>>;
    fn draw(
        &mut self,
        f: &mut Frame,
        area: Rect,
        is_active: bool,
        ctx: &MongoContext,
    ) -> Result<()>;
    fn get_shortcuts(&self) -> Vec<(&'static str, &'static str)>;
    fn update(&mut self, _action: Action, _ctx: &mut MongoContext) -> Result<Option<Action>> {
        Ok(None)
    }
}

pub struct PaneRegistry {
    panes: HashMap<PaneId, Box<dyn Pane>>,
    ordered_ids: Vec<PaneId>, // Defines navigation cycle order
    active_pane: Option<PaneId>,
}

impl Default for PaneRegistry {
    fn default() -> Self {
        Self {
            panes: HashMap::new(),
            ordered_ids: Vec::new(),
            active_pane: None,
        }
    }
}

impl PaneRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register<P: Pane + 'static>(&mut self, pane: P) {
        let id = pane.id();
        self.panes.insert(id, Box::new(pane));
        self.ordered_ids.push(id);

        // Auto-select first registered pane if none selected
        if self.active_pane.is_none() {
            self.active_pane = Some(id);
        }
    }

    pub fn get_active_pane(&mut self) -> Option<&mut Box<dyn Pane>> {
        if let Some(id) = self.active_pane {
            self.panes.get_mut(&id)
        } else {
            None
        }
    }

    pub fn get_pane(&mut self, id: PaneId) -> Option<&mut Box<dyn Pane>> {
        self.panes.get_mut(&id)
    }

    pub fn active_pane_id(&self) -> Option<PaneId> {
        self.active_pane
    }

    pub fn cycle_next(&mut self) {
        if self.ordered_ids.is_empty() {
            return;
        }

        if let Some(curr) = self.active_pane {
            if let Some(pos) = self.ordered_ids.iter().position(|x| *x == curr) {
                let next_pos = (pos + 1) % self.ordered_ids.len();
                self.active_pane = Some(self.ordered_ids[next_pos]);
            } else {
                self.active_pane = Some(self.ordered_ids[0]);
            }
        } else {
            self.active_pane = Some(self.ordered_ids[0]);
        }
    }

    pub fn set_active(&mut self, id: PaneId) {
        if self.panes.contains_key(&id) {
            self.active_pane = Some(id);
        }
    }

    pub fn handle_key_event(
        &mut self,
        key: KeyEvent,
        ctx: &mut MongoContext,
    ) -> Result<Option<Action>> {
        if let Some(pane) = self.get_active_pane() {
            pane.handle_key_event(key, ctx)
        } else {
            Ok(None)
        }
    }

    pub fn update_all(&mut self, action: Action, ctx: &mut MongoContext) -> Result<()> {
        // Broadcast updates to all panes
        for pane in self.panes.values_mut() {
            let _ = pane.update(action.clone(), ctx)?;
        }
        Ok(())
    }

    pub fn get_all_shortcuts(&self) -> Vec<(&'static str, Vec<(&'static str, &'static str)>)> {
        let mut result = Vec::new();
        for id in &self.ordered_ids {
            if let Some(pane) = self.panes.get(id) {
                result.push((pane.name(), pane.get_shortcuts()));
            }
        }
        result
    }
}
