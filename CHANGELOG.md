
# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Builder methods on `SerializeOptions` and `DeserializeOptions` are now `const`.

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

[unreleased]: https://github.com/andrewhickman/prost-reflect/compare/0.3.1...HEAD
[0.3.1]: https://github.com/andrewhickman/prost-reflect/compare/0.3.0...0.3.1
[0.3.0]: https://github.com/andrewhickman/prost-reflect/compare/0.2.0...0.3.0
[0.2.0]: https://github.com/andrewhickman/prost-reflect/compare/0.1.0...0.2.0
[0.1.0]: https://github.com/andrewhickman/prost-reflect/releases/tag/0.1.0
