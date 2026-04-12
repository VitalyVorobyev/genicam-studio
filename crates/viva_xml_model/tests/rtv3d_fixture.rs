use viva_xml_model::parse_genicam_xml;

#[test]
fn parse_rtv3d_fixture_tolerates_groups_and_propagates_metadata() {
    let xml = include_str!("../fixtures/rtv3d_genicam.xml");
    let graph = parse_genicam_xml(xml).expect("parse rtv3d fixture");

    assert_eq!(graph.root_category, "Root");

    let device_information = graph
        .nodes_by_name
        .get("DeviceInformation")
        .expect("DeviceInformation category node present");
    assert_eq!(
        device_information.tooltip.as_deref(),
        Some("The device information category provides description of the camera and its sensor")
    );

    // Nodes inside <Group> elements must be parsed (Group is transparent).
    let device_vendor = graph
        .nodes_by_name
        .get("DeviceVendorName")
        .expect("DeviceVendorName node present (inside Group)");
    assert_eq!(
        device_vendor.tooltip.as_deref(),
        Some("Indicates the name of the device vendor")
    );

    let image_format_category = graph
        .categories
        .get("ImageFormatControl")
        .expect("ImageFormatControl category present");
    assert_eq!(
        image_format_category.tooltip.as_deref(),
        Some("Category for Image Format Control features.")
    );
}
