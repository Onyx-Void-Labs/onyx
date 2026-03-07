// ─── Onyx Core — Data Model ────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeType {
    Void,
    Note,
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
