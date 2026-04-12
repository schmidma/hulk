use std::time::Duration;

use ros_z::{
    Builder, MessageTypeInfo,
    context::ZContextBuilder,
    dynamic::{DynamicValue, EnumPayloadValue},
};
use serde::{Deserialize, Serialize};
use zenoh::{Wait, config::WhatAmI};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ros_z::MessageTypeInfo)]
#[ros_msg(type_name = "custom_msgs/msg/TelemetryLite")]
struct TelemetryLite {
    label: String,
    temperatures: Vec<f32>,
}

impl ros_z::msg::ZMessage for TelemetryLite {
    type Serdes = ros_z::msg::SerdeCdrSerdes<Self>;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ros_z::MessageTypeInfo)]
#[ros_msg(type_name = "custom_msgs/msg/RobotState")]
enum RobotState {
    Idle,
    Error(String),
    Charging { minutes_remaining: u32 },
}

impl ros_z::msg::ZMessage for RobotState {
    type Serdes = ros_z::msg::SerdeCdrSerdes<Self>;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ros_z::MessageTypeInfo)]
#[ros_msg(type_name = "custom_msgs/msg/RobotEnvelope")]
struct RobotEnvelope {
    label: String,
    mission_id: Option<u32>,
    state: RobotState,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ros_z::MessageTypeInfo)]
#[ros_msg(type_name = "custom_msgs/msg/GenericEnvelope")]
struct GenericEnvelope<T> {
    payload: T,
    items: Vec<T>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ros_z::MessageTypeInfo)]
#[ros_msg(type_name = "custom_msgs/msg/GenericOptionalEnvelope")]
struct GenericOptionalEnvelope<T> {
    payload: Option<T>,
    items: Vec<T>,
}

impl ros_z::msg::ZMessage for RobotEnvelope {
    type Serdes = ros_z::msg::SerdeCdrSerdes<Self>;
}

struct TestRouter {
    endpoint: String,
    _session: zenoh::Session,
}

impl TestRouter {
    fn new() -> Self {
        let port = {
            let listener =
                std::net::TcpListener::bind("127.0.0.1:0").expect("failed to bind port 0");
            listener.local_addr().unwrap().port()
        };

        let endpoint = format!("tcp/127.0.0.1:{port}");
        let mut config = zenoh::Config::default();
        config.set_mode(Some(WhatAmI::Router)).unwrap();
        config
            .insert_json5("listen/endpoints", &format!("[\"{endpoint}\"]"))
            .unwrap();
        config
            .insert_json5("scouting/multicast/enabled", "false")
            .unwrap();

        let session = zenoh::open(config)
            .wait()
            .expect("failed to open test router");
        std::thread::sleep(Duration::from_millis(300));

        Self {
            endpoint,
            _session: session,
        }
    }

    fn endpoint(&self) -> &str {
        &self.endpoint
    }
}

fn create_context_with_router(router: &TestRouter) -> ros_z::Result<ros_z::context::ZContext> {
    ZContextBuilder::default()
        .disable_multicast_scouting()
        .with_connect_endpoints([router.endpoint()])
        .build()
}

#[test]
fn extended_derive_keeps_standard_schema_for_compatible_structs() {
    let schema = TelemetryLite::message_schema();
    assert!(!schema.uses_extended_types());
    assert_eq!(schema.type_name, "custom_msgs/msg/TelemetryLite");

    assert!(RobotEnvelope::message_schema().uses_extended_types());
    assert!(RobotState::message_schema().uses_extended_types());
}

#[test]
fn extended_derive_generates_distinct_generic_names_and_hashes() {
    let u32_schema = GenericEnvelope::<u32>::message_schema();
    let msg_schema = GenericEnvelope::<TelemetryLite>::message_schema();

    assert_eq!(
        GenericEnvelope::<u32>::type_name(),
        "custom_msgs/msg/GenericEnvelope__u32"
    );
    assert_eq!(
        GenericEnvelope::<TelemetryLite>::type_name(),
        "custom_msgs/msg/GenericEnvelope__custom_msgs_msg_telemetrylite"
    );
    assert_ne!(
        GenericEnvelope::<u32>::type_hash(),
        GenericEnvelope::<TelemetryLite>::type_hash()
    );

    let payload = msg_schema.field("payload").expect("payload field");
    match &payload.field_type {
        ros_z::dynamic::FieldType::Message(nested) => {
            assert_eq!(nested.type_name, "custom_msgs/msg/TelemetryLite");
        }
        other => panic!("expected nested message payload, got {:?}", other),
    }

    assert_eq!(u32_schema.type_name, GenericEnvelope::<u32>::type_name());
}

#[test]
fn extended_derive_keeps_extended_only_generic_instantiations_on_extended_path() {
    let schema = GenericOptionalEnvelope::<u32>::message_schema();
    assert!(schema.uses_extended_types());
    assert_eq!(
        GenericOptionalEnvelope::<u32>::type_name(),
        "custom_msgs/msg/GenericOptionalEnvelope__u32"
    );

    let payload = schema.field("payload").expect("payload field");
    match &payload.field_type {
        ros_z::dynamic::FieldType::Optional(inner) => {
            assert!(matches!(inner.as_ref(), ros_z::dynamic::FieldType::Uint32));
        }
        other => panic!("expected optional payload field, got {:?}", other),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn discovery_uses_type_description_service_for_standard_compatible_types() {
    let router = TestRouter::new();

    let pub_ctx = create_context_with_router(&router).expect("publisher context");
    let pub_node = pub_ctx
        .create_node("telemetry_talker")
        .with_type_description_service()
        .build()
        .expect("publisher node");

    let publisher = pub_node
        .create_pub::<TelemetryLite>("/extended_standard_topic")
        .build()
        .expect("publisher");

    let registered = pub_node
        .type_description_service()
        .expect("standard type description service")
        .get_schema("custom_msgs/msg/TelemetryLite")
        .expect("schema lookup");
    assert!(
        registered.is_some(),
        "standard-compatible schema should register"
    );

    let sub_ctx = create_context_with_router(&router).expect("subscriber context");
    let sub_node = sub_ctx
        .create_node("telemetry_listener")
        .build()
        .expect("subscriber node");

    let publish_task = tokio::spawn(async move {
        for _ in 0..20 {
            let msg = TelemetryLite {
                label: "robot-7".to_string(),
                temperatures: vec![20.0, 20.5],
            };
            publisher.publish(&msg).expect("publish");
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    });

    tokio::time::sleep(Duration::from_millis(400)).await;

    let subscriber = sub_node
        .create_dyn_sub_auto("/extended_standard_topic", Duration::from_secs(10))
        .await
        .expect("dynamic subscriber")
        .build()
        .expect("subscriber build");
    let schema = subscriber.schema().expect("discovered schema");

    assert_eq!(schema.type_name, "custom_msgs/msg/TelemetryLite");
    assert!(!schema.uses_extended_types());

    let message = subscriber
        .recv_timeout(Duration::from_secs(3))
        .expect("dynamic message");
    assert_eq!(
        message.get::<String>("label").unwrap(),
        "robot-7".to_string()
    );

    publish_task.await.expect("publisher task");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn extended_only_types_require_type_description_service_enablement() {
    let router = TestRouter::new();

    let pub_ctx = create_context_with_router(&router).expect("publisher context");
    let pub_node = pub_ctx
        .create_node("extended_talker")
        .build()
        .expect("publisher node");

    let publisher = pub_node
        .create_pub::<RobotEnvelope>("/extended_robot_topic")
        .build()
        .expect("publisher");

    let sub_ctx = create_context_with_router(&router).expect("subscriber context");
    let sub_node = sub_ctx
        .create_node("extended_listener")
        .build()
        .expect("subscriber node");

    let publish_task = tokio::spawn(async move {
        for _ in 0..20 {
            let msg = RobotEnvelope {
                label: "robot-9".to_string(),
                mission_id: Some(42),
                state: RobotState::Charging {
                    minutes_remaining: 12,
                },
            };
            publisher.publish(&msg).expect("publish");
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    });

    tokio::time::sleep(Duration::from_millis(400)).await;

    let result = sub_node
        .create_dyn_sub_auto("/extended_robot_topic", Duration::from_secs(3))
        .await;
    assert!(
        result.is_err(),
        "extended discovery should fail when the publisher did not enable the type description service"
    );

    publish_task.await.expect("publisher task");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn extended_only_types_use_type_description_service_when_enabled() {
    let router = TestRouter::new();

    let pub_ctx = create_context_with_router(&router).expect("publisher context");
    let pub_node = pub_ctx
        .create_node("extended_talker")
        .with_type_description_service()
        .build()
        .expect("publisher node");

    let publisher = pub_node
        .create_pub::<RobotEnvelope>("/extended_robot_topic")
        .build()
        .expect("publisher");

    let registered = pub_node
        .type_description_service()
        .expect("type description service")
        .get_schema("custom_msgs/msg/RobotEnvelope")
        .expect("schema lookup");
    let registered = registered.expect("extended schema should register");
    assert_eq!(registered.type_hash, RobotEnvelope::type_hash().to_rihs_string());

    let sub_ctx = create_context_with_router(&router).expect("subscriber context");
    let sub_node = sub_ctx
        .create_node("extended_listener")
        .build()
        .expect("subscriber node");

    let publish_task = tokio::spawn(async move {
        for _ in 0..20 {
            let msg = RobotEnvelope {
                label: "robot-9".to_string(),
                mission_id: Some(42),
                state: RobotState::Charging {
                    minutes_remaining: 12,
                },
            };
            publisher.publish(&msg).expect("publish");
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    });

    tokio::time::sleep(Duration::from_millis(400)).await;

    let subscriber = sub_node
        .create_dyn_sub_auto("/extended_robot_topic", Duration::from_secs(10))
        .await
        .expect("dynamic subscriber")
        .build()
        .expect("subscriber build");
    let schema = subscriber.schema().expect("discovered schema");

    assert!(schema.uses_extended_types());
    assert_eq!(schema.type_name, "custom_msgs/msg/RobotEnvelope");

    let message = subscriber
        .recv_timeout(Duration::from_secs(3))
        .expect("dynamic message");
    assert_eq!(
        message.get::<String>("label").unwrap(),
        "robot-9".to_string()
    );

    match message.get_dynamic("mission_id").expect("mission_id") {
        DynamicValue::Optional(Some(value)) => {
            assert_eq!(value.as_ref().as_u32(), Some(42));
        }
        other => panic!("expected Some mission_id, got {other:?}"),
    }

    match message.get_dynamic("state").expect("state") {
        DynamicValue::Enum(value) => {
            assert_eq!(value.variant_name, "Charging");
            match value.payload {
                EnumPayloadValue::Struct(fields) => {
                    assert_eq!(fields.len(), 1);
                    assert_eq!(fields[0].name, "minutes_remaining");
                    assert_eq!(fields[0].value.as_u32(), Some(12));
                }
                other => panic!("expected struct payload, got {other:?}"),
            }
        }
        other => panic!("expected enum value, got {other:?}"),
    }

    publish_task.await.expect("publisher task");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn type_description_discovery_works_across_namespaces_for_extended_types() {
    let router = TestRouter::new();

    let pub_ctx = create_context_with_router(&router).expect("publisher context");
    let pub_node = pub_ctx
        .create_node("extended_talker")
        .with_namespace("tools")
        .with_type_description_service()
        .build()
        .expect("publisher node");

    let publisher = pub_node
        .create_pub::<RobotEnvelope>("/extended_robot_topic")
        .build()
        .expect("publisher");

    let sub_ctx = create_context_with_router(&router).expect("subscriber context");
    let sub_node = sub_ctx
        .create_node("extended_listener")
        .with_namespace("ui")
        .build()
        .expect("subscriber node");

    let publish_task = tokio::spawn(async move {
        for _ in 0..20 {
            let msg = RobotEnvelope {
                label: "robot-9".to_string(),
                mission_id: Some(42),
                state: RobotState::Charging {
                    minutes_remaining: 12,
                },
            };
            publisher.publish(&msg).expect("publish");
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    });

    tokio::time::sleep(Duration::from_millis(400)).await;

    let subscriber = sub_node
        .create_dyn_sub_auto("/extended_robot_topic", Duration::from_secs(10))
        .await
        .expect("dynamic subscriber")
        .build()
        .expect("subscriber build");
    let schema = subscriber.schema().expect("discovered schema");

    assert!(schema.uses_extended_types());
    assert_eq!(schema.type_name, "custom_msgs/msg/RobotEnvelope");

    let message = subscriber
        .recv_timeout(Duration::from_secs(3))
        .expect("dynamic message");
    assert_eq!(
        message.get::<String>("label").unwrap(),
        "robot-9".to_string()
    );

    publish_task.await.expect("publisher task");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn top_level_enums_are_discoverable_through_the_type_description_service() {
    let router = TestRouter::new();

    let pub_ctx = create_context_with_router(&router).expect("publisher context");
    let pub_node = pub_ctx
        .create_node("state_talker")
        .with_type_description_service()
        .build()
        .expect("publisher node");

    let publisher = pub_node
        .create_pub::<RobotState>("/robot_state_topic")
        .build()
        .expect("publisher");

    let sub_ctx = create_context_with_router(&router).expect("subscriber context");
    let sub_node = sub_ctx
        .create_node("state_listener")
        .build()
        .expect("subscriber node");

    let publish_task = tokio::spawn(async move {
        for _ in 0..20 {
            publisher
                .publish(&RobotState::Error("battery low".to_string()))
                .expect("publish");
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    });

    tokio::time::sleep(Duration::from_millis(400)).await;

    let subscriber = sub_node
        .create_dyn_sub_auto("/robot_state_topic", Duration::from_secs(10))
        .await
        .expect("enum discovery")
        .build()
        .expect("subscriber build");
    let schema = subscriber.schema().expect("discovered schema");

    assert_eq!(schema.field_count(), 1);
    assert_eq!(schema.field("value").unwrap().name, "value");

    let message = subscriber
        .recv_timeout(Duration::from_secs(3))
        .expect("dynamic message");

    match message.get_dynamic("value").expect("enum value") {
        DynamicValue::Enum(value) => {
            assert_eq!(value.variant_name, "Error");
            match value.payload {
                EnumPayloadValue::Newtype(value) => {
                    assert_eq!(value.as_ref().as_str(), Some("battery low"));
                }
                other => panic!("expected newtype payload, got {other:?}"),
            }
        }
        other => panic!("expected enum field, got {other:?}"),
    }

    publish_task.await.expect("publisher task");
}
