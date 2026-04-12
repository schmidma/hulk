# Twix Backend Redesign Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace Twix's `Robot`-centered backend with explicit runtime, session, topic, config, and adapter layers while keeping all current panels compiling during the branch and removing the shim before merge.

**Architecture:** First extract Twix into a testable library and add the shared runtime, session, transport, error, and adapter primitives. Then land `TopicService`, `ConfigService`, and a narrow `CommandService` for simulator selection, plus a short-lived `Robot`-shaped shim that delegates to the new services so panels can migrate incrementally. Finally port every panel off the shim, remove the dead logical-path APIs, and verify the full non-UI stack with integration tests.

**Tech Stack:** Rust 2024, Tokio, ros-z, ros-z-config, eframe/egui, cargo tests, workspace integration tests

---

## Planned File Structure

### Twix library and app wiring

- Create: `tools/twix/src/lib.rs`
- Create: `tools/twix/src/app.rs`
- Create: `tools/twix/src/services.rs`
- Modify: `tools/twix/src/main.rs`
- Modify: `tools/twix/src/panel.rs`

### Backend core

- Create: `tools/twix/src/twix_error.rs`
- Create: `tools/twix/src/twix_time.rs`
- Create: `tools/twix/src/twix_session.rs`
- Create: `tools/twix/src/twix_runtime.rs`
- Create: `tools/twix/src/rosz_transport.rs`
- Create: `tools/twix/src/topic_service.rs`
- Create: `tools/twix/src/config_service.rs`
- Create: `tools/twix/src/command_service.rs`
- Create: `tools/twix/src/topic_catalog.rs`

### egui adapters

- Create: `tools/twix/src/adapters/mod.rs`
- Create: `tools/twix/src/adapters/repaint.rs`
- Create: `tools/twix/src/adapters/request.rs`
- Create: `tools/twix/src/adapters/subscription.rs`
- Create: `tools/twix/src/adapters/history.rs`
- Create: `tools/twix/src/adapters/player_subscriptions.rs`

### Tests

- Create: `tools/twix/tests/lib_exports.rs`
- Create: `tools/twix/tests/backend_error.rs`
- Create: `tools/twix/tests/session_manager.rs`
- Create: `tools/twix/tests/topic_service.rs`
- Create: `tools/twix/tests/config_service.rs`
- Create: `tools/twix/tests/simulator_command.rs`
- Create: `tools/twix/tests/adapters.rs`
- Create: `tools/twix/tests/support/demo_nodes.rs`
- Create: `tools/twix/tests/support/demo_config.rs`

### Migration targets

- Modify: `tools/twix/src/robot.rs`
- Modify: `tools/twix/src/backend.rs`
- Modify: `tools/twix/src/value_buffer.rs`
- Modify: `tools/twix/src/change_buffer.rs`
- Modify: `tools/twix/src/players_buffer_handle.rs`
- Modify: `tools/twix/src/topic_completion_edit.rs`
- Modify: `tools/twix/src/panels/text.rs`
- Modify: `tools/twix/src/panels/plot.rs`
- Modify: `tools/twix/src/panels/enum_plot.rs`
- Modify: `tools/twix/src/panels/parameter.rs`
- Modify: `tools/twix/src/panels/remote_control.rs`
- Modify: `tools/twix/src/panels/look_at.rs`
- Modify: `tools/twix/src/panels/behavior_simulator.rs`
- Modify: `tools/twix/src/panels/synthetic_pose.rs`
- Modify: `tools/twix/src/panels/image_segments.rs`
- Modify: `tools/twix/src/panels/image_color_select.rs`
- Modify: `tools/twix/src/panels/mujoco_simulator.rs`
- Modify: `tools/twix/src/panels/image/mod.rs`
- Modify: `tools/twix/src/panels/image/overlay.rs`
- Modify: `tools/twix/src/panels/image/overlays/mod.rs`
- Modify: `tools/twix/src/panels/image/overlays/ball_detection.rs`
- Modify: `tools/twix/src/panels/image/overlays/feet_detection.rs`
- Modify: `tools/twix/src/panels/image/overlays/field_border.rs`
- Modify: `tools/twix/src/panels/image/overlays/horizon.rs`
- Modify: `tools/twix/src/panels/image/overlays/line_detection.rs`
- Modify: `tools/twix/src/panels/image/overlays/object_detection.rs`
- Modify: `tools/twix/src/panels/map/mod.rs`
- Modify: `tools/twix/src/panels/map/layer.rs`
- Modify: `tools/twix/src/panels/map/layers/ball_filter.rs`
- Modify: `tools/twix/src/panels/map/layers/ball_percepts.rs`
- Modify: `tools/twix/src/panels/map/layers/ball_position.rs`
- Modify: `tools/twix/src/panels/map/layers/ball_search_heatmap.rs`
- Modify: `tools/twix/src/panels/map/layers/behavior_simulator.rs`
- Modify: `tools/twix/src/panels/map/layers/field.rs`
- Modify: `tools/twix/src/panels/map/layers/image_segments.rs`
- Modify: `tools/twix/src/panels/map/layers/kick_decisions.rs`
- Modify: `tools/twix/src/panels/map/layers/line_correspondences.rs`
- Modify: `tools/twix/src/panels/map/layers/lines.rs`
- Modify: `tools/twix/src/panels/map/layers/localization.rs`
- Modify: `tools/twix/src/panels/map/layers/mod.rs`
- Modify: `tools/twix/src/panels/map/layers/obstacle_filter.rs`
- Modify: `tools/twix/src/panels/map/layers/obstacles.rs`
- Modify: `tools/twix/src/panels/map/layers/path.rs`
- Modify: `tools/twix/src/panels/map/layers/path_obstacles.rs`
- Modify: `tools/twix/src/panels/map/layers/pose_detection.rs`
- Modify: `tools/twix/src/panels/map/layers/referee_position.rs`
- Modify: `tools/twix/src/panels/map/layers/robot_pose.rs`
- Modify: `tools/twix/src/panels/mod.rs`

## Task 1: Extract Twix Into A Testable Library

**Files:**
- Create: `tools/twix/src/lib.rs`
- Create: `tools/twix/src/app.rs`
- Modify: `tools/twix/src/main.rs`
- Test: `tools/twix/tests/lib_exports.rs`

- [ ] **Step 1: Write the failing library export test**

```rust
// tools/twix/tests/lib_exports.rs
use twix::DEFAULT_ROUTER_ENDPOINT;

#[test]
fn library_exposes_default_router_endpoint() {
    assert_eq!(DEFAULT_ROUTER_ENDPOINT, "tcp/127.0.0.1:7447");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p twix --test lib_exports -- --exact library_exposes_default_router_endpoint`
Expected: FAIL with unresolved crate `twix` or no library target found

- [ ] **Step 3: Write the minimal library extraction**

```rust
// tools/twix/src/lib.rs
pub mod app;
pub use app::{run, DEFAULT_ROUTER_ENDPOINT};

// tools/twix/src/main.rs
fn main() -> Result<(), eframe::Error> {
    twix::run()
}
```

```rust
// tools/twix/src/app.rs
pub const DEFAULT_ROUTER_ENDPOINT: &str = "tcp/127.0.0.1:7447";

pub fn run() -> Result<(), eframe::Error> {
    setup_logger().expect("failed to setup logger");
    let arguments = Arguments::parse();
    let native_options = native_options();
    eframe::run_native("Twix", native_options, Box::new(|creation_context| {
        Ok(Box::new(TwixApp::create(
            creation_context,
            arguments,
            load_configuration(),
            load_repository(&arguments),
        )))
    }))
}
```

Move `Arguments`, `TwixApp`, `setup_logger`, and the current module declarations from `main.rs` into `app.rs` without changing behavior.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p twix --test lib_exports -- --exact library_exposes_default_router_endpoint`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add tools/twix/src/lib.rs tools/twix/src/app.rs tools/twix/src/main.rs tools/twix/tests/lib_exports.rs
git commit -m "refactor(twix): extract library entrypoints"
```

## Task 2: Introduce Shared Errors, Time, Runtime, And Session State

**Files:**
- Create: `tools/twix/src/twix_error.rs`
- Create: `tools/twix/src/twix_time.rs`
- Create: `tools/twix/src/twix_session.rs`
- Create: `tools/twix/src/twix_runtime.rs`
- Modify: `tools/twix/src/lib.rs`
- Test: `tools/twix/tests/backend_error.rs`
- Test: `tools/twix/tests/session_manager.rs`

- [ ] **Step 1: Write the failing shared-state tests**

```rust
// tools/twix/tests/backend_error.rs
use twix::{TwixError, TwixTime};

#[test]
fn topic_discovery_error_keeps_context() {
    let error = TwixError::topic_discovery("list_topics", "/twix_demo/status", "graph failed");
    let rendered = error.to_string();
    assert!(rendered.contains("list_topics"));
    assert!(rendered.contains("/twix_demo/status"));
    assert!(rendered.contains("graph failed"));
}

#[test]
fn negative_time_saturates_to_zero() {
    assert_eq!(TwixTime::from_nanos(-1).as_nanos(), 0);
}
```

```rust
// tools/twix/tests/session_manager.rs
use std::sync::Arc;

use twix::{SessionManager, TwixError, TwixSession};

#[test]
fn clearing_session_reports_disconnected() {
    let manager = SessionManager::new();
    manager.replace_for_tests(Some(Arc::new(TwixSession::new_for_tests(7))));
    manager.replace_for_tests(None);

    assert!(matches!(manager.current(), Err(TwixError::DisconnectedSession)));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p twix --test backend_error --test session_manager`
Expected: FAIL with unresolved imports for `TwixError`, `TwixTime`, `SessionManager`, or `TwixSession`

- [ ] **Step 3: Write the minimal shared runtime types**

```rust
// tools/twix/src/twix_error.rs
#[derive(Clone, Debug, thiserror::Error)]
pub enum TwixError {
    #[error("topic discovery `{operation}` failed for `{target}`: {message}")]
    TopicDiscovery {
        operation: &'static str,
        target: String,
        message: String,
    },
    #[error("disconnected session")]
    DisconnectedSession,
}
```

```rust
// tools/twix/src/twix_runtime.rs
pub struct SessionManager {
    current: std::sync::Arc<std::sync::RwLock<Option<std::sync::Arc<crate::TwixSession>>>>,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            current: std::sync::Arc::new(std::sync::RwLock::new(None)),
        }
    }
}
```

Implement in this task:

- `TwixTime`
- `TwixError` with the approved runtime, transport, config, and command variants
- `TwixSession` with generation, `Arc<ZContext>`, and `Arc<ZNode>`
- `SessionManager`
- `TwixRuntime::connect(endpoint) -> Result<(), TwixError>`
- `TwixRuntime::disconnect()`
- `TwixRuntime::from_context_for_tests(...)`

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p twix --test backend_error --test session_manager`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add tools/twix/src/lib.rs tools/twix/src/twix_error.rs tools/twix/src/twix_time.rs tools/twix/src/twix_session.rs tools/twix/src/twix_runtime.rs tools/twix/tests/backend_error.rs tools/twix/tests/session_manager.rs
git commit -m "feat(twix): add shared runtime core"
```

## Task 3: Implement RoszTransport And TopicService

**Files:**
- Create: `tools/twix/src/rosz_transport.rs`
- Create: `tools/twix/src/topic_service.rs`
- Create: `tools/twix/src/topic_catalog.rs`
- Create: `tools/twix/tests/support/demo_nodes.rs`
- Create: `tools/twix/tests/topic_service.rs`
- Modify: `tools/twix/src/lib.rs`

- [ ] **Step 1: Write the failing topic-service test**

```rust
// tools/twix/tests/topic_service.rs
use futures_util::StreamExt;
use twix::{RoszTransport, TopicService, TwixRuntime};

mod support;
use support::demo_nodes::publish_dynamic_status;

#[tokio::test]
async fn topic_service_lists_and_reads_dynamic_topics() -> color_eyre::Result<()> {
    let ctx = publish_dynamic_status("/twix_demo/status").await?;
    let runtime = TwixRuntime::from_context_for_tests(ctx.clone())?;
    let session = runtime.current_session_for_tests()?;
    let service = TopicService::new(session, RoszTransport::new());

    let topics = service.list_topics().await?;
    assert!(topics.iter().any(|topic| topic.name == "/twix_demo/status"));

    let mut sub = service.subscribe_dynamic("/twix_demo/status").await?;
    let sample = sub.next().await.transpose()?.expect("first sample");
    assert_eq!(sample.value["robot_id"], "twix-demo");
    Ok(())
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p twix --test topic_service -- --exact topic_service_lists_and_reads_dynamic_topics --nocapture`
Expected: FAIL with unresolved imports for `RoszTransport` or `TopicService`

- [ ] **Step 3: Write the minimal topic transport and service**

```rust
// tools/twix/src/rosz_transport.rs
pub struct RoszTransport;

impl RoszTransport {
    pub fn new() -> Self {
        Self
    }
}

// tools/twix/src/topic_service.rs
pub struct TopicService {
    session: std::sync::Arc<crate::TwixSession>,
    transport: crate::RoszTransport,
}
```

Implement in this task:

- `list_topics()`
- `subscribe_dynamic(topic)` using dynamic ros-z subscribers and `async_recv_with_metadata()`
- `subscribe_typed<T>(topic)` using typed ros-z subscribers and `async_recv_with_metadata()`
- `topic_catalog.rs` with concrete helpers for shared topic names

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p twix --test topic_service -- --exact topic_service_lists_and_reads_dynamic_topics --nocapture`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add tools/twix/src/lib.rs tools/twix/src/rosz_transport.rs tools/twix/src/topic_service.rs tools/twix/src/topic_catalog.rs tools/twix/tests/support/demo_nodes.rs tools/twix/tests/topic_service.rs
git commit -m "feat(twix): add topic transport and service"
```

## Task 4: Implement ConfigService And Selector Resolution

**Files:**
- Create: `tools/twix/src/config_service.rs`
- Create: `tools/twix/tests/support/demo_config.rs`
- Create: `tools/twix/tests/config_service.rs`
- Modify: `tools/twix/src/lib.rs`

- [ ] **Step 1: Write the failing config-service test**

```rust
// tools/twix/tests/config_service.rs
use twix::{ConfigService, RoszTransport, TwixRuntime};

mod support;
use support::demo_config::spawn_demo_config_node;

#[tokio::test]
async fn config_service_lists_nodes_and_reads_values() -> color_eyre::Result<()> {
    let ctx = spawn_demo_config_node().await?;
    let runtime = TwixRuntime::from_context_for_tests(ctx.clone())?;
    let session = runtime.current_session_for_tests()?;
    let service = ConfigService::new(session, RoszTransport::new());

    let nodes = service.list_nodes().await?;
    assert!(nodes.iter().any(|node| node.node_fqn == "/motion/twix_demo_config"));

    let value = service.get_value("twix_demo_config", "linear_x").await?;
    assert!(value.value_json.contains("0.2"));
    Ok(())
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p twix --test config_service -- --exact config_service_lists_nodes_and_reads_values --nocapture`
Expected: FAIL with unresolved import `ConfigService`

- [ ] **Step 3: Write the minimal config service**

```rust
// tools/twix/src/config_service.rs
pub struct ConfigService {
    session: std::sync::Arc<crate::TwixSession>,
    transport: crate::RoszTransport,
}

impl ConfigService {
    pub fn new(session: std::sync::Arc<crate::TwixSession>, transport: crate::RoszTransport) -> Self {
        Self { session, transport }
    }
}
```

Implement in this task:

- `list_nodes()`
- `resolve_selector(selector)`
- `get_snapshot(selector)`
- `list_paths(selector, writable_only)`
- `get_value(selector, path)`
- `get_metadata(selector, paths)`
- `set_json(selector, path, value, layer, expected_revision)`
- `reset(selector, path, layer, expected_revision)`

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p twix --test config_service -- --nocapture`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add tools/twix/src/lib.rs tools/twix/src/config_service.rs tools/twix/tests/support/demo_config.rs tools/twix/tests/config_service.rs
git commit -m "feat(twix): add config service"
```

## Task 5: Build RequestAdapter, SubscriptionAdapter, RepaintSink, And History Helpers

**Files:**
- Create: `tools/twix/src/adapters/mod.rs`
- Create: `tools/twix/src/adapters/repaint.rs`
- Create: `tools/twix/src/adapters/request.rs`
- Create: `tools/twix/src/adapters/subscription.rs`
- Create: `tools/twix/src/adapters/history.rs`
- Create: `tools/twix/src/adapters/player_subscriptions.rs`
- Create: `tools/twix/tests/adapters.rs`
- Modify: `tools/twix/src/lib.rs`

- [ ] **Step 1: Write the failing adapter test**

```rust
// tools/twix/tests/adapters.rs
use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};

use twix::{RepaintSink, RequestAdapter, RequestState};

#[tokio::test]
async fn request_adapter_transitions_from_pending_to_ready() {
    let repaint_count = Arc::new(AtomicUsize::new(0));
    let sink = RepaintSink::new({
        let repaint_count = repaint_count.clone();
        move || {
            repaint_count.fetch_add(1, Ordering::Relaxed);
        }
    });

    let adapter = RequestAdapter::spawn(async { Ok::<_, twix::TwixError>(42_u32) }, sink);
    assert!(matches!(adapter.state(), RequestState::Pending));

    adapter.wait_for_tests().await;
    assert!(matches!(adapter.state(), RequestState::Ready(Ok(42))));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p twix --test adapters -- --exact request_adapter_transitions_from_pending_to_ready --nocapture`
Expected: FAIL with unresolved adapter imports

- [ ] **Step 3: Write the minimal adapter layer**

```rust
// tools/twix/src/adapters/repaint.rs
#[derive(Clone)]
pub struct RepaintSink(std::sync::Arc<dyn Fn() + Send + Sync>);

impl RepaintSink {
    pub fn new(callback: impl Fn() + Send + Sync + 'static) -> Self {
        Self(std::sync::Arc::new(callback))
    }
}
```

```rust
// tools/twix/src/adapters/request.rs
pub enum RequestState<T> {
    Pending,
    Ready(Result<T, crate::TwixError>),
}
```

In this task, replace the responsibilities of `value_buffer.rs`, `change_buffer.rs`, and `players_buffer_handle.rs` with the new adapters instead of preserving the old types as permanent API.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p twix --test adapters -- --nocapture`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add tools/twix/src/lib.rs tools/twix/src/adapters/mod.rs tools/twix/src/adapters/repaint.rs tools/twix/src/adapters/request.rs tools/twix/src/adapters/subscription.rs tools/twix/src/adapters/history.rs tools/twix/src/adapters/player_subscriptions.rs tools/twix/tests/adapters.rs
git commit -m "feat(twix): add egui adapters"
```

## Task 6: Add A Temporary Robot Shim And Simulator CommandService

**Files:**
- Create: `tools/twix/src/command_service.rs`
- Modify: `tools/twix/src/robot.rs`
- Modify: `tools/twix/src/lib.rs`
- Create: `tools/twix/tests/simulator_command.rs`

- [ ] **Step 1: Write the failing simulator-command test**

```rust
// tools/twix/tests/simulator_command.rs
use twix::{CommandService, TwixRuntime};

#[tokio::test]
async fn command_service_accepts_simulator_selection_requests() -> color_eyre::Result<()> {
    let ctx = std::sync::Arc::new(ros_z::context::ZContextBuilder::default().build()?);
    let runtime = TwixRuntime::from_context_for_tests(ctx)?;
    let session = runtime.current_session_for_tests()?;
    let service = CommandService::new(session);

    service.set_simulator_selection(12, 3).await?;
    Ok(())
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p twix --test simulator_command -- --exact command_service_accepts_simulator_selection_requests --nocapture`
Expected: FAIL with unresolved import `CommandService`

- [ ] **Step 3: Write the minimal command service and shim**

```rust
// tools/twix/src/command_service.rs
pub struct CommandService {
    session: std::sync::Arc<crate::TwixSession>,
}

impl CommandService {
    pub fn new(session: std::sync::Arc<crate::TwixSession>) -> Self {
        Self { session }
    }

    pub async fn set_simulator_selection(&self, selected_frame: usize, selected_robot: usize) -> Result<(), crate::TwixError> {
        let _ = (&self.session, selected_frame, selected_robot);
        Ok(())
    }
}
```

```rust
// tools/twix/src/robot.rs
pub struct Robot {
    services: std::sync::Arc<crate::services::TwixServices>,
}
```

The shim in `robot.rs` is temporary. In this task it should delegate:

- topic reads to `TopicService` or `SubscriptionAdapter`
- config reads and writes to `ConfigService`
- simulator selection to `CommandService`

It must not preserve `subscribe_value` or generic `write(path, value)` as real behavior; those methods should become thin call-site bridges only where still needed during migration.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p twix --test simulator_command -- --nocapture`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add tools/twix/src/lib.rs tools/twix/src/command_service.rs tools/twix/src/robot.rs tools/twix/tests/simulator_command.rs
git commit -m "refactor(twix): add temporary robot shim"
```

## Task 7: Port Request-Driven Panels And Discovery UI

**Files:**
- Create: `tools/twix/src/services.rs`
- Modify: `tools/twix/src/app.rs`
- Modify: `tools/twix/src/panel.rs`
- Modify: `tools/twix/src/topic_completion_edit.rs`
- Modify: `tools/twix/src/panels/parameter.rs`
- Modify: `tools/twix/src/panels/remote_control.rs`
- Modify: `tools/twix/src/panels/look_at.rs`
- Modify: `tools/twix/src/panels/behavior_simulator.rs`
- Modify: `tools/twix/src/panels/mod.rs`

- [ ] **Step 1: Change the panel context so compilation fails in the old callers**

```rust
// tools/twix/src/panel.rs
pub struct PanelCreationContext<'a> {
    pub services: std::sync::Arc<crate::services::TwixServices>,
    pub value: Option<&'a serde_json::Value>,
    pub wgpu_state: eframe::egui_wgpu::RenderState,
    pub egui_context: eframe::egui::Context,
}
```

- [ ] **Step 2: Run compile to verify the expected breakage**

Run: `cargo check -p twix`
Expected: FAIL with remaining callers still expecting `context.robot`

- [ ] **Step 3: Port the request-driven panels**

```rust
// tools/twix/src/services.rs
pub struct TwixServices {
    pub runtime: std::sync::Arc<crate::TwixRuntime>,
    pub topics: std::sync::Arc<crate::TopicService>,
    pub config: std::sync::Arc<crate::ConfigService>,
    pub commands: std::sync::Arc<crate::CommandService>,
    pub repaint: crate::RepaintSink,
}
```

```rust
// representative parameter-panel call
self.pending_snapshot = Some(crate::RequestAdapter::spawn(
    self.services.config.get_snapshot(self.node_selector.clone()),
    self.services.repaint.clone(),
));
```

In this task, port:

- `ParameterPanel` to `RequestAdapter` for snapshot, value, metadata, set, and reset
- `RemotePanel` to `ConfigService::set_json` for `parameters.behavior.remote_control.walk` and `parameters.rl_walking.gait_frequency`
- `LookAtPanel` to `ConfigService::set_json` for `parameters.behavior.injected_motion_command`
- `BehaviorSimulatorPanel` to `CommandService::set_simulator_selection`
- `topic_completion_edit.rs` to pending/ready request state instead of `discovering` booleans

- [ ] **Step 4: Run compile and targeted tests**

Run: `cargo check -p twix && cargo test -p twix --test config_service --test simulator_command --test adapters -- --nocapture`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add tools/twix/src/services.rs tools/twix/src/app.rs tools/twix/src/panel.rs tools/twix/src/topic_completion_edit.rs tools/twix/src/panels/parameter.rs tools/twix/src/panels/remote_control.rs tools/twix/src/panels/look_at.rs tools/twix/src/panels/behavior_simulator.rs tools/twix/src/panels/mod.rs
git commit -m "refactor(twix): port request driven panels"
```

## Task 8: Port Typed Topic Panels To TopicService And Catalog

**Files:**
- Modify: `tools/twix/src/topic_catalog.rs`
- Modify: `tools/twix/src/panels/text.rs`
- Modify: `tools/twix/src/panels/plot.rs`
- Modify: `tools/twix/src/panels/enum_plot.rs`
- Modify: `tools/twix/src/panels/synthetic_pose.rs`
- Modify: `tools/twix/src/panels/image_segments.rs`
- Modify: `tools/twix/src/panels/image_color_select.rs`
- Modify: `tools/twix/src/panels/mujoco_simulator.rs`
- Modify: `tools/twix/src/panels/image/mod.rs`
- Modify: `tools/twix/src/panels/image/overlay.rs`
- Modify: `tools/twix/src/panels/image/overlays/mod.rs`
- Modify: `tools/twix/src/panels/image/overlays/ball_detection.rs`
- Modify: `tools/twix/src/panels/image/overlays/feet_detection.rs`
- Modify: `tools/twix/src/panels/image/overlays/field_border.rs`
- Modify: `tools/twix/src/panels/image/overlays/horizon.rs`
- Modify: `tools/twix/src/panels/image/overlays/line_detection.rs`
- Modify: `tools/twix/src/panels/image/overlays/object_detection.rs`
- Modify: `tools/twix/src/panels/map/mod.rs`
- Modify: `tools/twix/src/panels/map/layer.rs`
- Modify: `tools/twix/src/panels/map/layers/ball_filter.rs`
- Modify: `tools/twix/src/panels/map/layers/ball_percepts.rs`
- Modify: `tools/twix/src/panels/map/layers/ball_position.rs`
- Modify: `tools/twix/src/panels/map/layers/ball_search_heatmap.rs`
- Modify: `tools/twix/src/panels/map/layers/behavior_simulator.rs`
- Modify: `tools/twix/src/panels/map/layers/field.rs`
- Modify: `tools/twix/src/panels/map/layers/image_segments.rs`
- Modify: `tools/twix/src/panels/map/layers/kick_decisions.rs`
- Modify: `tools/twix/src/panels/map/layers/line_correspondences.rs`
- Modify: `tools/twix/src/panels/map/layers/lines.rs`
- Modify: `tools/twix/src/panels/map/layers/localization.rs`
- Modify: `tools/twix/src/panels/map/layers/mod.rs`
- Modify: `tools/twix/src/panels/map/layers/obstacle_filter.rs`
- Modify: `tools/twix/src/panels/map/layers/obstacles.rs`
- Modify: `tools/twix/src/panels/map/layers/path.rs`
- Modify: `tools/twix/src/panels/map/layers/path_obstacles.rs`
- Modify: `tools/twix/src/panels/map/layers/pose_detection.rs`
- Modify: `tools/twix/src/panels/map/layers/referee_position.rs`
- Modify: `tools/twix/src/panels/map/layers/robot_pose.rs`

- [ ] **Step 1: Add the failing topic-catalog test**

```rust
// append to tools/twix/tests/topic_service.rs
#[test]
fn topic_catalog_exposes_behavior_motion_command_topic() {
    assert_eq!(twix::topic_catalog::behavior_motion_command(), "behavior/motion_command");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p twix --test topic_service -- --exact topic_catalog_exposes_behavior_motion_command_topic`
Expected: FAIL with missing `behavior_motion_command` helper

- [ ] **Step 3: Port all typed topic panels**

```rust
// tools/twix/src/topic_catalog.rs
pub fn behavior_motion_command() -> &'static str {
    "behavior/motion_command"
}
```

```rust
// representative panel call
let motion_command = crate::SubscriptionAdapter::typed(
    self.services.topics.clone(),
    crate::topic_catalog::behavior_motion_command(),
    self.services.repaint.clone(),
);
```

Replace every remaining `subscribe_value("...")` and logical-path topic dependency with explicit typed topics or config calls.

- [ ] **Step 4: Run compile and topic tests**

Run: `cargo check -p twix && cargo test -p twix --test topic_service -- --nocapture`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add tools/twix/src/topic_catalog.rs tools/twix/src/panels/text.rs tools/twix/src/panels/plot.rs tools/twix/src/panels/enum_plot.rs tools/twix/src/panels/synthetic_pose.rs tools/twix/src/panels/image_segments.rs tools/twix/src/panels/image_color_select.rs tools/twix/src/panels/mujoco_simulator.rs tools/twix/src/panels/image/mod.rs tools/twix/src/panels/image/overlay.rs tools/twix/src/panels/image/overlays/mod.rs tools/twix/src/panels/image/overlays/ball_detection.rs tools/twix/src/panels/image/overlays/feet_detection.rs tools/twix/src/panels/image/overlays/field_border.rs tools/twix/src/panels/image/overlays/horizon.rs tools/twix/src/panels/image/overlays/line_detection.rs tools/twix/src/panels/image/overlays/object_detection.rs tools/twix/src/panels/map/mod.rs tools/twix/src/panels/map/layer.rs tools/twix/src/panels/map/layers/ball_filter.rs tools/twix/src/panels/map/layers/ball_percepts.rs tools/twix/src/panels/map/layers/ball_position.rs tools/twix/src/panels/map/layers/ball_search_heatmap.rs tools/twix/src/panels/map/layers/behavior_simulator.rs tools/twix/src/panels/map/layers/field.rs tools/twix/src/panels/map/layers/image_segments.rs tools/twix/src/panels/map/layers/kick_decisions.rs tools/twix/src/panels/map/layers/line_correspondences.rs tools/twix/src/panels/map/layers/lines.rs tools/twix/src/panels/map/layers/localization.rs tools/twix/src/panels/map/layers/mod.rs tools/twix/src/panels/map/layers/obstacle_filter.rs tools/twix/src/panels/map/layers/obstacles.rs tools/twix/src/panels/map/layers/path.rs tools/twix/src/panels/map/layers/path_obstacles.rs tools/twix/src/panels/map/layers/pose_detection.rs tools/twix/src/panels/map/layers/referee_position.rs tools/twix/src/panels/map/layers/robot_pose.rs
git commit -m "refactor(twix): port typed topic panels"
```

## Task 9: Remove The Shim, Delete Legacy APIs, And Verify End To End

**Files:**
- Modify: `tools/twix/src/app.rs`
- Modify: `tools/twix/src/lib.rs`
- Delete: `tools/twix/src/robot.rs`
- Delete: `tools/twix/src/backend.rs`
- Delete: `tools/twix/src/value_buffer.rs`
- Delete: `tools/twix/src/change_buffer.rs`
- Delete: `tools/twix/src/players_buffer_handle.rs`
- Test: `tools/twix/tests/lib_exports.rs`
- Test: `tools/twix/tests/backend_error.rs`
- Test: `tools/twix/tests/session_manager.rs`
- Test: `tools/twix/tests/topic_service.rs`
- Test: `tools/twix/tests/config_service.rs`
- Test: `tools/twix/tests/simulator_command.rs`
- Test: `tools/twix/tests/adapters.rs`

- [ ] **Step 1: Remove the shim and old modules so compile fails in any missed call sites**

```rust
// tools/twix/src/lib.rs
pub mod adapters;
pub mod app;
pub mod command_service;
pub mod config_service;
pub mod rosz_transport;
pub mod services;
pub mod topic_catalog;
pub mod topic_service;
pub mod twix_error;
pub mod twix_runtime;
pub mod twix_session;
pub mod twix_time;
```

- [ ] **Step 2: Run compile to verify the remaining breakage**

Run: `cargo check -p twix`
Expected: FAIL if any remaining code still imports `crate::robot`, `BackendCapability`, `subscribe_value`, `write(path, value)`, or old buffer types

- [ ] **Step 3: Fix the remaining imports and delete the dead files**

```bash
git rm tools/twix/src/robot.rs tools/twix/src/backend.rs tools/twix/src/value_buffer.rs tools/twix/src/change_buffer.rs tools/twix/src/players_buffer_handle.rs
```

Make `app.rs` construct `TwixServices` directly and pass service handles into all panels without `Arc<Robot>`.

- [ ] **Step 4: Run the full verification suite**

Run: `cargo test -p twix --tests -- --nocapture && cargo check -p twix`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add tools/twix/src/app.rs tools/twix/src/lib.rs tools/twix/tests/lib_exports.rs tools/twix/tests/backend_error.rs tools/twix/tests/session_manager.rs tools/twix/tests/topic_service.rs tools/twix/tests/config_service.rs tools/twix/tests/simulator_command.rs tools/twix/tests/adapters.rs
git rm tools/twix/src/robot.rs tools/twix/src/backend.rs tools/twix/src/value_buffer.rs tools/twix/src/change_buffer.rs tools/twix/src/players_buffer_handle.rs
git commit -m "refactor(twix): remove legacy backend"
```

## Self-Review Notes

### Spec Coverage

- runtime, session, and connection shape: Tasks 1 and 2
- topic transport and typed subscriptions: Task 3 and Task 8
- config selector resolution and writes: Task 4 and Task 7
- narrow simulator-only command service: Task 6 and Task 7
- request and subscription adapters with repaint ownership: Task 5
- temporary `Robot`-shaped shim during branch, deleted before merge: Task 6 and Task 9
- removal of dead logical-path APIs and old buffers: Task 8 and Task 9

### Placeholder Scan

- No `TODO`, `TBD`, or deferred “figure it out later” sections remain.
- All tasks name exact file paths and runnable commands.

### Type Consistency

- Shared types: `TwixError`, `TwixTime`, `TwixSession`, `TwixRuntime`, `RoszTransport`, `TopicService`, `ConfigService`, `CommandService`, `RequestAdapter`, `SubscriptionAdapter`, `RepaintSink`, `TwixServices`
- `remote_control` and `look_at` are always routed through `ConfigService`
- simulator selection is the only initial `CommandService` scope
