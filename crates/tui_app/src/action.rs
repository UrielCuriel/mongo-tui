use serde::{Deserialize, Serialize};
use strum::Display;

#[derive(Debug, Clone, PartialEq, Display, Serialize, Deserialize)]
pub enum Action {
    Tick,
    Render,
    Resize(u16, u16),
    Suspend,
    Resume,
    Quit,
    ClearScreen,
    Error(String),
    Help,

    // MongoDB Actions
    Connect(String),
    SelectDatabase(usize),
    SelectCollection(usize),
    RefreshDatabases,
    RefreshDocuments,
    NextPage,
    PreviousPage,
    ToggleViewMode,
    OpenJsonPopup(String),
    OpenConnectionManager,
    ClosePopup,

    // Connection Actions
    SaveConnection(String, String), // Name, URI
    DeleteConnection(usize),

    // Async Results
    DatabasesLoaded(Vec<mongo_core::DatabaseInfo>),
    DocumentsLoaded(Vec<mongo_core::bson::Document>),
    SchemaLoaded(Vec<String>),
    ErrorMsg(String),
}
