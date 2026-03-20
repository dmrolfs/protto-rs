# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.6.2] - 2026-03-19

### Fixed

- **Enum prefix for multi-word names**: `SubscriptionTier` now correctly produces
  the prefix `SUBSCRIPTION_TIER` instead of `SUBSCRIPTIONTIER`. Any enum with a
  multi-word PascalCase name was generating incorrect proto variant lookups.

- **Bare variant matching for prefix-free proto enums**: Proto enums that don't use
  the `ENUM_NAME_VARIANT` prefix convention (e.g., `FREE` instead of
  `SUBSCRIPTION_TIER_FREE`) now match correctly. The matching strategy tries
  prefixed, bare SCREAMING_SNAKE, and PascalCase candidates in both conversion
  directions.

- **`#[protto(proto_required)]` now works**: Previously this attribute was parsed
  but silently ignored during code generation, falling back to schema-based
  inference. It now correctly forces `Required` optionality, preventing erroneous
  `.expect()` calls on non-optional proto scalar fields (e.g., `i32` for enums).

### Added

- Unit tests for `to_screaming_snake_case` covering single-word, multi-word, and
  the known consecutive-uppercase limitation (e.g., `HTTPStatus` → `H_T_T_P_STATUS`).

- Integration tests for multi-word enum prefix matching (`PaymentMethod`,
  `SubscriptionTier`), required enum struct fields, `proto_required` attribute,
  and combined enum-in-struct roundtrips.

- Negative tests verifying that invalid `i32` values panic with clear messages.

- Regression test for `AnotherStatus` (prefix-free proto enum that was silently
  broken with no prior test coverage).

### Changed

- `RustToProtoStruct` test: corrected `#[protto(proto_required)]` to
  `#[protto(proto_optional)]` on a field that maps to an optional proto field.
  The annotation was previously a no-op and is now enforced.

## [0.6.1] - 2026-03-18

### Changed

- Updated Cargo.lock dependencies.
- Improved README for clarity and fixed errors.

## [0.6.0] - 2026-03-18

- Initial published release.
