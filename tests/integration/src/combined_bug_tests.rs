// ABOUTME: Tests exercising both enum prefix and optionality bugs simultaneously.
// ABOUTME: Real-world scenario: multi-word enum as a struct field on required proto i32.

use crate::proto;
use protto::Protto;

// Enums defined locally to avoid registry ordering dependency across modules.
// The proc-macro enum registry is populated during compilation, so the enums
// must be derived before structs that use them as fields.

#[derive(Protto, PartialEq, Debug, Clone)]
pub enum SubscriptionTier {
    Free,
    Trial,
    Premium,
}

#[derive(Protto, PartialEq, Debug, Clone)]
pub enum PaymentMethod {
    CreditCard,
    BankTransfer,
    Crypto,
}

// === Category 7: Combined bug tests ===

#[derive(Protto, PartialEq, Debug, Clone)]
#[protto(proto_name = "SubscriptionMessage")]
pub struct Subscription {
    #[protto(proto_name = "tier")]
    pub tier: SubscriptionTier,
    #[protto(proto_name = "payment")]
    pub payment: PaymentMethod,
    pub active: bool,
    pub name: String,
}

#[test]
fn full_subscription_roundtrip() {
    let original = Subscription {
        tier: SubscriptionTier::Premium,
        payment: PaymentMethod::CreditCard,
        active: true,
        name: "Pro Plan".to_string(),
    };

    let proto_msg: proto::SubscriptionMessage = original.clone().into();
    assert_eq!(proto_msg.tier, proto::SubscriptionTier::Premium as i32);
    assert_eq!(proto_msg.payment, proto::PaymentMethod::CreditCard as i32);
    assert_eq!(proto_msg.active, true);
    assert_eq!(proto_msg.name, "Pro Plan");

    let roundtrip: Subscription = proto_msg.into();
    assert_eq!(original, roundtrip);
}

#[test]
fn subscription_all_variants() {
    let tiers = [
        SubscriptionTier::Free,
        SubscriptionTier::Trial,
        SubscriptionTier::Premium,
    ];
    let methods = [
        PaymentMethod::CreditCard,
        PaymentMethod::BankTransfer,
        PaymentMethod::Crypto,
    ];

    for tier in &tiers {
        for method in &methods {
            let sub = Subscription {
                tier: tier.clone(),
                payment: method.clone(),
                active: false,
                name: String::new(),
            };

            let proto_msg: proto::SubscriptionMessage = sub.clone().into();
            let roundtrip: Subscription = proto_msg.into();
            assert_eq!(
                sub, roundtrip,
                "Failed for tier={:?} method={:?}",
                tier, method
            );
        }
    }
}

#[test]
fn subscription_default_proto_values() {
    let proto_msg = proto::SubscriptionMessage::default();

    let converted: Subscription = proto_msg.into();
    assert_eq!(converted.tier, SubscriptionTier::Free); // 0 → Free
    assert_eq!(converted.payment, PaymentMethod::CreditCard); // 0 → CreditCard
    assert_eq!(converted.active, false);
    assert_eq!(converted.name, "");
}
