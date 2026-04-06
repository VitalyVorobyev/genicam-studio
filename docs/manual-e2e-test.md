# Manual E2E Test: Real Service + Fake Camera

## Setup (3 terminals)

**Terminal 1 — fake camera:**
```bash
arv-fake-gv-camera-0.8 -i 127.0.0.1
```

**Terminal 2 — camera service:**
```bash
cd ../genicam-rs/crates/genicam-service
cargo run -- --iface lo0 --zenoh-config ../../../genicam-studio/config/zenoh-local.json5
```

**Terminal 3 — studio:**
```bash
cd apps/genicam-studio-tauri
cargo tauri dev
```

## Test checklist

### 1. Discovery
- [ ] Device appears in sidebar within ~5s
- [ ] Shows name "Fake", device ID `cam-000000000000`

### 2. Connect
- [ ] Click device → feature tree loads
- [ ] Width=512, Height=512 visible in tree or Image Viewer controls

### 3. Node writes
- [ ] Change Width to 320 → readback shows 320
- [ ] Change Height to 240 → readback shows 240
- [ ] Change PixelFormat (Mono8 → RGB8 if available)
- [ ] Restore original values

### 4. Acquisition
- [ ] Click Start → image appears in viewer
- [ ] FPS counter shows non-zero value in header bar
- [ ] Status bar shows resolution and pixel format
- [ ] Pixel inspector shows coordinates on hover

### 5. Stream tools
- [ ] Zoom/pan with scroll wheel and drag
- [ ] Histogram toggle works
- [ ] ROI selection draws rectangle
- [ ] Line profile draws line and shows plot

### 6. Recording
- [ ] Click record button → icon pulses
- [ ] Tooltip shows frame count incrementing
- [ ] Click stop → recording stops
- [ ] File created in `~/.genicam-studio/recordings/`

### 7. Stop & disconnect
- [ ] Click Stop → frames stop, FPS goes to 0
- [ ] Disconnect device → feature tree clears
- [ ] Reconnect → features reload

### 8. Service crash recovery
- [ ] While connected, kill Terminal 2 (Ctrl+C)
- [ ] Studio shows reconnecting banner
- [ ] Restart service in Terminal 2
- [ ] Studio reconnects automatically (or shows error after max attempts)
