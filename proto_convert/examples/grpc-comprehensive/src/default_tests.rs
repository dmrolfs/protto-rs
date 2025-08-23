use crate::default_types::*;
use crate::proto;
use crate::shared_types::arb_proto_track_with_optionals;
use crate::shared_types::*;
use proptest::prelude::*;

proptest! {

    #[test]
    fn test_default_behavior(proto_track in arb_proto_track_with_optionals()) {
        let rust_track: TrackWithDefault = proto_track.clone().into();

        // Check ID conversion
        assert_eq!(rust_track.id, TrackId::new(proto_track.track_id));

        // Check default behavior
        if let Some(ref name) = proto_track.name {
            assert_eq!(rust_track.name, *name);
        } else {
            // Should use String::default() = ""
            assert_eq!(rust_track.name, String::default());
        }

        if let Some(duration) = proto_track.duration {
            assert_eq!(rust_track.duration, duration);
        } else {
            // Should use u32::default() = 0
            assert_eq!(rust_track.duration, u32::default());
        }

        // Test roundtrip
        let back_to_proto: proto::TrackWithOptionals = rust_track.into();

        // The roundtrip should preserve the original values or convert defaults back
        assert_eq!(back_to_proto.track_id, proto_track.track_id);

        // For optional fields, check if defaults are handled correctly
        match proto_track.name {
            Some(ref original_name) => assert_eq!(back_to_proto.name, Some(original_name.clone())),
            None => {
                // Default value should be converted back to Some("")
                assert_eq!(back_to_proto.name, Some(String::default()));
            }
        }
    }

    #[test]
    fn test_custom_default_behavior(proto_track in arb_proto_track_with_optionals()) {
        let rust_track: TrackWithCustomDefault = proto_track.clone().into();

        // Check custom defaults
        if proto_track.name.is_none() {
            assert_eq!(rust_track.name, "Unknown Track");
        }

        if proto_track.duration.is_none() {
            assert_eq!(rust_track.duration, 180);
        }
    }
}

#[test]
fn test_none_values_get_defaults() {
    let proto_with_nones = proto::TrackWithOptionals {
        track_id: 123,
        name: None,
        duration: None,
    };

    let rust_track: TrackWithDefault = proto_with_nones.into();

    assert_eq!(rust_track.id, TrackId::new(123));
    assert_eq!(rust_track.name, ""); // String::default()
    assert_eq!(rust_track.duration, 0); // u32::default()
}

#[test]
fn test_custom_defaults_on_none() {
    let proto_with_nones = proto::TrackWithOptionals {
        track_id: 456,
        name: None,
        duration: None,
    };

    let rust_track: TrackWithCustomDefault = proto_with_nones.into();

    assert_eq!(rust_track.id, TrackId::new(456));
    assert_eq!(rust_track.name, "Unknown Track");
    assert_eq!(rust_track.duration, 180);
}

#[test]
fn test_some_values_preserved() {
    let proto_with_values = proto::TrackWithOptionals {
        track_id: 789,
        name: Some("My Track".to_string()),
        duration: Some(240),
    };

    let rust_track: TrackWithDefault = proto_with_values.clone().into();

    assert_eq!(rust_track.id, TrackId::new(789));
    assert_eq!(rust_track.name, "My Track");
    assert_eq!(rust_track.duration, 240);

    // Test roundtrip preserves values
    let back_to_proto: proto::TrackWithOptionals = rust_track.into();
    assert_eq!(back_to_proto, proto_with_values);
}
