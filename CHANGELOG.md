
# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- Updated logos dependency to [0.13.0](https://github.com/maciejhirsz/logos/releases/tag/v0.13).

## [0.11.2] - 2023-04-09

### Changed

- Adjusted the `Debug` implementation for `DescriptorError` to be more concise and readable.

### Fixed

- Fixed parsing of group fields from text format. The field name must now match the type name of the group field.

## [0.11.1] - 2023-04-05

### Added

- Added [`Kind::wire_type`](https://docs.rs/prost-reflect/latest/prost_reflect/enum.Kind.html#method.wire_type). Thanks to [@slinkydeveloper] for [#34]

## [0.11.0] - 2023-03-27

### Added

- Added a global descriptor pool which can be fetched using [`DescriptorPool::global()`](https://docs.rs/prost-reflect/latest/prost_reflect/struct.DescriptorPool.html#method.global). By default it just contains well-known types, but additional files can be added using [`DescriptorPool::decode_global_file_descriptor_set()`](https://docs.rs/prost-reflect/latest/prost_reflect/struct.DescriptorPool.html#method.decode_global_file_descriptor_set) and [`DescriptorPool::add_global_file_descriptor_proto()`](https://docs.rs/prost-reflect/latest/prost_reflect/struct.DescriptorPool.html#method.add_global_file_descriptor_proto). ([#13])
- *prost-reflect-derive* Added the `file_descriptor_set_bytes` attribute as an alternative to `descriptor_pool`, which automatically registers the file with the global pool.
- *prost-reflect-build* Added `Builder::file_descriptor_pool_bytes` to set the `file_descriptor_set_bytes` derive attribute.

### Changed

- Duplicate files are now always ignored when adding to a `DescriptorPool` (previously, the code would skip files with identical contents, but now it skips any file with the same name).
- *prost-reflect-derive* Update syn requirement from 1.0.84 to 2.0.3
- *prost-reflect-build* **Breaking** Renamed `Builder::file_descriptor_expr` to `Builder::descriptor_pool`.
- *prost-reflect-build* **Breaking** Removed the default behaviour of looking for the file descriptor under `crate::DESCRIPTOR_POOL`. One of `descriptor_pool` or `file_descriptor_pool_bytes` must be set explicitly.

## [0.10.3] - 2023-03-20

### Fixed

- Fixed type resolution for double fields. Thanks to [@jackkleeman] for [#29]

## [0.10.2] - 2023-02-17

### Changed

- Updated the base64 dependency to version [0.21.0](https://crates.io/crates/base64/0.21.0).
- If the `json_name` property for a field is unset, it will now be populated with the camel-cased field name ([#5](https://github.com/andrewhickman/prost-reflect/issues/5#issuecomment-1432230706)).

## [0.10.1] - 2023-01-07

### Fixed

- The path for repeated extension options now includes the array index (for consistency with the output of protoc).

## [0.10.0] - 2023-01-04

### Added

- Added the [`path()`](https://docs.rs/prost-reflect/latest/prost_reflect/struct.MessageDescriptor.html#method.path) method to all descriptor types, which returns a path that can be used to get source code info by comparing against [`Location::path`](https://docs.rs/prost-types/latest/prost_types/source_code_info/struct.Location.html#structfield.path).
- Added the [`options()`](https://docs.rs/prost-reflect/latest/prost_reflect/struct.MessageDescriptor.html#method.options) method to all descriptor types, which returns a message containing the options defined for the descriptor, including custom options.
- The `uninterpreted_options` field of options is now used to populate options if it is present.
  - Note that if the `text-format` feature flag is not enabled, then options set through the [`aggregate_value`](https://docs.rs/prost-types/latest/prost_types/struct.UninterpretedOption.html#method.aggregate_value) field will be ignored.
- Added several new validation checks when constructing a `DescriptorPool` instance.
- Added new [`file()`](https://docs.rs/prost-reflect/latest/prost_reflect/struct.DescriptorError.html#method.file), [`line()`](https://docs.rs/prost-reflect/latest/prost_reflect/struct.DescriptorError.html#method.line) and [`column()`](https://docs.rs/prost-reflect/latest/prost_reflect/struct.DescriptorError.html#method.column) methods to get more context information about errors.
- When the `miette` feature is enabled, `DescriptorError` now implements [`Diagnostic`](https://docs.rs/miette/latest/miette/trait.Diagnostic.html). When source code is provided through [`DescriptorError::with_source_code`](https://docs.rs/prost-reflect/latest/prost_reflect/struct.DescriptorError.html#method.with_source_code), and span information is provided in [`FileDescriptorProto::source_code_info`](https://docs.rs/prost-types/latest/prost_types/struct.FileDescriptorProto.html#structfield.source_code_info), then the error will have labels annotating relevant portions of the source.

### Changed

- The minimum supported rust version is now **1.60.0**.
- **Breaking**: The [`FileDescriptor::dependencies`](https://docs.rs/prost-reflect/latest/prost_reflect/struct.FileDescriptor.html#method.dependencies) now returns all imported files, instead of just those imported with `import public`. The new `public_dependencies` method preserves the old behaviour ([#19]).
- **Breaking**: The `reflect-well-known-types` feature flag has been removed, and the functionality is now always available.
- Updated the base64 dependency to version [0.20.0](https://crates.io/crates/base64/0.20.0).

## [0.9.2] - 2022-08-14

### Added

- Added support for parsing and formatting dynamic messages using the text format. This functionality is enabled with the new `text_format` feature flag. See [`DynamicMessage::parse_text_format`](https://docs.rs/prost-reflect/latest/prost_reflect/struct.DynamicMessage.html#method.parse_text_format) and [`DynamicMessage::to_text_format`](https://docs.rs/prost-reflect/latest/prost_reflect/struct.DynamicMessage.html#method.to_text_format).
- Added a `Display` implementation for `DynamicMessage` and `Value` which uses the text format. This is available even when the `text_format` feature is disabled.
- Added new methods for setting dynamic message fields without panicking: [`try_set_field`](https://docs.rs/prost-reflect/latest/prost_reflect/struct.DynamicMessage.html#method.try_set_field), [`try_set_field_by_number`](https://docs.rs/prost-reflect/latest/prost_reflect/struct.DynamicMessage.html#method.try_set_field_by_number) and [`try_set_field_by_name`](https://docs.rs/prost-reflect/latest/prost_reflect/struct.DynamicMessage.html#method.try_set_field_by_name)
- Added [`Value::into_map_key()`](https://docs.rs/prost-reflect/latest/prost_reflect/enum.Value.html#method.into_map_key)

## [0.9.1] - 2022-08-01

### Fixed

- Fixed docs build

## [0.9.0] - 2022-07-30

### Added

- Added `MessageDescriptor::get_extension_by_full_name()`.

### Changed

- Updated to prost [**0.11.0**](https://github.com/tokio-rs/prost/releases/tag/v0.11.0)
- When the `serde` feature is enabled, the functions in `prost-types` for parsing and formatting time types are now used. This removes the `time` dependency.
- The minimum supported rust version is now **1.56.0**.

## [0.8.1] - 2022-05-29

### Added

- Added the `FileDescriptor` API for inspecting individual protobuf files.
- Added methods to `MessageDescriptor` to get child messages, enums and extensions.

## [0.8.0] - 2022-05-09

### Added

- `DescriptorPool` (formerly `FileDescriptor`) now supports adding individual `FileDescriptorProto` instances ([#6](https://github.com/andrewhickman/prost-reflect/issues/6)).

### Changed

- **Breaking**: `FileDescriptor` has been renamed to `DescriptorPool`. (The name `FileDescriptor` may be used in a future release to provide an API for inspecting individual source files)
  - `FileDescriptor::new` has been renamed to `DescriptorPool::from_file_descriptor_set`.
  - `FileDescriptor::file_descriptor_set` has been replaced by `DescriptorPool::file_descriptor_protos` to allow for it containing multiple sets of descriptors.
  - The `parent_file` method on all descriptor types has been renamed to `parent_pool`.
  - The `file_descriptor` parameter of the `ReflectMessage` derive macro has been renamed to `descriptor_pool`.
  - The default value of the descriptor pool expression for `prost-reflect-build` is changed from `crate::FILE_DESCRIPTOR` to `crate::DESCRIPTOR_POOL`.

## [0.7.0] - 2022-04-03

### Changed

- Updated to version [`0.10.0`](https://crates.io/crates/prost) of prost.

## [0.6.1] - 2022-02-27

### Added

- The public dependencies `prost`, `prost-types` and `bytes` are now re-exported from the crate root.

## [0.6.0] - 2022-02-20

### Added

- Added implementations of [`ReflectMessage`] for the google well-known types in [`prost-types`](https://docs.rs/prost-types/0.9.0/prost_types), behind a feature flag `reflect-well-known-types`.

### Changed

- The minimum supported rust version is now **1.54.0**.

## [0.5.7] - 2022-02-14

### Added

- Added `file_descriptor_proto` methods to descriptor types to access raw details about the file the object is defined in.

## [0.5.6] - 2022-02-03

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

[Unreleased]: https://github.com/andrewhickman/prost-reflect/compare/0.11.2...HEAD
[0.11.2]: https://github.com/andrewhickman/prost-reflect/compare/0.11.1...0.11.2
[0.11.1]: https://github.com/andrewhickman/prost-reflect/compare/0.11.0...0.11.1
[0.11.0]: https://github.com/andrewhickman/prost-reflect/compare/0.10.3...0.11.0
[0.10.3]: https://github.com/andrewhickman/prost-reflect/compare/0.10.2...0.10.3
[0.10.2]: https://github.com/andrewhickman/prost-reflect/compare/0.10.1...0.10.2
[0.10.1]: https://github.com/andrewhickman/prost-reflect/compare/0.10.0...0.10.1
[0.10.0]: https://github.com/andrewhickman/prost-reflect/compare/0.9.0...0.10.0
[0.9.2]: https://github.com/andrewhickman/prost-reflect/compare/0.9.1...0.9.2
[0.9.1]: https://github.com/andrewhickman/prost-reflect/compare/0.9.0...0.9.1
[0.9.0]: https://github.com/andrewhickman/prost-reflect/compare/0.8.1...0.9.0
[0.8.1]: https://github.com/andrewhickman/prost-reflect/compare/0.8.0...0.8.1
[0.8.0]: https://github.com/andrewhickman/prost-reflect/compare/0.7.0...0.8.0
[0.7.0]: https://github.com/andrewhickman/prost-reflect/compare/0.6.1...0.7.0
[0.6.1]: https://github.com/andrewhickman/prost-reflect/compare/0.6.0...0.6.1
[0.6.0]: https://github.com/andrewhickman/prost-reflect/compare/0.5.7...0.6.0
[0.5.7]: https://github.com/andrewhickman/prost-reflect/compare/0.5.6...0.5.7
[0.5.6]: https://github.com/andrewhickman/prost-reflect/compare/0.5.5...0.5.6
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
[@jackkleeman]: https://github.com/jackkleeman
[@slinkydeveloper]: https://github.com/slinkydeveloper

[#4]: https://github.com/andrewhickman/prost-reflect/pull/4
[#9]: https://github.com/andrewhickman/prost-reflect/issues/9
[#13]: https://github.com/andrewhickman/prost-reflect/issues/13
[#19]: https://github.com/andrewhickman/prost-reflect/issues/19
[#29]: https://github.com/andrewhickman/prost-reflect/issues/29
[#34]: https://github.com/andrewhickman/prost-reflect/pull/34
