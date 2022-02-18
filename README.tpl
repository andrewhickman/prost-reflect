[![crates.io](https://img.shields.io/crates/v/prost-reflect.svg)](https://crates.io/crates/prost-reflect/)
[![docs.rs](https://docs.rs/prost-reflect/badge.svg)](https://docs.rs/prost-reflect/)
[![deps.rs](https://deps.rs/crate/prost-reflect/0.5.7/status.svg)](https://deps.rs/crate/prost-reflect)
![MSRV](https://img.shields.io/badge/rustc-1.54+-blue.svg)
[![Continuous integration](https://github.com/andrewhickman/prost-reflect/actions/workflows/ci.yml/badge.svg)](https://github.com/andrewhickman/prost-reflect/actions/workflows/ci.yml)
[![codecov.io](https://codecov.io/gh/andrewhickman/prost-reflect/branch/main/graph/badge.svg?token=E2OITYXO7M)](https://codecov.io/gh/andrewhickman/prost-reflect)
![Apache 2.0 OR MIT licensed](https://img.shields.io/badge/license-Apache2.0%2FMIT-blue.svg)

# prost-reflect

A protobuf library extending [`prost`](https://crates.io/crates/prost) with reflection support and dynamic messages.

## Usage

{{intro}}

### Example - decoding

{{decoding}}

### Example - JSON mapping

{{json}}

### Example - implementing `ReflectMessage`

{{reflect}}

## Minimum Supported Rust Version

Rust **1.54** or higher.

The minimum supported Rust version may be changed in the future, but it will be
done with a minor version bump.

## License

Licensed under either of

 * Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license
   ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.

[`FileDescriptor`]: https://docs.rs/prost-reflect/0.5.7/prost_reflect/struct.FileDescriptor.html
[`DynamicMessage`]: https://docs.rs/prost-reflect/0.5.7/prost_reflect/struct.DynamicMessage.html
[`MessageDescriptor`]: https://docs.rs/prost-reflect/0.5.7/prost_reflect/struct.MessageDescriptor.html
[`MessageDescriptor`]: https://docs.rs/prost-reflect/0.5.7/prost_reflect/struct.MessageDescriptor.html
[`DynamicMessage::decode`]: https://docs.rs/prost-reflect/0.5.7/prost_reflect/struct.DynamicMessage.html#method.decode
[`ReflectMessage`]: https://docs.rs/prost-reflect/0.5.7/prost_reflect/trait.ReflectMessage.html

[`Default`]: https://doc.rust-lang.org/stable/core/default/trait.Default.html
[prost_types::FileDescriptorSet]: https://docs.rs/prost-types/latest/prost_types/struct.FileDescriptorSet.html