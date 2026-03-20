// ABOUTME: Tests for optionality inference on non-optional proto fields.
// ABOUTME: Exercises Bug 2 — custom types on required proto fields and proto_required attribute.

use crate::basic_types::Status;
use crate::proto;
use protto::Protto;

// === Category 4: Required enum fields (no .expect() on i32) ===

// All fields from RequiredFieldsMessage must be present or ignored.
// The key test: `required_enum: Status` compiles without .expect() on i32.
#[derive(Protto, PartialEq, Debug, Clone)]
#[protto(proto_name = "RequiredFieldsMessage")]
pub struct RequiredEnumStruct {
    pub required_enum: Status,
    pub required_repeated: Vec<i32>,
    pub required_bool: bool,
    pub required_string: String,
    pub required_number: u64,
}

#[test]
fn required_enum_from_proto() {
    let proto_msg = proto::RequiredFieldsMessage {
        required_enum: proto::Status::Found as i32,
        required_repeated: vec![],
        required_bool: false,
        required_string: "hello".to_string(),
        required_number: 0,
    };

    let converted: RequiredEnumStruct = proto_msg.into();
    assert_eq!(converted.required_enum, Status::Found);
    assert_eq!(converted.required_string, "hello");
}

#[test]
fn required_enum_to_proto() {
    let rust_struct = RequiredEnumStruct {
        required_enum: Status::Ok,
        required_repeated: vec![1, 2, 3],
        required_bool: true,
        required_string: "world".to_string(),
        required_number: 99,
    };

    let proto_msg: proto::RequiredFieldsMessage = rust_struct.into();
    assert_eq!(proto_msg.required_enum, proto::Status::Ok as i32);
    assert_eq!(proto_msg.required_string, "world");
    assert_eq!(proto_msg.required_repeated, vec![1, 2, 3]);
}

#[test]
fn required_enum_zero_value() {
    let proto_msg = proto::RequiredFieldsMessage::default();

    let converted: RequiredEnumStruct = proto_msg.into();
    assert_eq!(converted.required_enum, Status::Ok);
    assert_eq!(converted.required_string, "");
    assert_eq!(converted.required_repeated, Vec::<i32>::new());
    assert_eq!(converted.required_bool, false);
    assert_eq!(converted.required_number, 0);
}

// === Category 5: proto_required attribute ===

#[derive(Protto, PartialEq, Debug, Clone)]
#[protto(proto_name = "RequiredFieldsMessage")]
pub struct ExplicitRequiredStruct {
    #[protto(proto_required)]
    pub required_enum: Status,
    pub required_repeated: Vec<i32>,
    pub required_bool: bool,
    pub required_string: String,
    pub required_number: u64,
}

#[test]
fn proto_required_on_enum_from_proto() {
    let proto_msg = proto::RequiredFieldsMessage {
        required_enum: proto::Status::NotFound as i32,
        required_repeated: vec![42],
        required_bool: true,
        required_string: String::new(),
        required_number: 0,
    };

    let converted: ExplicitRequiredStruct = proto_msg.into();
    assert_eq!(converted.required_enum, Status::NotFound);
    assert_eq!(converted.required_bool, true);
    assert_eq!(converted.required_repeated, vec![42]);
}

#[test]
fn proto_required_on_enum_to_proto() {
    let rust_struct = ExplicitRequiredStruct {
        required_enum: Status::MovedPermanently,
        required_repeated: vec![],
        required_bool: false,
        required_string: "test".to_string(),
        required_number: 7,
    };

    let proto_msg: proto::RequiredFieldsMessage = rust_struct.into();
    assert_eq!(
        proto_msg.required_enum,
        proto::Status::MovedPermanently as i32
    );
    assert_eq!(proto_msg.required_repeated, Vec::<i32>::new());
    assert_eq!(proto_msg.required_bool, false);
    assert_eq!(proto_msg.required_string, "test");
    assert_eq!(proto_msg.required_number, 7);
}

// === Category 6: Mixed optionality in same struct ===

#[derive(Protto, PartialEq, Debug, Clone)]
#[protto(proto_name = "RequiredFieldsMessage")]
pub struct MixedRequiredStruct {
    pub required_enum: Status,
    pub required_repeated: Vec<i32>,
    pub required_bool: bool,
    pub required_number: u64,
    pub required_string: String,
}

#[test]
fn mixed_required_roundtrip() {
    let original = MixedRequiredStruct {
        required_enum: Status::Found,
        required_repeated: vec![10, 20],
        required_bool: true,
        required_number: 42,
        required_string: "test".to_string(),
    };

    let proto_msg: proto::RequiredFieldsMessage = original.clone().into();
    assert_eq!(proto_msg.required_enum, proto::Status::Found as i32);
    assert_eq!(proto_msg.required_bool, true);
    assert_eq!(proto_msg.required_number, 42);
    assert_eq!(proto_msg.required_string, "test");
    assert_eq!(proto_msg.required_repeated, vec![10, 20]);

    let roundtrip: MixedRequiredStruct = proto_msg.into();
    assert_eq!(original, roundtrip);
}

#[test]
fn mixed_required_default_values() {
    let proto_msg = proto::RequiredFieldsMessage::default();

    let converted: MixedRequiredStruct = proto_msg.into();
    assert_eq!(converted.required_enum, Status::Ok);
    assert_eq!(converted.required_bool, false);
    assert_eq!(converted.required_number, 0);
    assert_eq!(converted.required_string, "");
    assert_eq!(converted.required_repeated, Vec::<i32>::new());
}
