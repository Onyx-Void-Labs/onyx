// ─── Onyx Core — Data Model ────────────────────────────────────────

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeType {
    Void,
    Note,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum PropertyType {
    Text,
    Select(Vec<String>),
    Date,
    Checkbox,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PropertyDefinition {
    pub name: String,
    pub kind: PropertyType,
}

#[derive(Debug, Clone)]
pub struct OnyxNode {
    pub id: String,
    pub node_type: NodeType,
    pub title: String,
}

#[derive(Debug, Clone)]
pub struct SectionConfig {
    pub name: String,
    pub filter_prop: Option<String>,
    pub filter_val: Option<String>,
}
