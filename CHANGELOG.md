
# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed

- Fixed handling of enums with the `allow_alias` option (see [#9]).

## [0.5.5] - 2022-02-01

### Added

- Added `MessageDescriptor::descriptor_proto` and friends to get the raw `prost_types` representation of protobuf definitions.

## [0.5.4] - 2022-02-01

### Changed

- Replace [`chrono`](https://crates.io/crates/chrono) dependency with [`time`](https://crates.io/crates/time) for parsing and formatting RFC 3339 timestamps when the `serde` feature is enabled. This avoids security vulnerabilities [RUSTSEC-2020-0071](https://rustsec.org/advisories/RUSTSEC-2020-0071) and [RUSTSEC-2020-0159](https://rustsec.org/advisories/RUSTSEC-2020-0159) which are not yet patched in `chrono`.

## [0.5.3] - 2022-01-18

### Fixed

- The `Cardinality` enum is now exported. Thanks to [@benesch] for [#4].

## [0.5.2] - 2022-01-09

### Changed

- `DynamicMessage` now stores all fields (normal, extensions, and unknown) in the same storage, reducing its footprint from 48 bytes to 40.

## [0.5.1] - 2022-01-05

### Added

- Added `DynamicMessage::get_field_mut` and friends for in-place modification of messages.

## [0.5.0] - 2022-01-05

### Added

- Extension fields are now decoded from their byte representation
- Added APIs to get extensions for a given message.

### Changed

- `DynamicMessage::get_extension` no longer returns an option.

## [0.4.0] - 2022-01-04

### Added

- New helper method `DynamicMessage::decode`.
- New APIs to get reserved names and fields for messages and enums
- New descriptor APIs to inspect extension fields
- New dynamic message APIs to get and set extension fields

### Changed

- Renamed `SerializeOptions::emit_unpopulated_fields` to `SerializeOptions::skip_default_fields` (note the meaning is inverted as well!).
- `DynamicMessage::{has_field,get_field,set_field,clear_field}` now take a field descriptor instead of a number. Use the  new `_by_number` variants for the old behavior.

### Fixed

- The library now passes the protobuf conformance tests. This uncovered a number of edge cases:

  - Fixed unpacked fields not accepting packed bytes
  - Unknown fields are now preserved and roundtripped.
  - Fixed default value for enums with negative values
  - When receiving multiple fields of a oneof in the byte representation, only the last is set.
  - Trailing zeros (e.g. `10.00`) are now accepted when parsing integers from the JSON representation.
  - Fixed handling of `null` in JSON deserialization.
  - Deserialization of `google.protobuf.NullValue` now accepts the `"NULL_VALUE"` string.
  - Deserialization of floats from JSON now validates the value is in range.
  - Nonzero trailing bits are allowed when deserializing JSON bytes from base64.
  - Serialization of `google.protobuf.FieldMask` fails if the path cannot be roundtripped through camelCase.
  - `google.protobuf.Duration` uses a different number of trailing zeroes depending on the precision of the input.
  - JSON serialization of `google.protobuf.Timestamp` and `google.protobuf.Duration` fails if it is outside the allowed range.
  - Duplicate oneof fields is now an error during JSON deserialization
  - Extensions are roundtripped though JSON format

## [0.3.5] - 2022-01-02

### Fixed

- Fixed deserialization of negative durations

## [0.3.4] - 2022-01-01

### Added

- Added `MessageDescriptor::map_entry_key_field` and `MessageDescriptor::map_entry_value_field` to help with inspecting message types.
- Added `Value::is_valid_for_field` to validate that value types are compatible with message fields.
- `DynamicMessage::set_value` now asserts that the value type is compatible with the field type.

### Fixed

- Fixed `FieldDescriptor::is_packed` returning true for non-repeated fields.

## [0.3.3] - 2021-12-30

### Added

- Added support for JSON mapping of `google.protobuf.Any`.

## [0.3.2] - 2021-12-29

### Added

- Builder methods on `SerializeOptions` and `DeserializeOptions` are now `const`.

### Fixed

- Fixed a case where serialization could produce invalid JSON.

## [0.3.1] - 2021-12-29

### Fixed

- Fixed docs for `ReflectMessage` macro.

## [0.3.0] - 2021-12-29

### Added

- Added `parent_message()` method to `MessageDescriptor` and `EnumDescriptor` to support inspecting the structure of nested types.
- Added `package_name()` method to `MessageDescriptor`, `EnumDescriptor` and `ServiceDescriptor` to determine which package they are defined in.
- Added `ReflectMessage` trait for types which can identify themselves with a `MessageDescriptor`.
- Added a derive macro which can be used as part of `prost_build` to generate `ReflectMessage` implementations.

### Changed

- Renamed `merge_from_message` to `transcode_from` and `to_message` to `transcode_to`.

## [0.2.0] - 2021-12-27

### Added

- Added support for serializing and deserializing with [serde](https://crates.io/crates/serde). By default the serialization format is the canonical [JSON encoding](https://developers.google.com/protocol-buffers/docs/proto3#json).

## [0.1.0] - 2021-12-26

### Added

- Initial release, including support for inspecting message types at runtime.

[Unreleased]: https://github.com/andrewhickman/prost-reflect/compare/0.5.5...HEAD
[0.5.5]: https://github.com/andrewhickman/prost-reflect/compare/0.5.4...0.5.5
[0.5.4]: https://github.com/andrewhickman/prost-reflect/compare/0.5.3...0.5.4
[0.5.3]: https://github.com/andrewhickman/prost-reflect/compare/0.5.2...0.5.3
[0.5.2]: https://github.com/andrewhickman/prost-reflect/compare/0.5.1...0.5.2
[0.5.1]: https://github.com/andrewhickman/prost-reflect/compare/0.5.0...0.5.1
[0.5.0]: https://github.com/andrewhickman/prost-reflect/compare/0.4.0...0.5.0
[0.4.0]: https://github.com/andrewhickman/prost-reflect/compare/0.3.4...0.4.0
[0.3.5]: https://github.com/andrewhickman/prost-reflect/compare/0.3.4...0.3.5
[0.3.4]: https://github.com/andrewhickman/prost-reflect/compare/0.3.3...0.3.4
[0.3.3]: https://github.com/andrewhickman/prost-reflect/compare/0.3.2...0.3.3
[0.3.2]: https://github.com/andrewhickman/prost-reflect/compare/0.3.1...0.3.2
[0.3.1]: https://github.com/andrewhickman/prost-reflect/compare/0.3.0...0.3.1
[0.3.0]: https://github.com/andrewhickman/prost-reflect/compare/0.2.0...0.3.0
[0.2.0]: https://github.com/andrewhickman/prost-reflect/compare/0.1.0...0.2.0
[0.1.0]: https://github.com/andrewhickman/prost-reflect/releases/tag/0.1.0

[@benesch]: https://github.com/benesch

[#4]: https://github.com/andrewhickman/prost-reflect/pull/4
[#9]: https://github.com/andrewhickman/prost-reflect/issues/9
