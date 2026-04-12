# Schema Generation And Discovery

ros-z has one internal schema model and one discovery transport.

- The internal model is `MessageSchema`, `FieldSchema`, and `FieldType`.
- The discovery transport is ros-z `GetTypeDescription`, which carries the full schema JSON.

This chapter explains how static Rust types become runtime schemas and how dynamic discovery drives tools such as `rosz echo`.

## The Core Model

Every runtime schema in ros-z uses the same in-memory representation:

- `MessageSchema`: the full message type
- `FieldSchema`: one named field
- `FieldType`: the field shape

`FieldType` is the key boundary between standard-compatible and ros-z-only schemas.

Standard-compatible field kinds:

- primitives
- `Message`
- `Array`
- `Sequence`
- `BoundedSequence`
- `BoundedString`

Extended-only field kinds:

- `Optional`
- `Enum`

That split still affects hashing and ROS 2 compatibility, but not ros-z schema transport.

## Trait Responsibilities

Schema generation is split across three traits.

### `MessageTypeInfo`

`MessageTypeInfo` is the top-level metadata trait.

It is responsible for:

- DDS type name
- advertised type hash
- the full `message_schema()`
- a default nested field shape via `field_type()`

This trait is used by typed publishers when they advertise a topic and register schemas with the node's type description service.

### `FieldTypeInfo`

`FieldTypeInfo` is the nested field-shape trait.

It is responsible for exactly one thing:

- mapping a Rust field type to a runtime `FieldType`

Most ordinary message types get this automatically through `MessageTypeInfo`.

The main reason `FieldTypeInfo` exists is to support Rust field types that are not standalone ROS messages but still have a well-defined wire shape. The new nalgebra integration uses this hook directly:

- `Point2<f32>` maps to `float32[2]`
- `Point3<f64>` maps to `float64[3]`
- `Isometry3<f32>` maps to a nested message with `rotation` and `translation`

## Derive Behavior

ros-z provides one derive.

### `#[derive(MessageTypeInfo)]`

This derive supports the full ros-z schema surface.

It accepts:

- named structs
- enums
- primitives
- `String`
- `Option<T>`
- `Vec<T>`
- `[T; N]`
- nested field types that implement `FieldTypeInfo`

It rejects:

- maps
- tuples
- other non-ROS shapes

For unknown nested fields the derive now calls:

```rust,ignore
<#ty as ::ros_z::FieldTypeInfo>::field_type()
```

This is what allows nalgebra-backed and other reusable field families to appear inside richer message schemas directly.

## Publisher Registration Flow

When you create a typed publisher with:

```rust,ignore
let publisher = node.create_pub::<T>("topic").build()?;
```

ros-z runs the following flow:

1. Read `T::type_info()` for the advertised DDS type name and hash.
2. Call `T::message_schema()`.
3. Register that schema with the node's `TypeDescriptionService`.
4. Attach the schema to the publisher builder for dynamic serialization support.

There is one schema service.

- Registration uses `~get_type_description`.

## Discovery

Dynamic subscriber auto-discovery always produces the same result:

- a `MessageSchema`
- a discovered type hash

Discovery flow:

1. Qualify the topic name.
2. Look up current publishers in the graph.
3. Collect candidate publisher nodes plus their advertised type names and hashes.
4. Query each candidate's `~get_type_description` service.
5. Receive ros-z schema JSON and convert it back into `MessageSchema`.
6. Build a dynamic subscriber with that schema.

## Dynamic Runtime Values

Once a schema is known, dynamic decoding uses:

- `DynamicMessage`
- `DynamicValue`

`DynamicMessage` stores:

- the discovered `MessageSchema`
- one `DynamicValue` per field

`DynamicValue` supports:

- primitives
- nested messages
- arrays
- `Optional`
- enums

`rosz echo` uses `create_dyn_sub_auto()` and then renders `DynamicMessage` as JSON. That is why fixing schema generation automatically improves the CLI.

## Mixing Standard And Non-Standard Fields

The key composition rule is now:

- top-level message identity comes from `MessageTypeInfo`
- nested field shape comes from `FieldTypeInfo`

That means a message containing `Optional` or enums can contain nested fields that only need a standard-compatible schema shape.

Examples:

- generated ROS messages can be nested inside richer Rust-native messages
- nalgebra field types can appear inside both standard-compatible and non-standard messages
- only the presence of `Optional` or enums changes the hash/compatibility story; discovery still uses one transport

This keeps the model simple:

- standard-compatible nested fields stay standard-compatible
- non-standard nested fields remain explicit
- the full message still serializes through one schema transport

## Nalgebra Support

ros-z now includes first-class `FieldTypeInfo` impls for a focused set of nalgebra aliases.

Supported families:

- `Point2`, `Point3`
- `Vector2`, `Vector3`
- `Translation2`, `Translation3`
- `Rotation2`, `Rotation3`
- `UnitComplex`, `UnitQuaternion`
- `Isometry2`, `Isometry3`

for both `f32` and `f64`.

The contract is simple:

- ros-z mirrors serde's wire shape
- not a hand-designed ROS-friendly shape

Examples:

- `Point3<f64>` becomes `float64[3]`
- `UnitComplex<f64>` becomes `float64[2]`
- `Isometry3<f32>` becomes a nested message with:
  - `rotation`
  - `translation`

This is exactly what makes nalgebra-backed transport messages work with dynamic discovery and with `rosz echo`.

## The Hulk Example

The `hulk` binary is now a concrete end-to-end example of the model.

Pipeline:

- `ball_detection` publishes `BallObservation`
- `ball_filter` consumes `BallObservation` and publishes `BallTrack`
- `motion` consumes `BallTrack` and publishes `WalkCommand`

The pipeline intentionally exercises both schema shapes:

- `BallTrack` is standard-compatible
- `BallObservation` and `WalkCommand` use `Option` or enums

All three messages use nalgebra field types directly.

That makes `hulk` a good reference for:

- nalgebra-backed transport messages
- mixed standard and non-standard schemas
- dynamic subscriber auto-discovery
- CLI inspection with `rosz echo`

## Practical Rules

When authoring new message types, use these rules:

1. Use `MessageTypeInfo` for Rust-native messages, including enums and `Option<T>`.
2. Add `FieldTypeInfo` impls for reusable field families that are not ordinary top-level ROS messages.
3. Enable `with_type_description_service()` on nodes that publish schemas you want to expose dynamically.

That is enough for `create_dyn_sub_auto()` and `rosz echo` to discover and decode the topic automatically.
