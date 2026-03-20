// ABOUTME: Tests for enum prefix computation and bare variant name matching.
// ABOUTME: Exercises Bug 1 — multi-word enum names and prefix-free proto conventions.

use crate::proto;
use protto::Protto;

// --- Multi-word enum WITH standard prefix convention ---
// Proto: PAYMENT_METHOD_CREDIT_CARD, PAYMENT_METHOD_BANK_TRANSFER, PAYMENT_METHOD_CRYPTO

#[derive(Protto, PartialEq, Debug, Clone)]
pub enum PaymentMethod {
    CreditCard,
    BankTransfer,
    Crypto,
}

// --- Multi-word enum WITHOUT prefix convention (bare variants) ---
// Proto: FREE, TRIAL, PREMIUM

#[derive(Protto, PartialEq, Debug, Clone)]
pub enum SubscriptionTier {
    Free,
    Trial,
    Premium,
}

// === Category 1: Prefix computation tests ===

#[test]
fn payment_method_to_proto_roundtrip() {
    let cases = [
        (PaymentMethod::CreditCard, proto::PaymentMethod::CreditCard),
        (
            PaymentMethod::BankTransfer,
            proto::PaymentMethod::BankTransfer,
        ),
        (PaymentMethod::Crypto, proto::PaymentMethod::Crypto),
    ];

    for (rust_val, expected_proto) in cases {
        let proto_val: proto::PaymentMethod = rust_val.clone().into();
        assert_eq!(
            proto_val, expected_proto,
            "Rust {:?} should map to proto {:?}",
            rust_val, expected_proto
        );

        let back: PaymentMethod = proto_val.into();
        assert_eq!(
            back, rust_val,
            "Proto {:?} should map back to Rust {:?}",
            expected_proto, rust_val
        );
    }
}

#[test]
fn payment_method_i32_roundtrip() {
    let cases = [
        (0, PaymentMethod::CreditCard),
        (1, PaymentMethod::BankTransfer),
        (2, PaymentMethod::Crypto),
    ];

    for (i, expected) in cases {
        let method: PaymentMethod = i.into();
        assert_eq!(method, expected, "i32 {} should map to {:?}", i, expected);

        let back: i32 = method.into();
        assert_eq!(back, i, "{:?} should map back to i32 {}", expected, i);
    }
}

// === Category 2: Bare variant name matching tests ===

#[test]
fn subscription_tier_to_proto_roundtrip() {
    let cases = [
        (SubscriptionTier::Free, proto::SubscriptionTier::Free),
        (SubscriptionTier::Trial, proto::SubscriptionTier::Trial),
        (SubscriptionTier::Premium, proto::SubscriptionTier::Premium),
    ];

    for (rust_val, expected_proto) in cases {
        let proto_val: proto::SubscriptionTier = rust_val.clone().into();
        assert_eq!(
            proto_val, expected_proto,
            "Rust {:?} should map to proto {:?}",
            rust_val, expected_proto
        );

        let back: SubscriptionTier = proto_val.into();
        assert_eq!(
            back, rust_val,
            "Proto {:?} should map back to Rust {:?}",
            expected_proto, rust_val
        );
    }
}

#[test]
fn subscription_tier_i32_roundtrip() {
    let cases = [
        (0, SubscriptionTier::Free),
        (1, SubscriptionTier::Trial),
        (2, SubscriptionTier::Premium),
    ];

    for (i, expected) in cases {
        let tier: SubscriptionTier = i.into();
        assert_eq!(tier, expected, "i32 {} should map to {:?}", i, expected);

        let back: i32 = tier.into();
        assert_eq!(back, i, "{:?} should map back to i32 {}", expected, i);
    }
}

// === Category 3: Regression guards ===

#[test]
fn existing_another_status_bare_variant_roundtrip() {
    // AnotherStatus uses bare variants: OK, MOVED_PERMANENTLY, FOUND, NOT_FOUND
    // This was silently broken before the fix — no test existed.
    use crate::basic_types::AnotherStatus;

    let cases = [
        (AnotherStatus::Ok, proto::AnotherStatus::Ok),
        (
            AnotherStatus::MovedPermanently,
            proto::AnotherStatus::MovedPermanently,
        ),
        (AnotherStatus::Found, proto::AnotherStatus::Found),
        (AnotherStatus::NotFound, proto::AnotherStatus::NotFound),
    ];

    for (rust_val, expected_proto) in cases {
        let proto_val: proto::AnotherStatus = rust_val.clone().into();
        assert_eq!(proto_val, expected_proto);

        let back: AnotherStatus = proto_val.into();
        assert_eq!(back, rust_val);
    }
}

#[test]
fn existing_another_status_i32_roundtrip() {
    use crate::basic_types::AnotherStatus;

    let cases = [
        (0, AnotherStatus::Ok),
        (1, AnotherStatus::MovedPermanently),
        (2, AnotherStatus::Found),
        (3, AnotherStatus::NotFound),
    ];

    for (i, expected) in cases {
        let status: AnotherStatus = i.into();
        assert_eq!(status, expected);

        let back: i32 = status.into();
        assert_eq!(back, i);
    }
}

// === All variants covered (exhaustive) ===

#[test]
fn all_payment_method_variants_roundtrip_without_panic() {
    for i in 0..=2 {
        let method: PaymentMethod = i.into();
        let proto_val: proto::PaymentMethod = method.clone().into();
        let back: PaymentMethod = proto_val.into();
        assert_eq!(method, back);

        let i32_val: i32 = method.into();
        assert_eq!(i32_val, i);
    }
}

#[test]
fn all_subscription_tier_variants_roundtrip_without_panic() {
    for i in 0..=2 {
        let tier: SubscriptionTier = i.into();
        let proto_val: proto::SubscriptionTier = tier.clone().into();
        let back: SubscriptionTier = proto_val.into();
        assert_eq!(tier, back);

        let i32_val: i32 = tier.into();
        assert_eq!(i32_val, i);
    }
}

// === Negative tests: invalid i32 values panic ===

#[test]
#[should_panic(expected = "Unknown enum value")]
fn invalid_i32_panics_for_payment_method() {
    let _: PaymentMethod = 99.into();
}

#[test]
#[should_panic(expected = "Unknown enum value")]
fn invalid_i32_panics_for_subscription_tier() {
    let _: SubscriptionTier = 99.into();
}
