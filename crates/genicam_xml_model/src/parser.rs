use crate::error::ParseError;
use crate::model::{
    EnumEntry, NumericConstraints, RawNode, UiCategory, UiGraph, UiNode, UiNodeKind,
};
use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;
use std::collections::HashMap;

pub fn parse_genicam_xml(xml: &str) -> Result<UiGraph, ParseError> {
    let mut reader = Reader::from_str(xml);
    reader.trim_text(true);

    let mut buf = Vec::new();
    // We call handle_start before incrementing depth, and handle_end after decrementing depth,
    // so a Start and its matching End observe the same depth value.
    let mut depth = 0usize;
    let mut nodes_by_name: HashMap<String, UiNode> = HashMap::new();
    let mut categories: HashMap<String, UiCategory> = HashMap::new();
    let mut root_category: Option<String> = None;

    let mut current_node: Option<TempNode> = None;
    let mut current_node_depth: Option<usize> = None;
    let mut current_enum_entry: Option<TempEnumEntry> = None;
    let mut text_context: Option<TextContext> = None;
    // Some GenICam files wrap feature definitions in <Group Comment="..."> without a Name.
    // We tolerate that by treating Group as a container and mapping its Comment to a category.
    let mut current_group_depth: Option<usize> = None;
    let mut current_group_comment: Option<String> = None;
    let mut pending_group_comments: Vec<String> = Vec::new();
    // When a top-level element is missing Name, synthesize a stable debug-friendly key.
    let mut synthetic_counters: HashMap<String, usize> = HashMap::new();

    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(event) => {
                handle_start(
                    &event,
                    depth,
                    &mut current_group_depth,
                    &mut current_group_comment,
                    &mut synthetic_counters,
                    &mut current_node,
                    &mut current_node_depth,
                    &mut current_enum_entry,
                    &mut text_context,
                )?;
                depth += 1;
            }
            Event::Empty(event) => {
                handle_start(
                    &event,
                    depth,
                    &mut current_group_depth,
                    &mut current_group_comment,
                    &mut synthetic_counters,
                    &mut current_node,
                    &mut current_node_depth,
                    &mut current_enum_entry,
                    &mut text_context,
                )?;

                let tag = qname_to_string(event.local_name().as_ref());
                handle_end(
                    &tag,
                    depth,
                    &mut current_group_depth,
                    &mut current_group_comment,
                    &mut pending_group_comments,
                    &mut current_node,
                    &mut current_node_depth,
                    &mut current_enum_entry,
                    &mut text_context,
                    &mut nodes_by_name,
                    &mut categories,
                    &mut root_category,
                );
            }
            Event::Text(event) => {
                if let Some(context) = &text_context {
                    let text = event.unescape()?.into_owned();
                    match context {
                        TextContext::NodeField { tag, .. } => {
                            if let Some(node) = current_node.as_mut() {
                                apply_node_field(node, tag, &text);
                            }
                        }
                        TextContext::EnumEntryField { tag, .. } => {
                            if let Some(entry) = current_enum_entry.as_mut() {
                                apply_enum_entry_field(entry, tag, &text);
                            }
                        }
                    }
                }
            }
            Event::End(event) => {
                if depth == 0 {
                    break;
                }
                depth -= 1;

                let tag = qname_to_string(event.local_name().as_ref());
                handle_end(
                    &tag,
                    depth,
                    &mut current_group_depth,
                    &mut current_group_comment,
                    &mut pending_group_comments,
                    &mut current_node,
                    &mut current_node_depth,
                    &mut current_enum_entry,
                    &mut text_context,
                    &mut nodes_by_name,
                    &mut categories,
                    &mut root_category,
                );
            }
            Event::Eof => break,
            _ => {}
        }

        buf.clear();
    }

    // Resolve any deferred group comments after the full pass.
    apply_pending_group_comments(
        &mut pending_group_comments,
        &mut categories,
        &mut nodes_by_name,
    );

    Ok(UiGraph {
        nodes_by_name,
        categories,
        root_category: root_category.unwrap_or_default(),
    })
}

struct TempNode {
    name: String,
    tag: String,
    kind: UiNodeKind,
    display_name: Option<String>,
    comment: Option<String>,
    tooltip: Option<String>,
    description: Option<String>,
    visibility: Option<String>,
    access_mode: Option<String>,
    unit: Option<String>,
    representation: Option<String>,
    constraints: Option<NumericConstraints>,
    enum_entries: Vec<EnumEntry>,
    features: Vec<String>,
    raw_attributes: HashMap<String, String>,
    raw_children: HashMap<String, String>,
}

impl TempNode {
    fn into_ui_node(self) -> UiNode {
        UiNode {
            name: self.name,
            kind: self.kind,
            display_name: self.display_name,
            comment: self.comment,
            tooltip: self.tooltip,
            description: self.description,
            visibility: self.visibility,
            access_mode: self.access_mode,
            unit: self.unit,
            representation: self.representation,
            constraints: self.constraints,
            enum_entries: self.enum_entries,
            raw: RawNode {
                tag: self.tag,
                attributes: self.raw_attributes,
                children_text: self.raw_children,
            },
            dependencies: vec![],
            dependents: vec![],
            expression: None,
            int_min: None,
            int_max: None,
            int_inc: None,
        }
    }
}

struct TempEnumEntry {
    name: Option<String>,
    value: Option<String>,
    display_name: Option<String>,
}

impl TempEnumEntry {
    fn finalize(self) -> Option<EnumEntry> {
        self.name.map(|name| EnumEntry {
            name,
            value: self.value,
            display_name: self.display_name,
        })
    }
}

enum TextContext {
    NodeField { tag: String, depth: usize },
    EnumEntryField { tag: String, depth: usize },
}

#[allow(clippy::too_many_arguments)]
fn handle_start(
    event: &BytesStart<'_>,
    depth: usize,
    current_group_depth: &mut Option<usize>,
    current_group_comment: &mut Option<String>,
    synthetic_counters: &mut HashMap<String, usize>,
    current_node: &mut Option<TempNode>,
    current_node_depth: &mut Option<usize>,
    current_enum_entry: &mut Option<TempEnumEntry>,
    text_context: &mut Option<TextContext>,
) -> Result<(), ParseError> {
    let tag = qname_to_string(event.local_name().as_ref());

    // Treat <Group ...> as a container. Many real-world GenICam files place all feature
    // nodes inside a Group without a Name attribute, which would otherwise be a hard error.
    if current_node.is_none() && depth == 1 && tag == "Group" {
        let attributes = attributes_to_map(event)?;
        *current_group_depth = Some(depth);
        *current_group_comment = attributes.get("Comment").cloned();
        return Ok(());
    }

    let is_group_child = current_group_depth.is_some_and(|group_depth| depth == group_depth + 1);
    let is_top_level = depth == 1 || is_group_child;

    if current_node.is_none() && is_top_level {
        let attributes = attributes_to_map(event)?;
        let name = attributes
            .get("Name")
            .cloned()
            .unwrap_or_else(|| synthetic_name(&tag, synthetic_counters));

        let kind = match tag.as_str() {
            "Category" => UiNodeKind::Category,
            "Integer" => UiNodeKind::Integer,
            "Float" => UiNodeKind::Float,
            "Boolean" => UiNodeKind::Boolean,
            "String" => UiNodeKind::String,
            "Enumeration" => UiNodeKind::Enumeration,
            "Command" => UiNodeKind::Command,
            "Register" => UiNodeKind::Register,
            _ if tag.ends_with("Reg") => UiNodeKind::Register,
            _ => UiNodeKind::Unknown { tag: tag.clone() },
        };

        *current_node = Some(TempNode {
            name,
            tag: tag.clone(),
            kind,
            display_name: None,
            comment: attributes.get("Comment").cloned(),
            tooltip: None,
            description: None,
            visibility: None,
            access_mode: None,
            unit: None,
            representation: None,
            constraints: None,
            enum_entries: Vec::new(),
            features: Vec::new(),
            raw_attributes: attributes,
            raw_children: HashMap::new(),
        });
        *current_node_depth = Some(depth);
        return Ok(());
    }

    if current_node.is_some() {
        if tag == "EnumEntry" {
            let attributes = attributes_to_map(event)?;
            let name = attributes.get("Name").cloned();
            let value = attributes.get("Value").cloned();
            *current_enum_entry = Some(TempEnumEntry {
                name,
                value,
                display_name: None,
            });
            return Ok(());
        }

        let context = if current_enum_entry.is_some() {
            TextContext::EnumEntryField { tag, depth }
        } else {
            TextContext::NodeField { tag, depth }
        };
        *text_context = Some(context);
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn handle_end(
    tag: &str,
    depth: usize,
    current_group_depth: &mut Option<usize>,
    current_group_comment: &mut Option<String>,
    pending_group_comments: &mut Vec<String>,
    current_node: &mut Option<TempNode>,
    current_node_depth: &mut Option<usize>,
    current_enum_entry: &mut Option<TempEnumEntry>,
    text_context: &mut Option<TextContext>,
    nodes_by_name: &mut HashMap<String, UiNode>,
    categories: &mut HashMap<String, UiCategory>,
    root_category: &mut Option<String>,
) {
    if tag == "Group" && current_group_depth.is_some_and(|group_depth| group_depth == depth) {
        if let Some(comment) = current_group_comment.take() {
            pending_group_comments.push(comment);
            apply_pending_group_comments(pending_group_comments, categories, nodes_by_name);
        }
        *current_group_depth = None;
        return;
    }

    if let Some(TextContext::NodeField {
        tag: ctx_tag,
        depth: ctx_depth,
    }) = text_context.as_ref()
    {
        if ctx_tag == tag && *ctx_depth == depth {
            *text_context = None;
        }
    }

    if let Some(TextContext::EnumEntryField {
        tag: ctx_tag,
        depth: ctx_depth,
    }) = text_context.as_ref()
    {
        if ctx_tag == tag && *ctx_depth == depth {
            *text_context = None;
        }
    }

    if tag == "EnumEntry" {
        if let Some(entry) = current_enum_entry.take().and_then(TempEnumEntry::finalize) {
            if let Some(node) = current_node.as_mut() {
                node.enum_entries.push(entry);
            }
        }
        return;
    }

    if let Some(node_depth) = current_node_depth.as_ref() {
        let should_finalize = current_node
            .as_ref()
            .is_some_and(|node| *node_depth == depth && node.tag == tag);

        if should_finalize {
            let Some(node) = current_node.take() else {
                *current_node_depth = None;
                return;
            };
            *current_node_depth = None;

            let name = node.name.clone();
            let kind = node.kind.clone();
            let display_name = node.display_name.clone().unwrap_or_else(|| name.clone());

            if matches!(kind, UiNodeKind::Category) {
                categories.insert(
                    name.clone(),
                    UiCategory {
                        name: name.clone(),
                        display_name,
                        features: node.features.clone(),
                        tooltip: node.tooltip.clone(),
                        comment: node.comment.clone(),
                    },
                );

                // Prefer an explicit "Root" category if it exists.
                if root_category.is_none() || name == "Root" {
                    *root_category = Some(name.clone());
                }
            }

            nodes_by_name.insert(name, node.into_ui_node());
            apply_pending_group_comments(pending_group_comments, categories, nodes_by_name);
        }
    }
}

fn apply_node_field(node: &mut TempNode, tag: &str, text: &str) {
    match tag {
        "DisplayName" => node.display_name = Some(text.to_string()),
        "Comment" => node.comment = Some(text.to_string()),
        "ToolTip" => node.tooltip = Some(text.to_string()),
        "Description" => node.description = Some(text.to_string()),
        "Visibility" => node.visibility = Some(text.to_string()),
        "AccessMode" => node.access_mode = Some(text.to_string()),
        "Unit" => node.unit = Some(text.to_string()),
        "Representation" => node.representation = Some(text.to_string()),
        "Min" => set_numeric_constraint(&mut node.constraints, |c| c.min = parse_number(text)),
        "Max" => set_numeric_constraint(&mut node.constraints, |c| c.max = parse_number(text)),
        "Inc" => set_numeric_constraint(&mut node.constraints, |c| c.inc = parse_number(text)),
        "Value" => set_numeric_constraint(&mut node.constraints, |c| c.value = parse_number(text)),
        "pFeature" => node.features.push(text.to_string()),
        _ => {}
    }

    insert_child_text(&mut node.raw_children, tag, text);
}

fn apply_enum_entry_field(entry: &mut TempEnumEntry, tag: &str, text: &str) {
    match tag {
        "Name" => entry.name = Some(text.to_string()),
        "Value" => entry.value = Some(text.to_string()),
        "DisplayName" => entry.display_name = Some(text.to_string()),
        _ => {}
    }
}

fn insert_child_text(map: &mut HashMap<String, String>, tag: &str, text: &str) {
    // Preserve repeated simple child fields (e.g. multiple <pFeature> values) without building a DOM.
    let key = tag.to_string();
    match map.get_mut(&key) {
        Some(existing) => {
            if !existing.is_empty() {
                existing.push('\n');
            }
            existing.push_str(text);
        }
        None => {
            map.insert(key, text.to_string());
        }
    }
}

fn set_numeric_constraint<F>(constraints: &mut Option<NumericConstraints>, updater: F)
where
    F: FnOnce(&mut NumericConstraints),
{
    let constraint = constraints.get_or_insert_with(NumericConstraints::default);
    updater(constraint);
}

fn parse_number(value: &str) -> Option<f64> {
    value.trim().parse::<f64>().ok()
}

fn attributes_to_map(event: &BytesStart<'_>) -> Result<HashMap<String, String>, ParseError> {
    let mut attributes = HashMap::new();
    for attribute in event.attributes() {
        let attribute = attribute?;
        let key = qname_to_string(attribute.key.as_ref());
        let value = attribute.unescape_value()?.into_owned();
        attributes.insert(key, value);
    }
    Ok(attributes)
}

fn qname_to_string(name: &[u8]) -> String {
    String::from_utf8_lossy(name).to_string()
}

fn synthetic_name(tag: &str, counters: &mut HashMap<String, usize>) -> String {
    let counter = counters.entry(tag.to_string()).or_insert(0);
    *counter += 1;
    format!("__{tag}_{counter}")
}

fn apply_pending_group_comments(
    pending_group_comments: &mut Vec<String>,
    categories: &mut HashMap<String, UiCategory>,
    nodes_by_name: &mut HashMap<String, UiNode>,
) {
    pending_group_comments
        .retain(|comment| !apply_group_comment(comment, categories, nodes_by_name));
}

fn apply_group_comment(
    comment: &str,
    categories: &mut HashMap<String, UiCategory>,
    nodes_by_name: &mut HashMap<String, UiNode>,
) -> bool {
    let trimmed = comment.trim();
    if trimmed.is_empty() {
        return true;
    }

    let matched_category = categories
        .iter()
        .find(|(_, category)| category.name == trimmed || category.display_name == trimmed)
        .map(|(name, _)| name.clone());

    let Some(category_name) = matched_category else {
        return false;
    };

    if let Some(category) = categories.get_mut(&category_name) {
        if category.comment.is_none() {
            category.comment = Some(trimmed.to_string());
        }
    }

    if let Some(node) = nodes_by_name.get_mut(&category_name) {
        if node.comment.is_none() {
            node.comment = Some(trimmed.to_string());
        }
    }

    true
}
