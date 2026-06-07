# RuboVision Engine Layered Refactor Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace string UART commands and loosely typed function configuration with a binary UART protocol, typed configuration, unified message outputs, DebugSource Web triggering, and a clean declarative function registry.

**Architecture:** Keep the existing `Source -> Event -> TaskListener` pipeline, but move UART ownership into a shared transport, make functions return channel-neutral results, and route results through Web/UART/GPIO sinks in `after_func`. Configuration switches once to `message.yaml`, typed device/function data, numeric UART bindings, and string DebugSource bindings.

**Tech Stack:** Rust 2024, Tokio, Axum, Serde, TOML/YAML, rppal UART/GPIO, OpenCV, embedded HTML/CSS/JavaScript.

---

## File Structure

### Configuration

- Create `src/config/type.rs`: shared typed runtime structures and validation helpers.
- Modify `src/config/mod.rs`: export `r#type` and remove obsolete exports.
- Modify `src/config/settings.rs`: load and validate the new files.
- Modify `src/config/binding.rs`: numeric UART and string DebugSource bindings.
- Modify `src/config/device.rs`: camera-only device configuration.
- Modify `src/config/func.rs`: typed function entry and return-target configuration.
- Delete `config/web.yaml` and `config/func_param.toml`.
- Create `config/message.yaml` and `config/functions.toml`.
- Modify `config/device.toml` and `config/bindings.toml`.

### UART And Messages

- Create `src/message/mod.rs`: public message-layer exports.
- Create `src/message/model.rs`: `WebOutput`, `UartOutput`, `GpioOutput`.
- Create `src/message/sink.rs`: `MessageSink` and concrete sink wrappers.
- Create `src/message/router.rs`: independent multi-sink routing.
- Create `src/message/uart.rs`: the single-owner UART transport thread.
- Create `src/message/gpio.rs`: GPIO worker and test backend.
- Modify `src/source/source_uart.rs`: binary frame parser and byte receiver.
- Modify `src/device/vision/config.rs`: remove UART from `CameraDevice`.
- Modify `src/device/vision/response.rs`: remove UART/GPIO response ownership.
- Modify `src/lib.rs`: construct transports, router, and channels.

### Functions

- Modify `src/func/functions.rs`: typed parameters, pure business functions, and `declare_functions!`.
- Rewrite `src/func/tarits.rs`: registry definitions, `FunctionResult`, typed erasure, and `FromDevice`.
- Rewrite `src/func/register.rs`: build the registry from macro descriptors and typed config.
- Delete `src/func/usual.rs`.
- Modify `src/func/mod.rs`: export the new API.
- Modify `src/init/task_dispatch.rs`: return recoverable lookup errors.
- Modify `src/init/task_exec.rs`: implement `pre_func -> func -> after_func`.
- Modify `src/init/task_listen.rs` and `src/init/register.rs`: pass the router and handle errors.

### DebugSource And Web

- Replace `src/source/source_web.rs` with `src/source/source_debug.rs`.
- Modify `src/source/mod.rs`: export DebugSource.
- Modify `src/web/state.rs`: hold debug binding metadata and Event sender.
- Modify `src/web/model.rs`: debug API request/response models.
- Modify `src/web/handler.rs` and `src/web/router.rs`: debug routes.
- Modify `src/web/main.rs`: accept initialized state instead of constructing disconnected state.
- Rewrite `static/index.html`: preserve message/history functions and add task controls.

### Documentation

- Modify `README.md` and `TaskTODO.md`.
- Create `changeTask/completed.md`.

---

### Task 1: One-Time Typed Configuration Migration

**Files:**
- Create: `src/config/type.rs`
- Modify: `src/config/mod.rs`
- Modify: `src/config/settings.rs`
- Modify: `src/config/binding.rs`
- Modify: `src/config/device.rs`
- Modify: `src/config/func.rs`
- Delete: `config/web.yaml`
- Delete: `config/func_param.toml`
- Create: `config/message.yaml`
- Create: `config/functions.toml`
- Modify: `config/device.toml`
- Modify: `config/bindings.toml`
- Test: `src/config/settings.rs`

- [ ] **Step 1: Write failing configuration shape tests**

Add tests that load the repository configuration and assert:

```rust
assert!(cfg.message.web.on);
assert_eq!(cfg.message.uart.serial, "/dev/ttyV0");
assert_eq!(cfg.bindings.uart_source[0].source_key, 0x01);
assert_eq!(cfg.bindings.debug_source[0].source_key, "color");
assert_eq!(cfg.devices.list[0].device_id, "color_camera");
assert!(cfg.functions.entries.iter().any(|entry| {
    entry.function_id == "color_detect"
        && entry.returns.uart
        && entry.returns.gpio.as_deref() == Some("color")
}));
```

Add a temporary-directory test that supplies the old `func_param_config` shape and asserts deserialization fails because `functions` is missing.

Extract:

```rust
fn load_config_from(base: &str) -> Result<RuntimeConfig, config::ConfigError>;
```

`load_config()` calls `load_config_from("config")`; tests call it with a temporary directory containing the desired fixture files.

- [ ] **Step 2: Run tests and verify RED**

Run:

```bash
cargo test config::settings::tests -- --nocapture
```

Expected: compilation or assertions fail because the new configuration types and files do not exist.

- [ ] **Step 3: Implement typed configuration**

Define the core structures:

```rust
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RuntimeConfig {
    pub app: AppConfig,
    pub message: MessageConfig,
    pub bindings: BindingsConfig,
    pub devices: DevicesConfig,
    pub functions: FunctionsConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MessageConfig {
    pub web: WebConfig,
    pub uart: UartConfig,
    pub gpio: GpioConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ReturnTargets {
    pub web: bool,
    pub uart: bool,
    pub gpio: Option<String>,
}
```

Use `toml::Value` only at the registry boundary:

```rust
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FunctionEntryConfig {
    pub function_id: String,
    pub returns: ReturnTargets,
    pub params: toml::Value,
}
```

Change UART binding keys to `u8`, add `DebugBinding`, and validate duplicate keys/IDs and missing references in `RuntimeConfig::validate()`.

- [ ] **Step 4: Replace configuration files**

Use these top-level sections:

```yaml
message:
  web:
    on: true
    host: 127.0.0.1
    port: 3000
  uart:
    on: true
    serial: /dev/ttyV0
    baud: 9600
    data_bit: 8
    stop_bit: 1
    parity_bit: false
  gpio:
    on: true
    active_low: true
    run_pin: 27
    signals:
      color: 17
      qr: 22
```

Use numeric UART keys and matching debug keys:

```toml
[[bindings.uart_source]]
task_id = "uart_color_detect"
source_key = 1
device_id = "color_camera"
function_id = "color_detect"

[[bindings.debug_source]]
task_id = "debug_color_detect"
source_key = "color"
device_id = "color_camera"
function_id = "color_detect"
```

- [ ] **Step 5: Adapt current callers without completing later refactors**

Update current registration code to read `cfg.message.web`, `cfg.message.uart`, `cfg.devices`, and `cfg.functions`. It is acceptable in this commit to pass UART configuration through a temporary function parameter to existing code, but no old file or old serialized field may remain supported.

- [ ] **Step 6: Verify GREEN**

Run:

```bash
cargo fmt --check
cargo test config::settings::tests -- --nocapture
cargo check
```

Expected: all commands exit 0.

- [ ] **Step 7: Commit**

```bash
git add -A src/config config src/lib.rs src/init src/device
git commit -m "refactor: migrate to typed runtime configuration"
```

---

### Task 2: Binary UART Frame Protocol

**Files:**
- Modify: `src/source/source_uart.rs`
- Modify: `config/bindings.toml`
- Test: `src/source/source_uart.rs`

- [ ] **Step 1: Write parser tests first**

Introduce a pure parser API:

```rust
fn take_uart_commands(pending: &mut Vec<u8>) -> Vec<u8>;
```

Test:

```rust
assert_eq!(parse(&[0xAA, 0x01]), vec![]);
assert_eq!(parse(&[0xAA, 0x01, 0x55]), vec![0x01]);
assert_eq!(parse(&[0xAA, 1, 0x55, 0xAA, 2, 0x55]), vec![1, 2]);
assert_eq!(parse(&[0x00, 0x99, 0xAA, 3, 0x55]), vec![3]);
assert_eq!(parse(&[0xAA, 1, 0x00, 0xAA, 2, 0x55]), vec![2]);
```

Update the virtual UART test to send:

```rust
peer.write_all(&[0xAA, 0x01, 0x55])?;
```

- [ ] **Step 2: Run tests and verify RED**

Run:

```bash
cargo test source::source_uart::tests -- --nocapture
```

Expected: old string parser does not satisfy binary-frame tests.

- [ ] **Step 3: Implement parser and numeric dispatch**

Add:

```rust
const UART_FRAME_HEAD: u8 = 0xAA;
const UART_FRAME_TAIL: u8 = 0x55;
const UART_FRAME_LEN: usize = 3;
const UART_PENDING_MAX_LEN: usize = 64;
```

Change pending to `Vec<u8>`, `binding_map` to `HashMap<u8, UartBinding>`, and `dispatch_command` to `u8`. Handle `0x04` and `0x05` with reserved-command logging before binding lookup.

- [ ] **Step 4: Verify GREEN**

Run:

```bash
cargo fmt --check
cargo test source::source_uart::tests -- --nocapture
cargo check
```

Expected: parser and virtual serial tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/source/source_uart.rs config/bindings.toml
git commit -m "feat: parse fixed binary UART command frames"
```

---

### Task 3: Message Layer And Shared UART/GPIO Infrastructure

**Files:**
- Create: `src/message/mod.rs`
- Create: `src/message/model.rs`
- Create: `src/message/sink.rs`
- Create: `src/message/router.rs`
- Create: `src/message/uart.rs`
- Create: `src/message/gpio.rs`
- Modify: `src/lib.rs`
- Modify: `src/source/source_uart.rs`
- Modify: `src/device/vision/config.rs`
- Modify: `src/device/vision/response.rs`
- Modify: `src/device/register.rs`
- Test: new message module tests and existing UART tests

- [ ] **Step 1: Write failing router and transport tests**

Test that:

- Web-only targets send only a `WebMessage`.
- UART-only targets append one newline and do not send Web.
- A failed UART sink does not prevent Web delivery.
- GPIO start followed by finish produces low/high state transitions in the fake backend.
- A virtual UART transport can receive an input frame and write a response through one owned serial handle.

- [ ] **Step 2: Run tests and verify RED**

Run:

```bash
cargo test message:: -- --nocapture
```

Expected: module and types do not yet exist.

- [ ] **Step 3: Add message models and sink contract**

Use:

```rust
pub trait MessageSink<M>: Send + Sync {
    fn send(
        &self,
        message: M,
    ) -> impl std::future::Future<Output = anyhow::Result<()>> + Send;
}

#[derive(Debug, Clone)]
pub struct UartOutput {
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GpioOutput {
    TaskStarted(String),
    TaskFinished(String),
    Reset,
}
```

- [ ] **Step 4: Implement single-owner UART transport**

`UartTransport::start` opens one `rppal::uart::Uart` inside a blocking thread and returns:

```rust
pub struct UartChannels {
    pub incoming: tokio::sync::mpsc::Receiver<Vec<u8>>,
    pub outgoing: tokio::sync::mpsc::Sender<Vec<u8>>,
}
```

The owner thread drains outgoing writes, then performs bounded reads and `blocking_send`s incoming bytes. `UartSource` consumes `incoming`; `UartSink` owns a clone of `outgoing`.

- [ ] **Step 5: Implement GPIO worker**

Create a private `PinBackend` trait with production `RppalPinBackend` and fake test implementation. The GPIO worker owns pins and maps named signals from `message.gpio.signals`.

- [ ] **Step 6: Remove UART/GPIO from Camera**

Make:

```rust
pub struct CameraDevice {
    pub path: String,
}
```

Delete response configuration construction from vision functions. Keep legacy send helpers only until Task 4 callers are migrated, marking them private if still temporarily needed.

- [ ] **Step 7: Verify GREEN**

Run:

```bash
cargo fmt --check
cargo test message:: -- --nocapture
cargo test source::source_uart::tests -- --nocapture
cargo check
```

Expected: all commands exit 0.

- [ ] **Step 8: Commit**

```bash
git add src/message src/lib.rs src/source/source_uart.rs src/device
git commit -m "refactor: add unified message transports and sinks"
```

---

### Task 4: Declarative Functions And Pre/Func/After Lifecycle

**Files:**
- Modify: `src/func/functions.rs`
- Modify: `src/func/tarits.rs`
- Modify: `src/func/register.rs`
- Modify: `src/func/mod.rs`
- Delete: `src/func/usual.rs`
- Modify: `src/init/task_dispatch.rs`
- Modify: `src/init/task_exec.rs`
- Modify: `src/init/task_listen.rs`
- Modify: `src/init/register.rs`
- Modify: `src/device/vision/config.rs`
- Modify: `src/device/vision/tests.rs`
- Test: function and executor modules

- [ ] **Step 1: Write failing registry and lifecycle tests**

Test:

```rust
assert!(FUNCTION_DESCRIPTORS.iter().any(|item| item.id == "color_detect"));
assert!(register_func(config_with_unknown_function()).is_err());
assert!(register_func(config_with_invalid_color_params()).is_err());
```

Use fake sinks to assert lifecycle order:

```text
GPIO TaskStarted("color")
function runs
Web/UART result routed
GPIO TaskFinished("color")
```

Repeat with a failing function and assert finish/reset still occurs.

- [ ] **Step 2: Run tests and verify RED**

Run:

```bash
cargo test func:: -- --nocapture
cargo test init::task_exec::tests -- --nocapture
```

Expected: current match-based registry and `WebMessage`-returning functions fail new tests.

- [ ] **Step 3: Define typed function API**

Use:

```rust
#[derive(Debug, Clone)]
pub struct FunctionResult {
    pub text: String,
    pub value: Option<String>,
    pub image: Option<String>,
}

pub trait FromDevice: Sized {
    fn from_device(device: &Device) -> anyhow::Result<&Self>;
}
```

Store erased params as `Arc<dyn Any + Send + Sync>` and an adapter function:

```rust
pub type ErasedFunction = fn(
    &(dyn Any + Send + Sync),
    &Device,
) -> anyhow::Result<FunctionResult>;
```

- [ ] **Step 4: Implement `declare_functions!`**

The macro invocation in `functions.rs` is:

```rust
declare_functions! {
    color_detect(params: ColorDetectParams, device: CameraDevice) => color_detect,
    qr_detect(params: QrDetectParams, device: CameraDevice) => qr_detect,
    cross_detect(params: CrossDetectParams, device: CameraDevice) => cross_detect,
    debug_fun(params: DebugParams, device: NoDevice) => debug_fun,
}
```

The macro generates descriptors with:

- stable ID from `stringify!`;
- `toml::Value -> typed params` parser;
- erased adapter that downcasts params and validates device type;
- no separate manual factory `match`.

- [ ] **Step 5: Move pure business functions**

Functions must have no sender, UART, GPIO, or Web dependencies:

```rust
fn color_detect(
    params: &ColorDetectParams,
    camera: &CameraDevice,
) -> anyhow::Result<FunctionResult> {
    let result = run_color_detect(&params.to_runtime(camera))?;
    Ok(FunctionResult::value(
        format!("color_detect finished: {result}"),
        result,
    ))
}
```

Convert QR, cross, and debug similarly. Delete `into_web_message`, `send_result_to_serial`, and direct light sessions.

- [ ] **Step 6: Implement execution lifecycle**

`execute` performs:

```rust
router.pre_func(&worker.returns).await;
let outcome = spawn_blocking(move || worker.run(&device)).await;
router.after_func(&worker.func_id, &worker.returns, outcome).await;
```

`after_func` always sends GPIO completion/reset, routes successful output, and converts failures to Web error output while preserving other sink attempts.

- [ ] **Step 7: Replace panic lookups**

Change `DeviceMap::get_device` and `FuncWorkerMap::get_func` to return `Result`. Dispatcher/listener errors must become routed error messages and logs rather than process panics.

- [ ] **Step 8: Verify GREEN**

Run:

```bash
cargo fmt --check
cargo test func:: -- --nocapture
cargo test init::task_exec::tests -- --nocapture
cargo test device::vision::tests::test_color_detect_config_from_config_file
cargo check
```

Expected: all commands exit 0.

- [ ] **Step 9: Commit**

```bash
git add src/func src/init src/device src/message src/lib.rs
git commit -m "refactor: add declarative function lifecycle"
```

---

### Task 5: DebugSource And Web Trigger API

**Files:**
- Delete: `src/source/source_web.rs`
- Create: `src/source/source_debug.rs`
- Modify: `src/source/mod.rs`
- Modify: `src/init/register.rs`
- Modify: `src/web/model.rs`
- Modify: `src/web/state.rs`
- Modify: `src/web/handler.rs`
- Modify: `src/web/router.rs`
- Modify: `src/web/main.rs`
- Modify: `src/lib.rs`
- Test: source and web modules

- [ ] **Step 1: Write failing source and handler tests**

Test that:

- `"color"` creates the configured `UsualEvent`.
- unknown keys return a not-found error and send no event.
- `GET /debug/bindings` returns safe public metadata.
- `POST /debug/trigger` returns `202` for a valid key, `404` for unknown, and `503` if the Event channel is closed.

- [ ] **Step 2: Run tests and verify RED**

Run:

```bash
cargo test source::source_debug::tests -- --nocapture
cargo test web:: -- --nocapture
```

Expected: DebugSource and routes do not exist.

- [ ] **Step 3: Implement DebugSource**

Use:

```rust
#[derive(Clone)]
pub struct DebugSource {
    bindings: Arc<HashMap<String, DebugBinding>>,
    sender: Sender<Event>,
}

pub async fn trigger(&self, source_key: &str) -> Result<Event, DebugSourceError>;
pub fn bindings(&self) -> Vec<DebugBindingView>;
```

Do not spawn a listening loop; Web handlers invoke the cloned DebugSource service.

- [ ] **Step 4: Add Web API**

Models:

```rust
pub struct DebugTriggerRequest {
    pub source_key: String,
}

pub struct DebugTriggerResponse {
    pub accepted: bool,
    pub task_id: String,
}
```

Routes:

```rust
.route("/debug/bindings", get(debug_bindings))
.route("/debug/trigger", post(debug_trigger))
```

- [ ] **Step 5: Verify GREEN**

Run:

```bash
cargo fmt --check
cargo test source::source_debug::tests -- --nocapture
cargo test web:: -- --nocapture
cargo check
```

Expected: all commands exit 0.

- [ ] **Step 6: Commit**

```bash
git add src/source src/web src/init src/lib.rs
git commit -m "feat: trigger configured tasks through DebugSource"
```

---

### Task 6: Preserve And Redesign The Web Console

**Files:**
- Modify: `static/index.html`
- Test: manual browser smoke check plus Rust embedded asset test

- [ ] **Step 1: Add an embedded page contract test**

Add a test that loads `Assets::get("index.html")` and asserts the page contains:

```text
/message
/history
/debug/bindings
/debug/trigger
start-polling
clear-history
debug-task-list
```

- [ ] **Step 2: Run test and verify RED**

Run:

```bash
cargo test web_page_contains_message_history_and_debug_controls -- --nocapture
```

Expected: fails because DebugSource controls are absent.

- [ ] **Step 3: Start the visual companion and present layout options**

Use the accepted visual companion to compare two responsive dashboard layouts. Preserve the implementation requirements regardless of visual choice:

- current message and image;
- message/history polling;
- counters;
- history list and local clear;
- DebugSource task list and trigger feedback.

- [ ] **Step 4: Implement selected layout**

Keep one embedded HTML file with no Node build pipeline. Fetch debug bindings on load and render buttons with escaped text. POST JSON:

```javascript
await fetch('/debug/trigger', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ source_key: binding.source_key }),
});
```

Disable the clicked button during the request and expose result text through an `aria-live` status element.

- [ ] **Step 5: Verify page behavior**

Run:

```bash
cargo fmt --check
cargo test web_page_contains_message_history_and_debug_controls -- --nocapture
cargo check
```

Start the server only if the local UART/GPIO configuration can be disabled through environment overrides; then verify `/`, `/message`, `/history`, `/debug/bindings`, and a DebugSource trigger. Otherwise record the hardware startup limitation and verify handlers through tests.

- [ ] **Step 6: Commit**

```bash
git add static/index.html src/embed.rs
git commit -m "feat: add task controls to web console"
```

---

### Task 7: Documentation, Completion Record, And Full Regression

**Files:**
- Modify: `README.md`
- Modify: `TaskTODO.md`
- Create: `changeTask/completed.md`

- [ ] **Step 1: Update documentation**

Document:

- new configuration filenames and examples;
- binary input frames `AA 01 55` through `AA 05 55`;
- reserved behavior for `0x04` and `0x05`;
- DebugSource API;
- `pre_func -> func -> after_func`;
- how to add a function through `functions.rs` and `functions.toml`;
- Web/UART/GPIO message routing;
- hardware and ignored-test requirements.

- [ ] **Step 2: Write completion record**

Create `changeTask/completed.md` with a compact table:

```markdown
| Task | Status | Verification | Commit |
| --- | --- | --- | --- |
```

Before writing each row, run `git log --format='%H %s' --reverse fd3f52f..HEAD` and replace the descriptive text in the Commit column with the exact returned hash. Explicitly mark `toSDK.md`, UART cancellation, heartbeat business logic, and real cross detection as not implemented by design.

- [ ] **Step 3: Run fresh full verification**

Run:

```bash
cargo fmt --check
cargo check
cargo test --all-targets
git diff --check
```

Expected:

- formatting exits 0;
- check exits 0;
- all non-ignored tests pass;
- ignored hardware/GUI tests remain listed;
- no whitespace errors.

- [ ] **Step 4: Inspect final repository state**

Run:

```bash
git status --short
git log --oneline --decorate -10
```

Confirm no unrelated user changes were reverted. Include every phase commit hash in `changeTask/completed.md`.

- [ ] **Step 5: Final commit**

```bash
git add README.md TaskTODO.md changeTask src config static Cargo.toml
git commit -m "docs: record layered engine refactor completion"
```

- [ ] **Step 6: Re-run verification after the final commit**

Run:

```bash
cargo fmt --check
cargo check
cargo test --all-targets
git status --short --branch
```

Expected: all commands exit 0 and the worktree contains no uncommitted implementation changes.
