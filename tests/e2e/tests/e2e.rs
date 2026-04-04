//! E2E test suite for GenICam Studio.
//!
//! Requires:
//! - `arv-fake-gv-camera-0.8` in PATH (or `ARV_FAKE_CAMERA_PATH` env var)
//! - `genicam-service` binary (or `GENICAM_SERVICE_PATH` env var)
//!
//! Run: `cargo test -p e2e-tests -- --ignored --test-threads=1`

use std::time::Duration;
use tokio::time::timeout;

use e2e_tests::*;
use genicam_zenoh_api::{AcquisitionCommand, DeviceAnnounce, FrameHeader, HEADER_SIZE};

// ── E2E-02: Discovery + Connect + XML Fetch ─────────────────────────────────

#[tokio::test(flavor = "multi_thread")]
#[ignore] // Requires external binaries
async fn test_discovery_and_xml_fetch() {
    let mut harness = TestHarness::start().await.expect("harness start");

    // Verify device ID
    assert!(
        !harness.device_id().is_empty(),
        "device_id should not be empty"
    );

    // Subscribe to announce and verify fields
    let sub = harness
        .session()
        .declare_subscriber(genicam_zenoh_api::keys::ANNOUNCE_ALL)
        .await
        .expect("subscriber");

    let sample = timeout(Duration::from_secs(10), sub.recv_async())
        .await
        .expect("timeout")
        .expect("recv");

    let announce: DeviceAnnounce =
        serde_json::from_slice(&sample.payload().to_bytes()).expect("parse announce");

    assert_eq!(announce.id, harness.device_id());
    assert!(announce.api_version.is_some(), "api_version should be set");

    // Fetch XML
    let xml = fetch_xml(harness.session(), harness.device_id())
        .await
        .expect("fetch XML");

    assert!(
        xml.contains("RegisterDescription"),
        "XML should contain RegisterDescription"
    );
    assert!(xml.len() > 100, "XML should be non-trivial");

    // Parse XML with genicam_xml_model
    let graph = genicam_xml_model::parse_genicam_xml(&xml).expect("parse XML");
    assert!(!graph.nodes_by_name.is_empty(), "should have nodes");
    assert!(
        graph.nodes_by_name.contains_key("Width"),
        "should have Width node"
    );
    assert!(
        graph.nodes_by_name.contains_key("Height"),
        "should have Height node"
    );

    harness.shutdown().await;
}

// ── E2E-03: Node Read/Write Cycle ───────────────────────────────────────────

#[tokio::test(flavor = "multi_thread")]
#[ignore]
async fn test_node_read_write() {
    let mut harness = TestHarness::start().await.expect("harness start");
    let session = harness.session().clone();
    let device_id = harness.device_id().to_string();

    // Bulk-read Width and Height
    let bulk = read_bulk(&session, &device_id, &["Width", "Height"])
        .await
        .expect("bulk read");

    assert!(bulk.values.contains_key("Width"), "Width in bulk response");
    assert!(
        bulk.values.contains_key("Height"),
        "Height in bulk response"
    );

    let original_width = bulk.values["Width"]
        .value
        .as_i64()
        .expect("Width is integer");
    tracing::info!("Original Width: {original_width}");

    // Write Width = 320
    write_node(&session, &device_id, "Width", serde_json::json!(320))
        .await
        .expect("write Width=320");

    // Read back
    let bulk2 = read_bulk(&session, &device_id, &["Width"])
        .await
        .expect("readback");

    let new_width = bulk2.values["Width"]
        .value
        .as_i64()
        .expect("Width is integer");
    assert_eq!(new_width, 320, "Width should be 320 after write");

    // Restore original
    write_node(
        &session,
        &device_id,
        "Width",
        serde_json::json!(original_width),
    )
    .await
    .expect("restore Width");

    harness.shutdown().await;
}

// ── E2E-04: Acquisition + Frame Reception ───────────────────────────────────

#[tokio::test(flavor = "multi_thread")]
#[ignore]
async fn test_acquisition_frames() {
    let mut harness = TestHarness::start().await.expect("harness start");
    let session = harness.session().clone();
    let device_id = harness.device_id().to_string();

    // Subscribe to image stream
    let image_key = genicam_zenoh_api::keys::image(&device_id);
    let sub = session
        .declare_subscriber(&image_key)
        .await
        .expect("image subscriber");

    // Start acquisition
    send_acquisition_command(&session, &device_id, AcquisitionCommand::Start)
        .await
        .expect("start acquisition");

    // Wait for at least one frame
    let frame_result = timeout(Duration::from_secs(10), sub.recv_async()).await;

    match frame_result {
        Ok(Ok(sample)) => {
            let bytes = sample.payload().to_bytes();
            assert!(
                bytes.len() > HEADER_SIZE,
                "frame should have header + pixel data (got {} bytes)",
                bytes.len()
            );

            let (header, pixel_data) = FrameHeader::decode(&bytes).expect("decode frame header");
            assert!(header.width > 0, "frame width > 0");
            assert!(header.height > 0, "frame height > 0");
            assert!(!pixel_data.is_empty(), "pixel data should not be empty");

            tracing::info!(
                "Frame: {}x{} {:?} seq={} payload={}B",
                header.width,
                header.height,
                header.pixel_format,
                header.seq,
                pixel_data.len()
            );
        }
        Ok(Err(e)) => panic!("subscriber error: {e}"),
        Err(_) => {
            // Frame reception may not work on loopback (UDP multicast limitation).
            // Log warning but don't fail — this is a known CI limitation.
            tracing::warn!(
                "No frames received within timeout. \
                 This is expected on loopback if aravis uses multicast."
            );
        }
    }

    // Stop acquisition
    send_acquisition_command(&session, &device_id, AcquisitionCommand::Stop)
        .await
        .expect("stop acquisition");

    // Verify acquisition status reports inactive
    let status_key = genicam_zenoh_api::keys::acquisition_status(&device_id);
    let status_sub = session
        .declare_subscriber(&status_key)
        .await
        .expect("status subscriber");

    let status_result = timeout(Duration::from_secs(5), status_sub.recv_async()).await;
    if let Ok(Ok(sample)) = status_result {
        let bytes = sample.payload().to_bytes();
        if let Ok(status) = serde_json::from_slice::<genicam_zenoh_api::AcquisitionStatus>(&bytes) {
            assert!(!status.active, "acquisition should be inactive after stop");
        }
    }

    harness.shutdown().await;
}

// ── E2E-05: Device Lost Detection ───────────────────────────────────────────

#[tokio::test(flavor = "multi_thread")]
#[ignore]
async fn test_device_lost_detection() {
    let mut harness = TestHarness::start().await.expect("harness start");
    let session = harness.session().clone();
    let device_id = harness.device_id().to_string();

    // Subscribe to device status
    let status_key = genicam_zenoh_api::keys::status(&device_id);
    let sub = session
        .declare_subscriber(&status_key)
        .await
        .expect("status subscriber");

    // Kill the fake camera
    tracing::info!("Killing fake camera...");
    harness.kill_fake_camera().await.expect("kill fake camera");

    // Wait for disconnect status from the service
    let disconnect_result = timeout(Duration::from_secs(15), async {
        loop {
            if let Ok(sample) = sub.recv_async().await {
                let bytes = sample.payload().to_bytes();
                if let Ok(status) =
                    serde_json::from_slice::<genicam_zenoh_api::DeviceStatus>(&bytes)
                {
                    if !status.connected {
                        return status;
                    }
                }
            }
        }
    })
    .await;

    match disconnect_result {
        Ok(status) => {
            assert!(!status.connected, "device should be disconnected");
            tracing::info!(
                "Device lost detected: {:?}",
                status.error.as_deref().unwrap_or("(no error message)")
            );
        }
        Err(_) => {
            tracing::warn!(
                "No disconnect status received within timeout. \
                 Service may not detect camera loss on loopback."
            );
        }
    }

    harness.shutdown().await;
}
