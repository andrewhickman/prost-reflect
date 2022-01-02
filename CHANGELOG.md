
# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

[unreleased]: https://github.com/andrewhickman/prost-reflect/compare/0.3.4...HEAD
[0.3.4]: https://github.com/andrewhickman/prost-reflect/compare/0.3.3...0.3.4
[0.3.3]: https://github.com/andrewhickman/prost-reflect/compare/0.3.2...0.3.3
[0.3.2]: https://github.com/andrewhickman/prost-reflect/compare/0.3.1...0.3.2
[0.3.1]: https://github.com/andrewhickman/prost-reflect/compare/0.3.0...0.3.1
[0.3.0]: https://github.com/andrewhickman/prost-reflect/compare/0.2.0...0.3.0
[0.2.0]: https://github.com/andrewhickman/prost-reflect/compare/0.1.0...0.2.0
[0.1.0]: https://github.com/andrewhickman/prost-reflect/releases/tag/0.1.0
