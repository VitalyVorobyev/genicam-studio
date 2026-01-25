use quick_xml::events::{attributes::AttrError, BytesStart, Event};
use quick_xml::Reader;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UiGraph {
    pub nodes_by_name: HashMap<String, UiNode>,
    pub categories: HashMap<String, UiCategory>,
    pub root_category: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UiNode {
    pub name: String,
    pub kind: UiNodeKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tooltip: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visibility: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub representation: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub constraints: Option<NumericConstraints>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub enum_entries: Vec<EnumEntry>,
    pub raw: RawNode,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum UiNodeKind {
    Category,
    Integer,
    Float,
    Boolean,
    String,
    Enumeration,
    Command,
    Register,
    Unknown { tag: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UiCategory {
    pub name: String,
    pub display_name: String,
    pub features: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct NumericConstraints {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inc: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnumEntry {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RawNode {
    pub tag: String,
    pub attributes: HashMap<String, String>,
    pub children_text: HashMap<String, String>,
}

#[derive(Debug)]
pub enum ParseError {
    Xml(quick_xml::Error),
    Attr(AttrError),
    MissingName { tag: String },
}

impl From<quick_xml::Error> for ParseError {
    fn from(value: quick_xml::Error) -> Self {
        Self::Xml(value)
    }
}

impl From<AttrError> for ParseError {
    fn from(value: AttrError) -> Self {
        Self::Attr(value)
    }
}

struct TempNode {
    name: String,
    tag: String,
    kind: UiNodeKind,
    display_name: Option<String>,
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

pub fn parse_genicam_xml(xml: &str) -> Result<UiGraph, ParseError> {
    let mut reader = Reader::from_str(xml);
    reader.trim_text(true);

    let mut buf = Vec::new();
    let mut depth = 0usize;
    let mut nodes_by_name: HashMap<String, UiNode> = HashMap::new();
    let mut categories: HashMap<String, UiCategory> = HashMap::new();
    let mut root_category: Option<String> = None;

    let mut current_node: Option<TempNode> = None;
    let mut current_node_depth: Option<usize> = None;
    let mut current_enum_entry: Option<TempEnumEntry> = None;
    let mut text_context: Option<TextContext> = None;

    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(event) => {
                handle_start(
                    &event,
                    depth,
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
                    &mut current_node,
                    &mut current_node_depth,
                    &mut current_enum_entry,
                    &mut text_context,
                )?;
                let tag = qname_to_string(event.local_name().as_ref());
                handle_end(
                    &tag,
                    depth,
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

    Ok(UiGraph {
        nodes_by_name,
        categories,
        root_category: root_category.unwrap_or_default(),
    })
}

fn handle_start(
    event: &BytesStart<'_>,
    depth: usize,
    current_node: &mut Option<TempNode>,
    current_node_depth: &mut Option<usize>,
    current_enum_entry: &mut Option<TempEnumEntry>,
    text_context: &mut Option<TextContext>,
) -> Result<(), ParseError> {
    let tag = qname_to_string(event.local_name().as_ref());

    if current_node.is_none() && depth == 1 {
        let attributes = attributes_to_map(event)?;
        let name = attributes
            .get("Name")
            .cloned()
            .ok_or(ParseError::MissingName { tag: tag.clone() })?;
        let kind = match tag.as_str() {
            "Category" => UiNodeKind::Category,
            "Integer" => UiNodeKind::Integer,
            "Float" => UiNodeKind::Float,
            "Boolean" => UiNodeKind::Boolean,
            "String" => UiNodeKind::String,
            "Enumeration" => UiNodeKind::Enumeration,
            "Command" => UiNodeKind::Command,
            "Register" => UiNodeKind::Register,
            _ => UiNodeKind::Unknown { tag: tag.clone() },
        };

        *current_node = Some(TempNode {
            name,
            tag: tag.clone(),
            kind,
            display_name: None,
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
            TextContext::EnumEntryField {
                tag,
                depth: depth + 1,
            }
        } else {
            TextContext::NodeField {
                tag,
                depth: depth + 1,
            }
        };
        *text_context = Some(context);
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn handle_end(
    tag: &str,
    depth: usize,
    current_node: &mut Option<TempNode>,
    current_node_depth: &mut Option<usize>,
    current_enum_entry: &mut Option<TempEnumEntry>,
    text_context: &mut Option<TextContext>,
    nodes_by_name: &mut HashMap<String, UiNode>,
    categories: &mut HashMap<String, UiCategory>,
    root_category: &mut Option<String>,
) {
    if let Some(TextContext::NodeField { tag: ctx_tag, depth: ctx_depth }) = text_context.as_ref()
    {
        if ctx_tag == tag && *ctx_depth == depth {
            *text_context = None;
        }
    }

    if let Some(TextContext::EnumEntryField { tag: ctx_tag, depth: ctx_depth }) =
        text_context.as_ref()
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

    if let (Some(node_depth), Some(node)) = (current_node_depth.as_ref(), current_node.as_ref()) {
        if *node_depth == depth && node.tag == tag {
            let node = current_node.take().expect("node present");
            *current_node_depth = None;

            let name = node.name.clone();
            let kind = node.kind.clone();
            let display_name = node
                .display_name
                .clone()
                .unwrap_or_else(|| name.clone());

            if matches!(kind, UiNodeKind::Category) {
                categories.insert(
                    name.clone(),
                    UiCategory {
                        name: name.clone(),
                        display_name,
                        features: node.features.clone(),
                    },
                );
                if root_category.is_none() {
                    *root_category = Some(name.clone());
                }
            }

            nodes_by_name.insert(name, node.into_ui_node());
        }
    }
}

fn apply_node_field(node: &mut TempNode, tag: &str, text: &str) {
    match tag {
        "DisplayName" => node.display_name = Some(text.to_string()),
        "ToolTip" => node.tooltip = Some(text.to_string()),
        "Description" => node.description = Some(text.to_string()),
        "Visibility" => node.visibility = Some(text.to_string()),
        "AccessMode" => node.access_mode = Some(text.to_string()),
        "Unit" => node.unit = Some(text.to_string()),
        "Representation" => node.representation = Some(text.to_string()),
        "Min" => set_numeric_constraint(&mut node.constraints, |c| c.min = parse_number(text)),
        "Max" => set_numeric_constraint(&mut node.constraints, |c| c.max = parse_number(text)),
        "Inc" => set_numeric_constraint(&mut node.constraints, |c| c.inc = parse_number(text)),
        "Value" => {
            set_numeric_constraint(&mut node.constraints, |c| c.value = parse_number(text))
        }
        "pFeature" => node.features.push(text.to_string()),
        _ => {}
    }

    node.raw_children.insert(tag.to_string(), text.to_string());
}

fn apply_enum_entry_field(entry: &mut TempEnumEntry, tag: &str, text: &str) {
    match tag {
        "Name" => entry.name = Some(text.to_string()),
        "Value" => entry.value = Some(text.to_string()),
        "DisplayName" => entry.display_name = Some(text.to_string()),
        _ => {}
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    #[test]
    fn parse_minimal_fixture() {
        let xml = include_str!("../fixtures/minimal.xml");
        let graph = parse_genicam_xml(xml).expect("parse minimal fixture");

        assert_eq!(graph.root_category, "Root");

        let root_category = graph
            .categories
            .get("Root")
            .expect("root category present");
        assert_eq!(
            root_category.features,
            vec![
                "ExposureTime",
                "PixelFormat",
                "AcquisitionEnable",
                "TriggerSoftware",
                "MagicCalc"
            ]
        );

        let exposure = graph
            .nodes_by_name
            .get("ExposureTime")
            .expect("exposure node present");
        assert!(matches!(exposure.kind, UiNodeKind::Integer));
        let constraints = exposure.constraints.as_ref().expect("constraints present");
        assert_eq!(constraints.min, Some(10.0));
        assert_eq!(constraints.max, Some(1000.0));
        assert_eq!(constraints.inc, Some(5.0));

        let pixel_format = graph
            .nodes_by_name
            .get("PixelFormat")
            .expect("enumeration present");
        assert!(matches!(pixel_format.kind, UiNodeKind::Enumeration));
        assert_eq!(pixel_format.enum_entries.len(), 3);
        assert_eq!(pixel_format.enum_entries[0].name, "Mono8");
        assert_eq!(pixel_format.enum_entries[1].name, "Mono12");
        assert_eq!(pixel_format.enum_entries[2].name, "RGB8");

        let unknown = graph
            .nodes_by_name
            .get("MagicCalc")
            .expect("unknown node present");
        assert!(matches!(
            &unknown.kind,
            UiNodeKind::Unknown { tag } if tag == "SwissKnife"
        ));
    }

    #[test]
    fn snapshot_minimal_fixture() {
        let xml = include_str!("../fixtures/minimal.xml");
        let graph = parse_genicam_xml(xml).expect("parse minimal fixture");
        let actual = normalize_json(serde_json::to_value(&graph).expect("json value"));
        let actual_string = serde_json::to_string_pretty(&actual).expect("json string");
        let expected = include_str!("../fixtures/expected_minimal.json").trim();

        assert_eq!(actual_string, expected);
    }

    fn normalize_json(value: Value) -> Value {
        match value {
            Value::Object(map) => {
                let mut entries: Vec<_> = map.into_iter().collect();
                entries.sort_by(|a, b| a.0.cmp(&b.0));
                let mut normalized = serde_json::Map::new();
                for (key, value) in entries {
                    normalized.insert(key, normalize_json(value));
                }
                Value::Object(normalized)
            }
            Value::Array(values) => {
                Value::Array(values.into_iter().map(normalize_json).collect())
            }
            other => other,
        }
    }
}
