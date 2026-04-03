use std::collections::HashMap;

use genapi_core::NodeMap;
use genapi_xml::{AccessMode, EnumValueSrc, NodeDecl, XmlModel};

use crate::error::ParseError;
use crate::model::*;

/// Parse GenICam XML using the full genapi-xml + genapi-core pipeline.
///
/// Produces a UiGraph with resolved dependencies, precise integer constraints,
/// and expression strings for SwissKnife/Converter nodes.
pub fn parse_full(xml: &str) -> Result<UiGraph, ParseError> {
    let xml_model: XmlModel =
        genapi_xml::parse(xml).map_err(|e| ParseError::Full(e.to_string()))?;

    let nodemap =
        NodeMap::try_from_xml(xml_model.clone()).map_err(|e| ParseError::Full(e.to_string()))?;

    let mut nodes_by_name = HashMap::new();
    let mut categories = HashMap::new();
    let mut root_category = String::from("Root");

    // Build categories from NodeMap
    for (cat_name, children) in nodemap.categories() {
        categories.insert(
            cat_name.to_string(),
            UiCategory {
                name: cat_name.to_string(),
                display_name: cat_name.to_string(),
                features: children.to_vec(),
                tooltip: None,
                comment: None,
            },
        );
        if cat_name == "Root" {
            root_category = "Root".to_string();
        }
    }

    // If no "Root" category found, use the first one
    if !categories.contains_key("Root") {
        if let Some(first) = nodemap.categories().first() {
            root_category = first.0.to_string();
        }
    }

    // Map each node from NodeDecl to UiNode
    for decl in &xml_model.nodes {
        let (name, ui_node) = node_decl_to_ui_node(decl, &nodemap);
        nodes_by_name.insert(name, ui_node);
    }

    Ok(UiGraph {
        nodes_by_name,
        categories,
        root_category,
    })
}

fn access_mode_str(am: AccessMode) -> String {
    match am {
        AccessMode::RO => "RO".to_string(),
        AccessMode::WO => "WO".to_string(),
        AccessMode::RW => "RW".to_string(),
    }
}

fn node_decl_to_ui_node(decl: &NodeDecl, nodemap: &NodeMap) -> (String, UiNode) {
    let name = decl_name(decl);
    let dependents = nodemap.dependents(&name).to_vec();

    match decl {
        NodeDecl::Integer {
            name,
            access,
            min,
            max,
            inc,
            unit,
            pvalue,
            p_min,
            p_max,
            ..
        } => {
            let mut deps = Vec::new();
            if let Some(pv) = pvalue {
                deps.push(pv.clone());
            }
            if let Some(pm) = p_min {
                deps.push(pm.clone());
            }
            if let Some(pm) = p_max {
                deps.push(pm.clone());
            }

            (
                name.clone(),
                UiNode {
                    name: name.clone(),
                    kind: UiNodeKind::Integer,
                    display_name: None,
                    comment: None,
                    tooltip: None,
                    description: None,
                    visibility: None,
                    access_mode: Some(access_mode_str(*access)),
                    unit: unit.clone(),
                    representation: None,
                    constraints: Some(NumericConstraints {
                        min: Some(*min as f64),
                        max: Some(*max as f64),
                        inc: inc.map(|i| i as f64),
                        value: None,
                    }),
                    enum_entries: vec![],
                    raw: empty_raw("Integer"),
                    dependencies: deps,
                    dependents,
                    expression: None,
                    int_min: Some(*min),
                    int_max: Some(*max),
                    int_inc: *inc,
                },
            )
        }
        NodeDecl::Float {
            name,
            access,
            min,
            max,
            unit,
            pvalue,
            ..
        } => {
            let mut deps = Vec::new();
            if let Some(pv) = pvalue {
                deps.push(pv.clone());
            }

            (
                name.clone(),
                UiNode {
                    name: name.clone(),
                    kind: UiNodeKind::Float,
                    display_name: None,
                    comment: None,
                    tooltip: None,
                    description: None,
                    visibility: None,
                    access_mode: Some(access_mode_str(*access)),
                    unit: unit.clone(),
                    representation: None,
                    constraints: Some(NumericConstraints {
                        min: Some(*min),
                        max: Some(*max),
                        inc: None,
                        value: None,
                    }),
                    enum_entries: vec![],
                    raw: empty_raw("Float"),
                    dependencies: deps,
                    dependents,
                    expression: None,
                    int_min: None,
                    int_max: None,
                    int_inc: None,
                },
            )
        }
        NodeDecl::Enum {
            name,
            access,
            entries,
            pvalue,
            ..
        } => {
            let mut deps = Vec::new();
            if let Some(pv) = pvalue {
                deps.push(pv.clone());
            }

            (
                name.clone(),
                UiNode {
                    name: name.clone(),
                    kind: UiNodeKind::Enumeration,
                    display_name: None,
                    comment: None,
                    tooltip: None,
                    description: None,
                    visibility: None,
                    access_mode: Some(access_mode_str(*access)),
                    unit: None,
                    representation: None,
                    constraints: None,
                    enum_entries: entries
                        .iter()
                        .map(|e| EnumEntry {
                            name: e.name.clone(),
                            value: match &e.value {
                                EnumValueSrc::Literal(v) => Some(v.to_string()),
                                EnumValueSrc::FromNode(_) => None,
                            },
                            display_name: e.display_name.clone(),
                        })
                        .collect(),
                    raw: empty_raw("Enumeration"),
                    dependencies: deps,
                    dependents,
                    expression: None,
                    int_min: None,
                    int_max: None,
                    int_inc: None,
                },
            )
        }
        NodeDecl::Boolean {
            name,
            access,
            pvalue,
            ..
        } => {
            let mut deps = Vec::new();
            if let Some(pv) = pvalue {
                deps.push(pv.clone());
            }

            (
                name.clone(),
                UiNode {
                    name: name.clone(),
                    kind: UiNodeKind::Boolean,
                    display_name: None,
                    comment: None,
                    tooltip: None,
                    description: None,
                    visibility: None,
                    access_mode: Some(access_mode_str(*access)),
                    unit: None,
                    representation: None,
                    constraints: None,
                    enum_entries: vec![],
                    raw: empty_raw("Boolean"),
                    dependencies: deps,
                    dependents,
                    expression: None,
                    int_min: None,
                    int_max: None,
                    int_inc: None,
                },
            )
        }
        NodeDecl::Command { name, pvalue, .. } => {
            let mut deps = Vec::new();
            if let Some(pv) = pvalue {
                deps.push(pv.clone());
            }

            (
                name.clone(),
                UiNode {
                    name: name.clone(),
                    kind: UiNodeKind::Command,
                    display_name: None,
                    comment: None,
                    tooltip: None,
                    description: None,
                    visibility: None,
                    access_mode: Some("WO".to_string()),
                    unit: None,
                    representation: None,
                    constraints: None,
                    enum_entries: vec![],
                    raw: empty_raw("Command"),
                    dependencies: deps,
                    dependents,
                    expression: None,
                    int_min: None,
                    int_max: None,
                    int_inc: None,
                },
            )
        }
        NodeDecl::Category { name, children } => (
            name.clone(),
            UiNode {
                name: name.clone(),
                kind: UiNodeKind::Category,
                display_name: None,
                comment: None,
                tooltip: None,
                description: None,
                visibility: None,
                access_mode: None,
                unit: None,
                representation: None,
                constraints: None,
                enum_entries: vec![],
                raw: empty_raw("Category"),
                dependencies: children.clone(),
                dependents: vec![],
                expression: None,
                int_min: None,
                int_max: None,
                int_inc: None,
            },
        ),
        NodeDecl::SwissKnife(sk) => {
            let deps: Vec<String> = sk
                .variables
                .iter()
                .map(|(_, target)| target.clone())
                .collect();
            (
                sk.name.clone(),
                UiNode {
                    name: sk.name.clone(),
                    kind: UiNodeKind::Unknown {
                        tag: "SwissKnife".to_string(),
                    },
                    display_name: None,
                    comment: None,
                    tooltip: None,
                    description: None,
                    visibility: None,
                    access_mode: Some("RO".to_string()),
                    unit: None,
                    representation: None,
                    constraints: None,
                    enum_entries: vec![],
                    raw: empty_raw("SwissKnife"),
                    dependencies: deps,
                    dependents,
                    expression: Some(sk.expr.clone()),
                    int_min: None,
                    int_max: None,
                    int_inc: None,
                },
            )
        }
        NodeDecl::Converter(cv) => {
            let mut deps = vec![cv.p_value.clone()];
            for (_, target) in &cv.variables_to {
                deps.push(target.clone());
            }
            for (_, target) in &cv.variables_from {
                deps.push(target.clone());
            }
            (
                cv.name.clone(),
                UiNode {
                    name: cv.name.clone(),
                    kind: UiNodeKind::Float,
                    display_name: None,
                    comment: None,
                    tooltip: None,
                    description: None,
                    visibility: None,
                    access_mode: Some("RO".to_string()),
                    unit: cv.unit.clone(),
                    representation: None,
                    constraints: None,
                    enum_entries: vec![],
                    raw: empty_raw("Converter"),
                    dependencies: deps,
                    dependents,
                    expression: Some(cv.formula_to.clone()),
                    int_min: None,
                    int_max: None,
                    int_inc: None,
                },
            )
        }
        NodeDecl::IntConverter(cv) => {
            let mut deps = vec![cv.p_value.clone()];
            for (_, target) in &cv.variables_to {
                deps.push(target.clone());
            }
            for (_, target) in &cv.variables_from {
                deps.push(target.clone());
            }
            (
                cv.name.clone(),
                UiNode {
                    name: cv.name.clone(),
                    kind: UiNodeKind::Integer,
                    display_name: None,
                    comment: None,
                    tooltip: None,
                    description: None,
                    visibility: None,
                    access_mode: Some("RO".to_string()),
                    unit: None,
                    representation: None,
                    constraints: None,
                    enum_entries: vec![],
                    raw: empty_raw("IntConverter"),
                    dependencies: deps,
                    dependents,
                    expression: Some(cv.formula_to.clone()),
                    int_min: None,
                    int_max: None,
                    int_inc: None,
                },
            )
        }
        NodeDecl::String(s) => (
            s.name.clone(),
            UiNode {
                name: s.name.clone(),
                kind: UiNodeKind::String,
                display_name: None,
                comment: None,
                tooltip: None,
                description: None,
                visibility: None,
                access_mode: Some(access_mode_str(s.access)),
                unit: None,
                representation: None,
                constraints: None,
                enum_entries: vec![],
                raw: empty_raw("String"),
                dependencies: vec![],
                dependents,
                expression: None,
                int_min: None,
                int_max: None,
                int_inc: None,
            },
        ),
    }
}

fn decl_name(decl: &NodeDecl) -> String {
    match decl {
        NodeDecl::Integer { name, .. }
        | NodeDecl::Float { name, .. }
        | NodeDecl::Enum { name, .. }
        | NodeDecl::Boolean { name, .. }
        | NodeDecl::Command { name, .. }
        | NodeDecl::Category { name, .. } => name.clone(),
        NodeDecl::SwissKnife(sk) => sk.name.clone(),
        NodeDecl::Converter(cv) => cv.name.clone(),
        NodeDecl::IntConverter(cv) => cv.name.clone(),
        NodeDecl::String(s) => s.name.clone(),
    }
}

fn empty_raw(tag: &str) -> RawNode {
    RawNode {
        tag: tag.to_string(),
        attributes: HashMap::new(),
        children_text: HashMap::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = r#"
        <RegisterDescription SchemaMajorVersion="1" SchemaMinorVersion="2" SchemaSubMinorVersion="3">
            <Category Name="Root">
                <pFeature>Width</pFeature>
                <pFeature>ExposureTime</pFeature>
                <pFeature>GainSelector</pFeature>
                <pFeature>GammaEnable</pFeature>
                <pFeature>AcquisitionStart</pFeature>
            </Category>
            <Integer Name="Width">
                <Address>0x0000_0100</Address>
                <Length>4</Length>
                <AccessMode>RW</AccessMode>
                <Min>16</Min>
                <Max>4096</Max>
                <Inc>2</Inc>
            </Integer>
            <Float Name="ExposureTime">
                <Address>0x0000_0200</Address>
                <Length>4</Length>
                <AccessMode>RW</AccessMode>
                <Min>10.0</Min>
                <Max>200000.0</Max>
                <Scale>1/1000</Scale>
                <Offset>0.0</Offset>
            </Float>
            <Enumeration Name="GainSelector">
                <Address>0x0000_0300</Address>
                <Length>2</Length>
                <AccessMode>RW</AccessMode>
                <EnumEntry Name="AnalogAll" Value="0" />
                <EnumEntry Name="DigitalAll" Value="1" />
            </Enumeration>
            <Boolean Name="GammaEnable">
                <Address>0x0000_0400</Address>
                <Length>1</Length>
                <AccessMode>RW</AccessMode>
            </Boolean>
            <Command Name="AcquisitionStart">
                <Address>0x0000_0500</Address>
                <Length>4</Length>
            </Command>
        </RegisterDescription>
    "#;

    #[test]
    fn test_parse_full_basic_fixture() {
        let graph = parse_full(FIXTURE).expect("parse full fixture");
        assert_eq!(graph.root_category, "Root");
        assert!(graph.categories.contains_key("Root"));

        let root = &graph.categories["Root"];
        assert_eq!(root.features.len(), 5);

        let width = graph.nodes_by_name.get("Width").expect("Width present");
        assert!(matches!(width.kind, UiNodeKind::Integer));
        assert_eq!(width.int_min, Some(16));
        assert_eq!(width.int_max, Some(4096));
        assert_eq!(width.int_inc, Some(2));
        assert_eq!(width.access_mode.as_deref(), Some("RW"));

        let constraints = width.constraints.as_ref().expect("constraints");
        assert_eq!(constraints.min, Some(16.0));
        assert_eq!(constraints.max, Some(4096.0));
        assert_eq!(constraints.inc, Some(2.0));
    }

    #[test]
    fn test_parse_full_enum_entries() {
        let graph = parse_full(FIXTURE).expect("parse full fixture");

        let gain_sel = graph
            .nodes_by_name
            .get("GainSelector")
            .expect("GainSelector present");
        assert!(matches!(gain_sel.kind, UiNodeKind::Enumeration));
        assert_eq!(gain_sel.enum_entries.len(), 2);
        assert_eq!(gain_sel.enum_entries[0].name, "AnalogAll");
        assert_eq!(gain_sel.enum_entries[0].value.as_deref(), Some("0"));
        assert_eq!(gain_sel.enum_entries[1].name, "DigitalAll");
    }

    #[test]
    fn test_parse_full_command_access_mode() {
        let graph = parse_full(FIXTURE).expect("parse full fixture");

        let cmd = graph
            .nodes_by_name
            .get("AcquisitionStart")
            .expect("AcquisitionStart present");
        assert!(matches!(cmd.kind, UiNodeKind::Command));
        assert_eq!(cmd.access_mode.as_deref(), Some("WO"));
    }

    #[test]
    fn test_parse_full_empty_xml() {
        let xml = r#"
            <RegisterDescription SchemaMajorVersion="1" SchemaMinorVersion="0" SchemaSubMinorVersion="0">
            </RegisterDescription>
        "#;
        let graph = parse_full(xml).expect("parse empty xml");
        assert!(graph.nodes_by_name.is_empty());
        assert!(graph.categories.is_empty());
    }

    #[test]
    fn test_parse_full_swissknife_expression() {
        let xml = r#"
            <RegisterDescription SchemaMajorVersion="1" SchemaMinorVersion="0" SchemaSubMinorVersion="0">
                <Integer Name="GainRaw">
                    <Address>0x3000</Address>
                    <Length>4</Length>
                    <AccessMode>RW</AccessMode>
                    <Min>0</Min>
                    <Max>1000</Max>
                </Integer>
                <SwissKnife Name="ComputedGain">
                    <Expression>(GainRaw * 0.5)</Expression>
                    <pVariable Name="GainRaw">GainRaw</pVariable>
                    <Output>Float</Output>
                </SwissKnife>
            </RegisterDescription>
        "#;
        let graph = parse_full(xml).expect("parse swissknife");

        let sk = graph
            .nodes_by_name
            .get("ComputedGain")
            .expect("SwissKnife present");
        assert!(matches!(
            &sk.kind,
            UiNodeKind::Unknown { tag } if tag == "SwissKnife"
        ));
        assert_eq!(sk.expression.as_deref(), Some("(GainRaw * 0.5)"));
        assert_eq!(sk.dependencies, vec!["GainRaw"]);
    }

    #[test]
    fn test_access_mode_str_all_variants() {
        assert_eq!(access_mode_str(AccessMode::RO), "RO");
        assert_eq!(access_mode_str(AccessMode::WO), "WO");
        assert_eq!(access_mode_str(AccessMode::RW), "RW");
    }

    #[test]
    fn test_empty_raw_creates_correct_structure() {
        let raw = empty_raw("TestTag");
        assert_eq!(raw.tag, "TestTag");
        assert!(raw.attributes.is_empty());
        assert!(raw.children_text.is_empty());
    }

    #[test]
    fn test_decl_name_all_variants() {
        let int_decl = NodeDecl::Integer {
            name: "W".to_string(),
            addressing: None,
            len: 4,
            access: AccessMode::RW,
            min: 0,
            max: 100,
            inc: None,
            unit: None,
            bitfield: None,
            selectors: vec![],
            selected_if: vec![],
            pvalue: None,
            p_max: None,
            p_min: None,
            value: None,
        };
        assert_eq!(decl_name(&int_decl), "W");

        let cat_decl = NodeDecl::Category {
            name: "Root".to_string(),
            children: vec![],
        };
        assert_eq!(decl_name(&cat_decl), "Root");
    }
}
