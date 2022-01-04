
# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Added a new helper method `DynamicMessage::decode`.
- Added new APIs to get reserved names and fields for messages and enums
- Added new APIs to inspect extension fields

### Changed

- Renamed `SerializeOptions::emit_unpopulated_fields` to `SerializeOptions::skip_default_fields` (note the meaning is inverted as well!).

### Fixed

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

[unreleased]: https://github.com/andrewhickman/prost-reflect/compare/0.3.5...HEAD
[0.3.5]: https://github.com/andrewhickman/prost-reflect/compare/0.3.4...0.3.5
[0.3.4]: https://github.com/andrewhickman/prost-reflect/compare/0.3.3...0.3.4
[0.3.3]: https://github.com/andrewhickman/prost-reflect/compare/0.3.2...0.3.3
[0.3.2]: https://github.com/andrewhickman/prost-reflect/compare/0.3.1...0.3.2
[0.3.1]: https://github.com/andrewhickman/prost-reflect/compare/0.3.0...0.3.1
[0.3.0]: https://github.com/andrewhickman/prost-reflect/compare/0.2.0...0.3.0
[0.2.0]: https://github.com/andrewhickman/prost-reflect/compare/0.1.0...0.2.0
[0.1.0]: https://github.com/andrewhickman/prost-reflect/releases/tag/0.1.0
