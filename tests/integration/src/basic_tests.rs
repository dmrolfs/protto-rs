use crate::basic_types::*;
use crate::complex_types::*;
use crate::proto;
use crate::shared_types::*;
use proptest::prelude::*;

proptest! {
    #[test]
    fn roundtrip_track(proto_track in any::<proto::Track>()) {
        let rust_track: Track = proto_track.into();
        let back_to_proto: proto::Track = rust_track.into();
        assert_eq!(proto_track, back_to_proto);
    }

    #[test]
    fn roundtrip_state(proto_state in any::<proto::State>()) {
        let rust_state: State = proto_state.clone().into();
        let back_to_proto: proto::State = rust_state.into();
        assert_eq!(proto_state, back_to_proto);
    }

    #[test]
    fn roundtrip_proto_state(proto_state in any::<proto::State>()) {
        let rust_state: ProtoState = proto_state.clone().into();
        let back_to_proto: proto::State = rust_state.into();
        assert_eq!(proto_state, back_to_proto);
    }

    #[test]
    fn roundtrip_header(proto_header in any::<proto::Header>()) {
        let rust_header: proto::Header = proto_header.clone();
        let back_to_proto: proto::Header = rust_header.clone();
        assert_eq!(proto_header, back_to_proto);
    }

    #[test]
    fn roundtrip_request(proto_request in any::<proto::Request>().prop_filter(
        "Header must not be None",
        |req| req.header.is_some()
    )) {
        let rust_request: Request = proto_request.clone().into();
        let back_to_proto: proto::Request = rust_request.into();
        assert_eq!(proto_request, back_to_proto);
    }

    #[test]
    fn roundtrip_status(status in any::<Status>()) {
        let proto_status: proto::Status = status.clone().into();
        let back_to_rust: Status = proto_status.into();
        assert_eq!(status, back_to_rust);
    }

    #[test]
    fn roundtrip_status_response(status in any::<StatusResponse>()) {
        let proto_status: proto::StatusResponse = status.clone().into();
        let back_to_rust: StatusResponse = proto_status.into();
        assert_eq!(status, back_to_rust);
    }

    #[test]
    fn roundtrip_has_straight(proto_has_straight in any::<proto::HasStraight>()) {
        if proto_has_straight.track.is_none() {
            // expect a panic when track is None due to #[proto(expect(panic))]
            let result = std::panic::catch_unwind(|| {
                let _: HasStraight = proto_has_straight.into();
            });
            assert!(result.is_err(), "Expected panic when track is `None`");
        } else {
            // Normal roundtrip test when track is Some
            let rust_has_straight: HasStraight = proto_has_straight.into();
            let back_to_proto: proto::HasStraight = rust_has_straight.into();
            assert_eq!(proto_has_straight, back_to_proto);
        }
    }

    #[test]
    fn roundtrip_has_optional(proto_has_optional in any::<proto::HasOptional>()) {
        let rust_has_optional: HasOptional = proto_has_optional.into();
        let back_to_proto: proto::HasOptional = rust_has_optional.into();
        assert_eq!(proto_has_optional, back_to_proto);
    }
}

// Test all tests from the documentation
#[test]
fn test_basic_usage_example() {
    let proto_track = proto::Track { track_id: 42 };
    let rust_track: Track = proto_track.clone().into();
    assert_eq!(rust_track.id, 42);

    let back_to_proto: proto::Track = rust_track.into();
    assert_eq!(back_to_proto, proto_track);
}

#[test]
fn test_error_handling_examples() {
    // Test panic example
    let proto_user = proto::SimpleMessage {
        required_field: Some("user123".to_string()),
        required_number: Some(456),
        optional_field: None,
    };

    let panic_result = std::panic::catch_unwind(|| {
        let user: ExpectPanicStruct = proto_user.clone().into();
        user
    });
    assert!(panic_result.is_ok());

    // Test error example
    let error_result: Result<ExpectErrorStruct, ExpectErrorStructConversionError> =
        proto_user.try_into();
    assert!(error_result.is_ok());
}

#[test]
fn test_default_values_example() {
    let proto_track = proto::OptionalMessage {
        id: 1,
        name: None,     // Should use default
        count: None,    // Should use default
        priority: None, // Should use custom default
        tags: vec![],   // Should use custom default
    };

    let rust_track: DefaultStruct = proto_track.into();
    assert_eq!(rust_track.name, ""); // String::default()
    assert_eq!(rust_track.count, 0); // u32::default()
    assert_eq!(rust_track.priority, 10); // custom default
    assert_eq!(rust_track.tags, vec!["default"]); // custom default
}

#[test]
fn test_ignoring_fields_example() {
    let proto_state = proto::State {
        tracks: vec![proto::Track { track_id: 999 }],
    };

    let complex_state: ComplexState = proto_state.into();
    assert_eq!(complex_state.tracks.len(), 1);
    assert_eq!(complex_state.tracks[0].id, 999);

    // Ignored fields use defaults
    assert!(complex_state.launches.is_empty());
    assert_eq!(
        complex_state
            .counter
            .load(std::sync::atomic::Ordering::Relaxed),
        0
    );
}

#[test]
fn test_enum_support_example() {
    let status_mappings = [
        (Status::Ok, proto::Status::Ok),
        (Status::MovedPermanently, proto::Status::MovedPermanently),
        (Status::Found, proto::Status::Found),
        (Status::NotFound, proto::Status::NotFound),
    ];

    for (rust_status, proto_status) in status_mappings {
        // Test Rust -> Proto
        let converted_proto: proto::Status = rust_status.clone().into();
        assert_eq!(converted_proto, proto_status);

        // Test Proto -> Rust
        let converted_rust: Status = proto_status.into();
        assert_eq!(converted_rust, rust_status);

        // Test through StatusResponse
        let proto_response = proto::StatusResponse {
            status: proto_status as i32,
            message: "test message".to_string(),
        };

        let rust_response: StatusResponse = proto_response.clone().into();
        assert_eq!(rust_response.status, rust_status);
        assert_eq!(rust_response.message, "test message");

        let back_to_proto: proto::StatusResponse = rust_response.into();
        assert_eq!(back_to_proto, proto_response);
    }
}

#[test]
fn test_status() {
    let mappings = vec![
        (proto::Status::Ok, Status::Ok),
        (proto::Status::MovedPermanently, Status::MovedPermanently),
        (proto::Status::Found, Status::Found),
        (proto::Status::NotFound, Status::NotFound),
    ];

    // Proto -> rust.
    for (proto_status, rust_status) in &mappings {
        let converted: Status = (*proto_status).into();
        assert_eq!(converted, *rust_status);
    }

    // Rust -> proto.
    for (proto_status, rust_status) in &mappings {
        let converted: proto::Status = (*rust_status).clone().into();
        assert_eq!(converted, *proto_status);
    }
}

// Edge case: Ensure None stays None
#[test]
fn test_has_optional_none() {
    let proto_msg = proto::HasOptional { track: None };
    let rust_msg: HasOptional = proto_msg.into();
    assert_eq!(rust_msg.track, None);

    let back_to_proto: proto::HasOptional = rust_msg.into();
    assert_eq!(back_to_proto.track, None);
}

// Edge case: Ensure empty State roundtrips correctly
#[test]
fn test_empty_state() {
    let proto_state = proto::State { tracks: vec![] };
    let rust_state: State = proto_state.clone().into();
    assert_eq!(rust_state.tracks, vec![]);

    let back_to_proto: proto::State = rust_state.into();
    assert_eq!(back_to_proto.tracks, vec![]);
}
