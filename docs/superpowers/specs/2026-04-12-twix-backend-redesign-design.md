# Twix Backend Redesign Design

## Goal

Redesign Twix's non-UI architecture around `ros-z` so that async execution, connection lifecycle, subscriptions, config access, and non-config control surfaces are explicit, testable, and easy to reason about.

The redesign should allow rewriting most of Twix outside the visual layer. UI behavior and layout are not design targets; panel code may be adapted as needed, but the new architecture should not be constrained by preserving the current panel-facing API.

## Scope

This spec covers the full non-UI platform of Twix:

- runtime ownership and async boundary
- connection and session lifecycle
- topic discovery and subscriptions
- config node discovery and config RPC access
- non-config control surfaces
- error typing and propagation across all backend operations
- UI-facing adapters for consuming async results from egui code
- tests for success and failure paths

This spec does not cover visual redesign, panel layout changes, or aesthetic UI behavior.

## Current Problems

Today, `tools/twix/src/robot.rs` combines too many responsibilities:

- Tokio runtime creation and ownership
- `ros-z` context and node creation
- connection state and backend generation tracking
- topic discovery
- dynamic subscriptions
- typed subscriptions
- synchronous wrappers around async config RPCs
- capability reporting
- UI repaint callback fanout
- partial legacy APIs such as logical-path value access and generic writes

The current `ros-z` migration state is also already degraded in two important places:

- `subscribe_value` currently pushes an unmapped-path error into the returned buffer and never creates a live subscription
- `write(path, value)` currently returns `UnsupportedCapability`

That means a large set of Twix panel call sites still compiles but already fails at runtime. Removing these APIs is therefore formalizing dead compatibility code, not regressing a working backend surface.

This causes four concrete problems:

1. Async behavior is hidden behind synchronous-looking APIs and background tasks.
2. Errors are often logged or folded into ad-hoc state instead of being returned to the caller.
3. Legacy concepts such as logical-path reads and generic writes no longer map cleanly to `ros-z`.
4. Testing boundaries are weak because runtime, transport, domain logic, and panel-facing helpers are entangled.

## Constraints And Decisions

- Use a big-bang replacement rather than a staged migration.
- Prefer fail-fast `Result`-based APIs over long-lived implicit state machines.
- Allow panel integration to change if that produces a cleaner backend design.
- Remove legacy concepts that do not fit `ros-z`, especially logical-path `subscribe_value` and generic `write(path, value)` APIs.
- Keep UI work focused on adaptation to the new backend rather than UI redesign.

## Approach Options

### Option 1: Layered Runtime And Domain Services

Create a small runtime/session layer, then expose explicit domain services for topics, config, and non-config controls. Add thin UI adapters for egui consumption.

Pros:

- clear ownership boundaries
- strong error propagation
- natural fit for async `Result`-based APIs
- easy to test by layer

Cons:

- requires broad call-site changes where panels currently depend on `Robot`

### Option 2: Central Event Store

Create one central store that owns connection state, subscriptions, caches, writes, and error state, with panels reading snapshots and dispatching actions.

Pros:

- one place for cross-panel state

Cons:

- risks replacing the current monolith with a new monolith
- makes fail-fast operation results less direct
- encourages hidden retries and implicit state transitions

### Option 3: Actor-Per-Resource Model

Represent connections, subscriptions, config queries, and writes as separate actors or tasks that communicate through messages.

Pros:

- strong isolation between resource lifecycles

Cons:

- high concurrency and lifecycle overhead
- too much machinery for Twix's current size and goals

## Decision

Use option 1: layered runtime and domain services.

Twix should have one explicit async boundary, a small session manager, focused domain services, and panel-facing adapters that do not own backend policy.

## Architecture

The redesigned backend is split into four layers.

### 1. Runtime Layer

The runtime layer owns the async world:

- Tokio runtime creation and shutdown
- `ros-z` context creation
- Twix node creation
- connect and disconnect operations
- handing out a live session handle

This layer does not expose topic, config, or command behavior directly.

### 2. Transport Layer

The transport layer is a thin wrapper around `ros-z` primitives:

- topic discovery calls
- dynamic subscriber construction
- typed subscriber construction
- config client creation and RPC execution
- outbound non-config control transport creation

Its job is translation, not orchestration. It converts raw `ros-z` errors into Twix transport errors with operation context.

Transport is implemented as an internal component, `RoszTransport`, used by the domain services. It is not a panel-facing API.

### 3. Domain Layer

The domain layer exposes backend functionality through focused services.

#### `TopicService`

Owns:

- listing topics
- subscribing to dynamic topics
- subscribing to typed topics

Public direction:

- `list_topics()`
- `subscribe_dynamic(topic)`
- `subscribe_typed<T>(topic)`

There is no logical-path subscription API in the new design.

#### `ConfigService`

Owns:

- listing config nodes
- resolving selectors to node FQNs
- snapshot access
- field-path discovery
- metadata lookup
- JSON config writes and resets

Public direction:

- `list_nodes()`
- `resolve_selector(selector)`
- `get_snapshot(selector)`
- `list_paths(selector, writable_only)`
- `get_value(selector, path)`
- `get_metadata(selector, paths)`
- `set_json(selector, path, value, layer, expected_revision)`
- `reset(selector, path, layer, expected_revision)`

#### `CommandService`

Owns explicit outbound actions that are not config RPCs.

This replaces `write(path, value)` with concrete command-oriented operations or typed publish helpers. Command behavior should be expressed in domain terms instead of through a stringly-typed generic write interface.

Current write-call triage for the existing Twix panels is:

- `panels/remote_control.rs` writes to `parameters.behavior.remote_control.walk` and `parameters.rl_walking.gait_frequency`
  These are parameter writes and belong in `ConfigService::set_json` against the owning config nodes.
- `panels/look_at.rs` writes to `parameters.behavior.injected_motion_command`
  This is also treated as a parameter write in the redesign unless the backend later grows a distinct non-config control topic.
- `panels/behavior_simulator.rs` writes to `simulator.selected_frame` and `simulator.selected_robot`
  These do not have a clean config-node mapping today and remain the initial scope of `CommandService`.

`CommandService` must stay limited to real non-config control surfaces. It must not become a bucket for parameter writes that belong in `ConfigService`.

### 4. Presentation Adapter Layer

The presentation adapter layer exists only to bridge async domain APIs into synchronous egui code.

Examples:

- latest-value adapters
- bounded history adapters for plots
- change-tracking adapters
- repaint notification helpers

These adapters consume service APIs. They do not create sessions, hide retries, or decide backend policy.

## Main Components

### `TwixRuntime`

Owns the Tokio runtime, creates the `ros-z` context and Twix node, and exposes connect, disconnect, and shutdown entry points.

`connect(endpoint)` should be an async one-shot operation returning `Result<(), TwixError>`. The app layer launches that operation through `RequestAdapter<()>` and separately observes connection or session state through `SessionManager`.

`disconnect()` should synchronously invalidate the current session and tear down runtime-owned connection state.

### `SessionManager`

Tracks whether a live session exists and hands out session handles to services. It may retain a generation counter or similar invalidation token, but it is only responsible for lifecycle and liveness.

It does not implement topic/config/command behavior.

### `TwixSession`

The session handle handed to services should be an `Arc<TwixSession>` wrapper rather than a raw `Arc<ZNode>`.

`TwixSession` owns the connection-scoped handles services actually need:

- generation or invalidation token
- `Arc<ZContext>` for graph access
- `Arc<ZNode>` for subscriptions, config RPCs, and outbound commands

This keeps service dependencies explicit and avoids scattering direct `ZContext` and `ZNode` ownership rules across the codebase.

After disconnect, previously issued `Arc<TwixSession>` handles are logically stale. Transport and service operations against a stale session should fail with `TwixError::DisconnectedSession` or an equivalent generation-invalidated error instead of silently continuing.

### `RoszTransport`

`RoszTransport` is the internal transport component used by services together with `TwixSession`. It owns the low-level `ros-z` calls for discovery, subscription construction, config client creation, and outbound non-config transport.

### `TopicService`

Uses the current session to perform discovery and create subscriptions.

Typed subscriptions should use `ros-z`'s async receive APIs such as `async_recv_with_metadata()` and run on the shared runtime. This has already been verified against `third_party/ros-z/crates/ros-z/src/pubsub.rs`, where typed subscribers expose both `async_recv()` and `async_recv_with_metadata()` today. The target architecture therefore does not include a dedicated blocking thread per typed subscription.

If a future `ros-z` operation lacks an async interface, the transport layer is responsible for isolating any blocking bridge.

### `ConfigService`

Uses the current session to discover config nodes, resolve selectors, and run config RPCs.

### `CommandService`

Uses the current session to publish outbound commands or run other explicit non-config operations.

### `SubscriptionAdapter<T>`

Bridges async topic streams into a pollable form suitable for egui panels. This is the likely home for the behavior currently spread across `Buffer`, `ChangeBuffer`, and repaint callback wiring.

### `RequestAdapter<T>`

Bridges one-shot async operations into egui polling. This is the adapter used for connect requests, config reads, config writes, config resets, topic-list refreshes, and config-node-list refreshes so panels do not call `block_on` directly.

`RequestAdapter<T>` owns spawned request tasks and exposes pollable completion as a pending or completed result.

### `RepaintSink`

Adapters should not capture backend policy through `egui::Context`, but they do need a repaint bridge. They should receive a lightweight repaint sink, such as an `Arc<dyn Fn() + Send + Sync>`, that the app wires to `ctx.request_repaint()`.

The repaint sink is owned by the app layer, constructed once, and passed into each adapter at creation time.

### `TwixError`

The public error type shared across runtime, transport, and domain APIs.

`TwixError` should be cheap to clone. Where source errors are not cloneable, the transport boundary should convert them into owned context strings and structured fields instead of storing non-cloneable error objects directly.

## Data Flow

Twix should have exactly one async boundary: the transition from egui-facing code into the runtime and domain services.

The domain layer is async-native.

- one-shot operations return `async fn -> Result<T, TwixError>`
- streaming operations return explicit subscription objects or streams
- dropping a subscription tears down its consumer-side state cleanly

Panels and panel adapters do not talk to `ros-z` directly.

The adapter layer may cache the latest successful value when that improves UI responsiveness, but it must never hide failures. If a refresh fails, the consumer can observe that failure and decide whether to keep stale data visible, clear the view, or show an error.

List-style UI state that is currently represented with `discovering` booleans should move to explicit adapter state, for example `Pending` or `Ready(Result<T, TwixError>)`. Panels should no longer infer backend health from booleans alone.

## Error Model

Error propagation is a primary design goal.

### Principles

1. Operations fail through `Result`, not through logs plus background state.
2. Streaming consumers can observe both successful items and terminal or transient failures.
3. No background task may swallow an error that matters to a consumer.
4. Errors preserve operation context such as topic name, node selector, config path, and expected type.
5. Removed abstractions should not survive as placeholder unsupported APIs.

### Public Error Shape

Twix should expose a single public `TwixError` enum with structured variants for at least:

- runtime creation or shutdown failure
- connection failure
- disconnected session access
- topic discovery failure
- dynamic schema discovery failure
- subscription creation failure
- message receive failure
- decode or type mismatch failure
- config selector resolution failure
- config RPC failure
- command publish failure

Each variant should retain enough identifying context to make logs and panel-level messages useful without additional guessing.

### Consequences For Existing Concepts

- Remove `UnsupportedCapability`-style fallbacks for concepts that no longer exist.
- Remove `discovering` booleans used as a substitute for actual operation results.
- Remove generic logical-path read and write APIs.
- Remove `BackendCapability` and `has_capability()`; availability should be represented by explicit services, explicit command adapters, and real operation results.
- Remove or rewrite `players_buffer_handle.rs`, which is built on the removed logical-path subscription API.

## UI Integration

The UI should no longer depend on one broad `Robot` facade that mixes all backend behavior.

Panels may instead receive a narrower application handle or explicit service or adapter dependencies. Panel code changes are acceptable where needed to align with the new backend, but the redesign should avoid visual behavior changes unless they are a direct consequence of improved error handling.

Expected UI-side adjustments include:

- replacing `Robot` calls with focused service or adapter usage
- handling `Result` values explicitly at panel boundaries
- using explicit topic, config, and command APIs instead of logical-path indirection
- replacing direct `block_on`-style config access with `RequestAdapter<T>` polling
- replacing `discovering` checks with explicit pending or completed request state

## Testing Strategy

Tests should follow the new boundaries.

### Unit Tests

- `TwixError` mapping and formatting
- config selector resolution logic
- adapter behavior for latest-value, history, and change-tracking cases
- session invalidation and generation handling where applicable

### Integration Tests

Use small `ros-z` demo nodes and Twix examples to test:

- topic discovery
- dynamic subscription success
- typed subscription success
- config node discovery
- config value reads
- config metadata reads
- config writes and resets
- command publishing

### Failure-Path Integration Tests

Add explicit tests for:

- connection failure
- disconnected session use
- dynamic schema discovery failure
- decode mismatch or wrong expected type
- ambiguous config node selector
- failed config write or reset
- subscription receive failure

UI tests should stay minimal. Most behavior should be verified by testing adapters and services instead of pixel output.

## Migration Shape

This is a big-bang replacement at merge time.

The old `Robot`-centered backend should be removed rather than preserved behind a long-lived compatibility layer. Inside the implementation branch, a short-lived `Robot`-shaped shim is acceptable and preferred if it delegates directly to the new services and keeps the tree buildable while panels migrate incrementally. That shim is scaffolding only and must be deleted before merge.

## Success Criteria

The redesign is successful when:

- no non-UI Twix module owns both backend lifecycle and domain behavior the way `Robot` does today
- async behavior is explicit at the runtime and domain boundary
- topic, config, and command APIs are separate and domain-specific
- panels or their adapters observe backend failures through structured results
- legacy logical-path and generic write abstractions are removed
- service and failure-path tests cover the new architecture
