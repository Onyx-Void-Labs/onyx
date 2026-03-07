// ─── Onyx Core — Templates (Workspace Presets) ──────────────────────

use crate::blocks::Block;
use crate::document::OnyxWorkspace;
use crate::model::PropertyType;

/// Built-in workspace templates.
#[derive(Clone, Debug, PartialEq)]
pub enum Template {
    /// University class: weekly schedule + assignments tracking.
    UniClass,
    /// Personal knowledge base.
    Personal,
    /// Project workspace with milestones.
    Project,
}

/// Apply a template to a void within a workspace.
/// Adds default properties and sections based on the template type.
pub fn apply_template(
    ws: &mut OnyxWorkspace,
    void_id: &str,
    template: Template,
) -> anyhow::Result<()> {
    match template {
        Template::UniClass => apply_uni_class(ws, void_id),
        Template::Personal => apply_personal(ws, void_id),
        Template::Project => apply_project(ws, void_id),
    }
}

fn apply_uni_class(ws: &mut OnyxWorkspace, void_id: &str) -> anyhow::Result<()> {
    // Add "Week" property (Select with week numbers)
    ws.add_property_schema(
        void_id,
        "Week",
        PropertyType::Select((1..=16).map(|w| format!("Week {}", w)).collect()),
    )?;

    // Add "Due Date" property (Date)
    ws.add_property_schema(void_id, "Due Date", PropertyType::Date)?;

    // Add "Week" section note
    let week_id = ws.create_note(void_id, "Week")?;
    ws.set_note_blocks(&week_id, &[Block::new_heading(1, "Weekly Schedule")])?;

    // Add "Assignments" section note
    let assign_id = ws.create_note(void_id, "Assignments")?;
    ws.set_note_blocks(&assign_id, &[Block::new_heading(1, "Assignments")])?;
    Ok(())
}

fn apply_personal(ws: &mut OnyxWorkspace, void_id: &str) -> anyhow::Result<()> {
    // Add "Tags" property
    ws.add_property_schema(
        void_id,
        "Tags",
        PropertyType::Select(vec![
            "Idea".into(),
            "Reference".into(),
            "Journal".into(),
            "Todo".into(),
        ]),
    )?;

    // Add "Status" property
    ws.add_property_schema(
        void_id,
        "Status",
        PropertyType::Select(vec!["Draft".into(), "In Progress".into(), "Done".into()]),
    )?;
    Ok(())
}

fn apply_project(ws: &mut OnyxWorkspace, void_id: &str) -> anyhow::Result<()> {
    // Add "Priority" property
    ws.add_property_schema(
        void_id,
        "Priority",
        PropertyType::Select(vec![
            "Low".into(),
            "Medium".into(),
            "High".into(),
            "Critical".into(),
        ]),
    )?;

    // Add "Due Date" property
    ws.add_property_schema(void_id, "Due Date", PropertyType::Date)?;

    // Add "Done" checkbox
    ws.add_property_schema(void_id, "Done", PropertyType::Checkbox)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uni_class_template_adds_properties_and_sections() -> anyhow::Result<()> {
        let mut ws = OnyxWorkspace::new();
        let void_id = ws.create_void(None, "CS101")?;
        apply_template(&mut ws, &void_id, Template::UniClass)?;

        let schema = ws.get_active_schema(&void_id);
        assert_eq!(schema.len(), 2);
        assert_eq!(schema[0].name, "Week");
        assert_eq!(schema[1].name, "Due Date");

        // Should have created 2 section notes
        let nodes = ws.get_tree_nodes();
        // 1 void + 2 notes
        assert_eq!(nodes.len(), 3);
        Ok(())
    }

    #[test]
    fn personal_template_adds_tags_and_status() -> anyhow::Result<()> {
        let mut ws = OnyxWorkspace::new();
        let void_id = ws.create_void(None, "My Notes")?;
        apply_template(&mut ws, &void_id, Template::Personal)?;

        let schema = ws.get_active_schema(&void_id);
        assert_eq!(schema.len(), 2);
        assert_eq!(schema[0].name, "Tags");
        assert_eq!(schema[1].name, "Status");
        Ok(())
    }

    #[test]
    fn project_template_adds_priority_and_dates() -> anyhow::Result<()> {
        let mut ws = OnyxWorkspace::new();
        let void_id = ws.create_void(None, "Sprint 1")?;
        apply_template(&mut ws, &void_id, Template::Project)?;

        let schema = ws.get_active_schema(&void_id);
        assert_eq!(schema.len(), 3);
        assert_eq!(schema[0].name, "Priority");
        assert_eq!(schema[1].name, "Due Date");
        assert_eq!(schema[2].name, "Done");
        Ok(())
    }
}
