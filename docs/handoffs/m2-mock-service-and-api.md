# M2 Handoff: Mock Camera Service & API Polish

## Goal

Build a **mock Zenoh camera service** binary that simulates a GenICam camera, and polish the Zenoh API contract. This enables end-to-end UI development and CI testing without real hardware.

**Scope:** MS-01 through MS-11, ZA-03, ZA-07 from `docs/backlog.md`

**Output:**
1. `apps/genicam-mock-service/` — a runnable binary that behaves like a real camera service over Zenoh
2. Updated `genicam_zenoh_api` crate with PixelFormat enum and any missing types
3. Updated `docs/zenoh-api.md` with reviewed, complete spec

---

## Why This First

Without the mock service, all UI development requires a real camera + real camera service. The mock service:
- Unblocks UI development (Image Viewer, Feature Browser improvements)
- Enables CI integration tests
- Validates the Zenoh API contract end-to-end
- Provides a demo mode for the app

---

## Part A: Mock Camera Service Binary

### MS-01: Binary Scaffold (M)

**Create `apps/genicam-mock-service/`**

```
apps/genicam-mock-service/
  Cargo.toml
  src/
    main.rs           # CLI, Zenoh session, shutdown
    config.rs         # MockConfig struct
    state.rs          # NodeStore, AcquisitionState
    discovery.rs      # MS-02: announce publisher
    xml.rs            # MS-03: XML queryable
    nodes.rs          # MS-04, MS-05: node value store & queryables
    acquisition.rs    # MS-07, MS-08: image generation & control
    status.rs         # MS-06: status publisher
```

**Cargo.toml:**
```toml
[package]
name = "genicam-mock-service"
version = "0.1.0"
edition = "2021"

[dependencies]
genicam_zenoh_api = { path = "../../crates/genicam_zenoh_api" }
genicam_xml_model = { path = "../../crates/genicam_xml_model" }
clap = { version = "4", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["full"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
zenoh = "1"
```

Add to workspace `Cargo.toml` members.

**CLI interface:**
```
genicam-mock-service [OPTIONS]

Options:
  --device-id <ID>          Device ID [default: mock-cam0]
  --device-name <NAME>      Display name [default: "Mock Camera 1"]
  --model <MODEL>           Model string [default: "MockCam-1000"]
  --serial <SERIAL>         Serial number [default: "MOCK00001"]
  --width <PX>              Image width [default: 640]
  --height <PX>             Image height [default: 480]
  --fps <FPS>               Acquisition frame rate [default: 30]
  --fixture <PATH>          GenICam XML fixture to serve (defaults to built-in SFNC fixture)
  --zenoh-config <PATH>     Zenoh configuration file
```

**main.rs pattern:**
```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_tracing();
    let cli = Cli::parse();
    let config = MockConfig::from_cli(&cli);

    let zenoh_config = load_zenoh_config(cli.zenoh_config.as_deref())?;
    let session = Arc::new(zenoh::open(zenoh_config).await?);

    // Parse fixture XML into UiGraph for node defaults
    let xml = load_fixture_xml(&cli.fixture);
    let graph = genicam_xml_model::parse_genicam_xml(&xml)?;

    // Initialize node store with defaults from XML
    let node_store = Arc::new(NodeStore::from_graph(&graph, &config));

    // Start all tasks
    let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);

    let tasks = vec![
        tokio::spawn(discovery::run(session.clone(), config.clone(), shutdown_rx.clone())),
        tokio::spawn(xml::run(session.clone(), config.clone(), xml.clone(), shutdown_rx.clone())),
        tokio::spawn(nodes::run_publisher(session.clone(), config.clone(), node_store.clone(), shutdown_rx.clone())),
        tokio::spawn(nodes::run_set_queryable(session.clone(), config.clone(), node_store.clone(), shutdown_rx.clone())),
        tokio::spawn(nodes::run_execute_queryable(session.clone(), config.clone(), shutdown_rx.clone())),
        tokio::spawn(acquisition::run(session.clone(), config.clone(), node_store.clone(), shutdown_rx.clone())),
        tokio::spawn(status::run(session.clone(), config.clone(), shutdown_rx.clone())),
    ];

    tokio::signal::ctrl_c().await?;
    let _ = shutdown_tx.send(true);
    for task in tasks {
        let _ = task.await;
    }
    session.close().await?;
    Ok(())
}
```

---

### MS-02: Discovery Announcer (S)

Publish `DeviceAnnounce` every 2 seconds to `genicam/devices/{id}/announce`.

```rust
pub async fn run(session: Arc<zenoh::Session>, config: MockConfig, mut shutdown: watch::Receiver<bool>) {
    let key = genicam_zenoh_api::keys::announce(&config.device_id);
    let announce = DeviceAnnounce {
        id: config.device_id.clone(),
        name: config.device_name.clone(),
        model: config.model.clone(),
        serial: config.serial.clone(),
    };
    let payload = serde_json::to_vec(&announce).unwrap();

    let publisher = session.declare_publisher(&key).await.unwrap();
    let mut interval = tokio::time::interval(Duration::from_secs(2));

    loop {
        tokio::select! {
            _ = shutdown.changed() => { if *shutdown.borrow() { break; } }
            _ = interval.tick() => {
                let _ = publisher.put(payload.clone()).await;
            }
        }
    }
}
```

---

### MS-03: XML Queryable (S)

Serve the fixture XML on `genicam/devices/{id}/xml`.

```rust
pub async fn run(session: Arc<zenoh::Session>, config: MockConfig, xml: String, mut shutdown: watch::Receiver<bool>) {
    let key = genicam_zenoh_api::keys::xml(&config.device_id);
    let queryable = session.declare_queryable(&key).await.unwrap();
    let response = DeviceXmlResponse { xml };
    let payload = serde_json::to_vec(&response).unwrap();

    loop {
        tokio::select! {
            _ = shutdown.changed() => { if *shutdown.borrow() { break; } }
            query = queryable.recv_async() => {
                if let Ok(query) = query {
                    let _ = query.reply(&key, payload.clone()).await;
                }
            }
        }
    }
}
```

---

### MS-04: Node Value Store (M)

In-memory store seeded from UiGraph defaults. Publishes changes.

```rust
pub struct NodeStore {
    values: RwLock<HashMap<String, NodeEntry>>,
    change_tx: broadcast::Sender<(String, NodeValueUpdate)>,
}

pub struct NodeEntry {
    pub value: serde_json::Value,
    pub access_mode: String,
    pub kind: String, // "Integer", "Float", "Boolean", "String", "Enumeration"
    pub constraints: Option<NodeConstraints>,
}

pub struct NodeConstraints {
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub inc: Option<f64>,
    pub enum_values: Vec<String>,
}

impl NodeStore {
    /// Seed from UiGraph: for each node, extract a sensible default value.
    pub fn from_graph(graph: &UiGraph, config: &MockConfig) -> Self {
        let mut values = HashMap::new();

        for (name, node) in &graph.nodes_by_name {
            let entry = match &node.kind {
                UiNodeKind::Integer => {
                    let default = node.constraints.as_ref()
                        .and_then(|c| c.min)
                        .unwrap_or(0.0) as i64;
                    NodeEntry {
                        value: serde_json::Value::Number(default.into()),
                        access_mode: node.access_mode.clone().unwrap_or("RW".to_string()),
                        kind: "Integer".to_string(),
                        constraints: node.constraints.as_ref().map(|c| NodeConstraints {
                            min: c.min, max: c.max, inc: c.inc,
                            enum_values: vec![],
                        }),
                    }
                }
                UiNodeKind::Float => { /* similar, default from min or 0.0 */ }
                UiNodeKind::Boolean => { /* default false */ }
                UiNodeKind::Enumeration => { /* default first enum entry */ }
                UiNodeKind::String => { /* default "" */ }
                _ => continue, // Skip Category, Command, Register, Unknown
            };
            values.insert(name.clone(), entry);
        }

        // Override with standard SFNC defaults
        Self::apply_sfnc_defaults(&mut values, config);

        let (change_tx, _) = broadcast::channel(256);
        Self { values: RwLock::new(values), change_tx }
    }

    /// Apply well-known SFNC node defaults
    fn apply_sfnc_defaults(values: &mut HashMap<String, NodeEntry>, config: &MockConfig) {
        // Width / Height from config
        if let Some(entry) = values.get_mut("Width") {
            entry.value = serde_json::json!(config.width);
        }
        if let Some(entry) = values.get_mut("Height") {
            entry.value = serde_json::json!(config.height);
        }
        // ExposureTime: 10000 us (10ms)
        if let Some(entry) = values.get_mut("ExposureTime") {
            entry.value = serde_json::json!(10000.0);
        }
        // Gain: 1.0
        if let Some(entry) = values.get_mut("Gain") {
            entry.value = serde_json::json!(1.0);
        }
        // PixelFormat: Mono8
        if let Some(entry) = values.get_mut("PixelFormat") {
            entry.value = serde_json::json!("Mono8");
        }
    }

    pub async fn get(&self, name: &str) -> Option<NodeValueUpdate> { /* ... */ }

    pub async fn set(&self, name: &str, value: serde_json::Value) -> Result<(), String> {
        // Validate type and constraints
        // Update store
        // Broadcast change
    }

    pub fn subscribe(&self) -> broadcast::Receiver<(String, NodeValueUpdate)> {
        self.change_tx.subscribe()
    }
}
```

**Node value publisher task:** subscribes to `NodeStore::subscribe()`, publishes each change to the appropriate Zenoh key.

---

### MS-05: Node Set/Execute Queryables (M)

**Set queryable** at `genicam/devices/{id}/nodes/*/set`:
- One queryable per node, OR use a single wildcard queryable and extract node name from key expression
- Parse `NodeSetRequest`, call `NodeStore::set()`, reply with `NodeOpResponse`

**Execute queryable** at `genicam/devices/{id}/nodes/*/execute`:
- For Command nodes: reply `{ ok: true }` immediately
- Log the execution for diagnostics

---

### MS-06: Status Publisher (S)

Publish `DeviceStatus { connected: true, error: None }` once at startup and on simulated events.

---

### MS-07: Synthetic Image Generator (M)

Generate test pattern images at the configured FPS.

```rust
pub fn generate_mono8(width: u32, height: u32, frame_id: u64) -> Vec<u8> {
    let mut pixels = vec![0u8; (width * height) as usize];
    for y in 0..height {
        for x in 0..width {
            let idx = (y * width + x) as usize;
            // Animated gradient with moving bars
            let bar = ((x as u64 + frame_id * 2) % 256) as u8;
            let gradient = ((y as f32 / height as f32) * 255.0) as u8;
            pixels[idx] = bar.wrapping_add(gradient);
        }
    }
    pixels
}

pub fn generate_checkerboard(width: u32, height: u32, cell_size: u32) -> Vec<u8> {
    let mut pixels = vec![0u8; (width * height) as usize];
    for y in 0..height {
        for x in 0..width {
            let idx = (y * width + x) as usize;
            let cx = x / cell_size;
            let cy = y / cell_size;
            pixels[idx] = if (cx + cy) % 2 == 0 { 220 } else { 35 };
        }
    }
    pixels
}
```

**Acquisition loop:** when active, generate a frame, publish to `{id}/image`, sleep for `1/fps` interval.

The image should visibly react to node changes:
- **ExposureTime**: affects brightness (scale pixel values)
- **Gain**: also affects brightness (multiply)
- **Width/Height**: change image dimensions
- **PixelFormat**: change pattern type (if multi-format enabled)

This makes the mock service useful for visual verification of the UI.

---

### MS-08: Acquisition Control (S)

Queryable at `genicam/devices/{id}/acquisition/control`. On `start`:
- Set acquisition active
- Publish `AcquisitionStatus { active: true, fps: Some(config.fps), dropped: 0 }`
- Start image generation loop

On `stop`:
- Set acquisition inactive
- Publish status with `active: false`
- Stop image generation loop

---

### MS-11: Realistic SFNC Node Set (M)

**Create a comprehensive GenICam XML fixture** at `apps/genicam-mock-service/fixtures/sfnc-standard.xml` with these SFNC standard features:

```xml
<?xml version="1.0"?>
<RegisterDescription
  ModelName="MockCam-1000"
  VendorName="GenICam Studio"
  StandardNameSpace="SFNC"
  SchemaMajorVersion="1"
  SchemaMinorVersion="1">

  <!-- Root -->
  <Category Name="Root">
    <pFeature>AcquisitionControl</pFeature>
    <pFeature>AnalogControl</pFeature>
    <pFeature>ImageFormatControl</pFeature>
    <pFeature>TriggerControl</pFeature>
    <pFeature>TransportLayerControl</pFeature>
    <pFeature>DeviceControl</pFeature>
    <pFeature>UserSetControl</pFeature>
  </Category>

  <!-- Acquisition Control -->
  <Category Name="AcquisitionControl">
    <DisplayName>Acquisition Control</DisplayName>
    <Visibility>Beginner</Visibility>
    <pFeature>AcquisitionMode</pFeature>
    <pFeature>AcquisitionStart</pFeature>
    <pFeature>AcquisitionStop</pFeature>
    <pFeature>AcquisitionFrameRate</pFeature>
    <pFeature>AcquisitionFrameRateEnable</pFeature>
    <pFeature>ExposureMode</pFeature>
    <pFeature>ExposureTime</pFeature>
    <pFeature>ExposureAuto</pFeature>
  </Category>

  <!-- Analog Control -->
  <Category Name="AnalogControl">
    <DisplayName>Analog Control</DisplayName>
    <Visibility>Beginner</Visibility>
    <pFeature>Gain</pFeature>
    <pFeature>GainAuto</pFeature>
    <pFeature>BlackLevel</pFeature>
    <pFeature>Gamma</pFeature>
  </Category>

  <!-- Image Format Control -->
  <Category Name="ImageFormatControl">
    <DisplayName>Image Format Control</DisplayName>
    <Visibility>Beginner</Visibility>
    <pFeature>Width</pFeature>
    <pFeature>Height</pFeature>
    <pFeature>WidthMax</pFeature>
    <pFeature>HeightMax</pFeature>
    <pFeature>OffsetX</pFeature>
    <pFeature>OffsetY</pFeature>
    <pFeature>PixelFormat</pFeature>
    <pFeature>BinningHorizontal</pFeature>
    <pFeature>BinningVertical</pFeature>
    <pFeature>ReverseX</pFeature>
    <pFeature>ReverseY</pFeature>
  </Category>

  <!-- Trigger Control -->
  <Category Name="TriggerControl">
    <DisplayName>Trigger Control</DisplayName>
    <Visibility>Expert</Visibility>
    <pFeature>TriggerMode</pFeature>
    <pFeature>TriggerSource</pFeature>
    <pFeature>TriggerActivation</pFeature>
    <pFeature>TriggerDelay</pFeature>
    <pFeature>TriggerSoftware</pFeature>
  </Category>

  <!-- Transport Layer Control -->
  <Category Name="TransportLayerControl">
    <DisplayName>Transport Layer Control</DisplayName>
    <Visibility>Guru</Visibility>
    <pFeature>PayloadSize</pFeature>
    <pFeature>GevSCPSPacketSize</pFeature>
    <pFeature>GevSCPD</pFeature>
  </Category>

  <!-- Device Control -->
  <Category Name="DeviceControl">
    <DisplayName>Device Control</DisplayName>
    <Visibility>Expert</Visibility>
    <pFeature>DeviceTemperature</pFeature>
    <pFeature>DeviceReset</pFeature>
    <pFeature>DeviceFirmwareVersion</pFeature>
    <pFeature>DeviceSerialNumber</pFeature>
  </Category>

  <!-- User Set Control -->
  <Category Name="UserSetControl">
    <DisplayName>User Set Control</DisplayName>
    <Visibility>Expert</Visibility>
    <pFeature>UserSetSelector</pFeature>
    <pFeature>UserSetLoad</pFeature>
    <pFeature>UserSetSave</pFeature>
  </Category>

  <!-- ═══════════ Nodes ═══════════ -->

  <!-- Acquisition -->
  <Enumeration Name="AcquisitionMode">
    <DisplayName>Acquisition Mode</DisplayName>
    <Visibility>Beginner</Visibility>
    <EnumEntry Name="Continuous"><Value>0</Value><DisplayName>Continuous</DisplayName></EnumEntry>
    <EnumEntry Name="SingleFrame"><Value>1</Value><DisplayName>Single Frame</DisplayName></EnumEntry>
    <EnumEntry Name="MultiFrame"><Value>2</Value><DisplayName>Multi Frame</DisplayName></EnumEntry>
  </Enumeration>

  <Command Name="AcquisitionStart">
    <DisplayName>Acquisition Start</DisplayName>
    <Visibility>Beginner</Visibility>
  </Command>

  <Command Name="AcquisitionStop">
    <DisplayName>Acquisition Stop</DisplayName>
    <Visibility>Beginner</Visibility>
  </Command>

  <Float Name="AcquisitionFrameRate">
    <DisplayName>Acquisition Frame Rate</DisplayName>
    <Visibility>Beginner</Visibility>
    <Unit>Hz</Unit>
    <Min>0.1</Min>
    <Max>120.0</Max>
    <Inc>0.1</Inc>
  </Float>

  <Boolean Name="AcquisitionFrameRateEnable">
    <DisplayName>Frame Rate Enable</DisplayName>
    <Visibility>Expert</Visibility>
  </Boolean>

  <!-- Exposure -->
  <Enumeration Name="ExposureMode">
    <DisplayName>Exposure Mode</DisplayName>
    <Visibility>Expert</Visibility>
    <EnumEntry Name="Timed"><Value>0</Value></EnumEntry>
    <EnumEntry Name="TriggerWidth"><Value>1</Value></EnumEntry>
  </Enumeration>

  <Float Name="ExposureTime">
    <DisplayName>Exposure Time</DisplayName>
    <Visibility>Beginner</Visibility>
    <Unit>us</Unit>
    <Min>10.0</Min>
    <Max>1000000.0</Max>
    <Inc>1.0</Inc>
  </Float>

  <Enumeration Name="ExposureAuto">
    <DisplayName>Exposure Auto</DisplayName>
    <Visibility>Beginner</Visibility>
    <EnumEntry Name="Off"><Value>0</Value></EnumEntry>
    <EnumEntry Name="Once"><Value>1</Value></EnumEntry>
    <EnumEntry Name="Continuous"><Value>2</Value></EnumEntry>
  </Enumeration>

  <!-- Gain -->
  <Float Name="Gain">
    <DisplayName>Gain</DisplayName>
    <Visibility>Beginner</Visibility>
    <Unit>dB</Unit>
    <Min>0.0</Min>
    <Max>48.0</Max>
    <Inc>0.1</Inc>
  </Float>

  <Enumeration Name="GainAuto">
    <DisplayName>Gain Auto</DisplayName>
    <Visibility>Beginner</Visibility>
    <EnumEntry Name="Off"><Value>0</Value></EnumEntry>
    <EnumEntry Name="Once"><Value>1</Value></EnumEntry>
    <EnumEntry Name="Continuous"><Value>2</Value></EnumEntry>
  </Enumeration>

  <Float Name="BlackLevel">
    <DisplayName>Black Level</DisplayName>
    <Visibility>Expert</Visibility>
    <Unit>DN</Unit>
    <Min>0.0</Min>
    <Max>255.0</Max>
    <Inc>1.0</Inc>
  </Float>

  <Float Name="Gamma">
    <DisplayName>Gamma</DisplayName>
    <Visibility>Expert</Visibility>
    <Min>0.1</Min>
    <Max>4.0</Max>
    <Inc>0.01</Inc>
  </Float>

  <!-- Image Format -->
  <Integer Name="Width">
    <DisplayName>Width</DisplayName>
    <Visibility>Beginner</Visibility>
    <Unit>px</Unit>
    <Min>1</Min>
    <Max>4096</Max>
    <Inc>1</Inc>
  </Integer>

  <Integer Name="Height">
    <DisplayName>Height</DisplayName>
    <Visibility>Beginner</Visibility>
    <Unit>px</Unit>
    <Min>1</Min>
    <Max>3072</Max>
    <Inc>1</Inc>
  </Integer>

  <Integer Name="WidthMax">
    <DisplayName>Width Max</DisplayName>
    <Visibility>Expert</Visibility>
    <AccessMode>RO</AccessMode>
  </Integer>

  <Integer Name="HeightMax">
    <DisplayName>Height Max</DisplayName>
    <Visibility>Expert</Visibility>
    <AccessMode>RO</AccessMode>
  </Integer>

  <Integer Name="OffsetX">
    <DisplayName>Offset X</DisplayName>
    <Visibility>Beginner</Visibility>
    <Unit>px</Unit>
    <Min>0</Min>
    <Max>4095</Max>
    <Inc>1</Inc>
  </Integer>

  <Integer Name="OffsetY">
    <DisplayName>Offset Y</DisplayName>
    <Visibility>Beginner</Visibility>
    <Unit>px</Unit>
    <Min>0</Min>
    <Max>3071</Max>
    <Inc>1</Inc>
  </Integer>

  <Enumeration Name="PixelFormat">
    <DisplayName>Pixel Format</DisplayName>
    <Visibility>Beginner</Visibility>
    <EnumEntry Name="Mono8"><Value>0x01080001</Value><DisplayName>Mono 8</DisplayName></EnumEntry>
    <EnumEntry Name="Mono12"><Value>0x01100005</Value><DisplayName>Mono 12</DisplayName></EnumEntry>
    <EnumEntry Name="Mono16"><Value>0x01100007</Value><DisplayName>Mono 16</DisplayName></EnumEntry>
    <EnumEntry Name="BayerRG8"><Value>0x01080009</Value><DisplayName>Bayer RG 8</DisplayName></EnumEntry>
    <EnumEntry Name="RGB8"><Value>0x02180014</Value><DisplayName>RGB 8</DisplayName></EnumEntry>
  </Enumeration>

  <Integer Name="BinningHorizontal">
    <DisplayName>Binning Horizontal</DisplayName>
    <Visibility>Expert</Visibility>
    <Min>1</Min>
    <Max>4</Max>
    <Inc>1</Inc>
  </Integer>

  <Integer Name="BinningVertical">
    <DisplayName>Binning Vertical</DisplayName>
    <Visibility>Expert</Visibility>
    <Min>1</Min>
    <Max>4</Max>
    <Inc>1</Inc>
  </Integer>

  <Boolean Name="ReverseX">
    <DisplayName>Reverse X</DisplayName>
    <Visibility>Expert</Visibility>
  </Boolean>

  <Boolean Name="ReverseY">
    <DisplayName>Reverse Y</DisplayName>
    <Visibility>Expert</Visibility>
  </Boolean>

  <!-- Trigger -->
  <Enumeration Name="TriggerMode">
    <DisplayName>Trigger Mode</DisplayName>
    <Visibility>Expert</Visibility>
    <EnumEntry Name="Off"><Value>0</Value></EnumEntry>
    <EnumEntry Name="On"><Value>1</Value></EnumEntry>
  </Enumeration>

  <Enumeration Name="TriggerSource">
    <DisplayName>Trigger Source</DisplayName>
    <Visibility>Expert</Visibility>
    <EnumEntry Name="Software"><Value>0</Value></EnumEntry>
    <EnumEntry Name="Line1"><Value>1</Value></EnumEntry>
    <EnumEntry Name="Line2"><Value>2</Value></EnumEntry>
  </Enumeration>

  <Enumeration Name="TriggerActivation">
    <DisplayName>Trigger Activation</DisplayName>
    <Visibility>Expert</Visibility>
    <EnumEntry Name="RisingEdge"><Value>0</Value></EnumEntry>
    <EnumEntry Name="FallingEdge"><Value>1</Value></EnumEntry>
    <EnumEntry Name="AnyEdge"><Value>2</Value></EnumEntry>
  </Enumeration>

  <Float Name="TriggerDelay">
    <DisplayName>Trigger Delay</DisplayName>
    <Visibility>Expert</Visibility>
    <Unit>us</Unit>
    <Min>0.0</Min>
    <Max>1000000.0</Max>
  </Float>

  <Command Name="TriggerSoftware">
    <DisplayName>Trigger Software</DisplayName>
    <Visibility>Expert</Visibility>
  </Command>

  <!-- Transport -->
  <Integer Name="PayloadSize">
    <DisplayName>Payload Size</DisplayName>
    <Visibility>Guru</Visibility>
    <AccessMode>RO</AccessMode>
    <Unit>B</Unit>
  </Integer>

  <Integer Name="GevSCPSPacketSize">
    <DisplayName>Packet Size</DisplayName>
    <Visibility>Guru</Visibility>
    <Unit>B</Unit>
    <Min>220</Min>
    <Max>9000</Max>
    <Inc>4</Inc>
  </Integer>

  <Integer Name="GevSCPD">
    <DisplayName>Inter-Packet Delay</DisplayName>
    <Visibility>Guru</Visibility>
    <Unit>ticks</Unit>
    <Min>0</Min>
    <Max>10000</Max>
  </Integer>

  <!-- Device -->
  <Float Name="DeviceTemperature">
    <DisplayName>Device Temperature</DisplayName>
    <Visibility>Expert</Visibility>
    <AccessMode>RO</AccessMode>
    <Unit>C</Unit>
  </Float>

  <Command Name="DeviceReset">
    <DisplayName>Device Reset</DisplayName>
    <Visibility>Guru</Visibility>
  </Command>

  <String Name="DeviceFirmwareVersion">
    <DisplayName>Firmware Version</DisplayName>
    <Visibility>Expert</Visibility>
    <AccessMode>RO</AccessMode>
  </String>

  <String Name="DeviceSerialNumber">
    <DisplayName>Serial Number</DisplayName>
    <Visibility>Expert</Visibility>
    <AccessMode>RO</AccessMode>
  </String>

  <!-- User Set -->
  <Enumeration Name="UserSetSelector">
    <DisplayName>User Set Selector</DisplayName>
    <Visibility>Expert</Visibility>
    <EnumEntry Name="Default"><Value>0</Value></EnumEntry>
    <EnumEntry Name="UserSet1"><Value>1</Value></EnumEntry>
    <EnumEntry Name="UserSet2"><Value>2</Value></EnumEntry>
  </Enumeration>

  <Command Name="UserSetLoad">
    <DisplayName>User Set Load</DisplayName>
    <Visibility>Expert</Visibility>
  </Command>

  <Command Name="UserSetSave">
    <DisplayName>User Set Save</DisplayName>
    <Visibility>Expert</Visibility>
  </Command>

  <!-- Unknown node for preservation invariant -->
  <IntSwissKnife Name="SensorReadoutTime">
    <DisplayName>Sensor Readout Time</DisplayName>
    <Visibility>Guru</Visibility>
    <Formula>Height * 10</Formula>
  </IntSwissKnife>

</RegisterDescription>
```

This fixture covers all the SFNC groups needed for the Image Viewer (ADR-007).

---

## Part B: Zenoh API Polish

### ZA-03: PixelFormat Enum (S)

Add to `genicam_zenoh_api`:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PixelFormat {
    Mono8,
    Mono10,
    Mono12,
    Mono16,
    BayerRG8,
    BayerGR8,
    BayerBG8,
    BayerGB8,
    RGB8,
    BGR8,
    RGBa8,
    YCbCr422_8,
    #[serde(other)]
    Unknown,
}

impl PixelFormat {
    pub fn bytes_per_pixel(&self) -> f32 {
        match self {
            Self::Mono8 | Self::BayerRG8 | Self::BayerGR8 | Self::BayerBG8 | Self::BayerGB8 => 1.0,
            Self::Mono10 | Self::Mono12 => 2.0,
            Self::Mono16 => 2.0,
            Self::RGB8 | Self::BGR8 => 3.0,
            Self::RGBa8 => 4.0,
            Self::YCbCr422_8 => 2.0,
            Self::Unknown => 1.0,
        }
    }
}
```

### ZA-07: API Spec Review (M)

Review `docs/zenoh-api.md` and ensure:
1. Every key expression has a corresponding type in `genicam_zenoh_api`
2. Add `image/meta` key and `ImageMeta` type
3. Add sequence diagrams for: discovery → connect → stream → disconnect
4. Document error scenarios and recovery
5. Ensure Tauri backend code matches the spec exactly

---

## How to Test

### Manual testing loop:
```bash
# Terminal 1: Start mock service
cargo run -p genicam-mock-service

# Terminal 2: Start Tauri app
cd apps/genicam-studio-tauri && cargo tauri dev
```

The app should:
1. Show "Mock Camera 1" in the device sidebar
2. Connect → load feature tree with all SFNC categories
3. Change ExposureTime slider → see change in mock service logs
4. Start acquisition → see animated test pattern in Image Viewer
5. Change Gain → see brightness change in the pattern
6. Stop acquisition → stream stops

---

## Definition of Done

- [ ] `cargo test -p genicam-mock-service` passes
- [ ] `cargo clippy --all-targets -p genicam-mock-service -- -D warnings` clean
- [ ] Mock service starts, announces, serves XML
- [ ] Tauri app discovers mock device, connects, loads feature tree
- [ ] Node read/write works end-to-end (mock ↔ Zenoh ↔ Tauri ↔ UI)
- [ ] Acquisition produces visible animated frames in Image Viewer
- [ ] Image reacts to ExposureTime/Gain changes
- [ ] SFNC fixture covers all categories needed for Image Viewer sections
- [ ] `PixelFormat` enum added to `genicam_zenoh_api`
- [ ] `docs/zenoh-api.md` reviewed and updated
