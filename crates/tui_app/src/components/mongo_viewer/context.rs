use crate::action::Action;
use crate::config::Connection;
use arboard::Clipboard;
use mongo_core::bson::Document;
use mongo_core::{DatabaseInfo, MongoCore};
use std::collections::HashMap;
use tokio::sync::mpsc::UnboundedSender;

use tui_textarea::TextArea;

pub struct MongoContext {
    pub action_tx: Option<UnboundedSender<Action>>,
    pub mongo_core: MongoCore,

    // Data
    pub connections: Vec<Connection>,
    pub databases: Vec<DatabaseInfo>,
    pub documents: Vec<Document>,

    // Selection Context
    pub selected_connection: Option<usize>,
    pub selected_db_index: Option<usize>,
    pub selected_coll_index: Option<usize>,

    // Query Inputs
    pub query_input: TextArea<'static>,
    pub projection_input: TextArea<'static>,
    pub sort_input: TextArea<'static>,
    pub limit_input: TextArea<'static>,
    pub input_validation_errors: HashMap<crate::components::mongo_viewer::defs::QueryField, String>,

    // System
    pub clipboard: Option<Clipboard>,
}

impl Default for MongoContext {
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
            selected_connection: None,
            selected_db_index: None,
            selected_coll_index: None,
            query_input: query,
            projection_input: proj,
            sort_input: sort,
            limit_input: limit,
            input_validation_errors: HashMap::new(),
            clipboard: Clipboard::new().ok(),
        }
    }
}

impl MongoContext {
    pub fn new() -> Self {
        Self::default()
    }
}
