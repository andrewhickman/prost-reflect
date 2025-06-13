//! `prost-reflect-build` contains [`Builder`] to configure [`prost_build::Config`]
//! to derive [`prost_reflect::ReflectMessage`] for all messages in protocol buffers.
//!
//! The simplest way to generate [`prost_reflect::ReflectMessage`] is:
//!
//! ```no_run
//! // build.rs
//! use prost_reflect_build::Builder;
//!
//! Builder::new()
//!     .descriptor_pool("crate::DESCRIPTOR_POOL")
//!     .compile_protos(&["path/to/protobuf.proto"], &["path/to/include"])
//!     .expect("Failed to compile protos");
//! ```
//!
//! Either [`Builder::descriptor_pool`] or [`Builder::file_descriptor_set_bytes`] must be set to an expression giving the implementation access to descriptors.
//! For example when using `descriptor_pool` a static instance of [`DescriptorPool`] must be available:
//!
//! ```ignore
//! static DESCRIPTOR_POOL: Lazy<DescriptorPool> = Lazy::new(|| DescriptorPool::decode(
//!     include_bytes!(concat!(env!("OUT_DIR"), "file_descriptor_set.bin")).as_ref()
//! ).unwrap());
//!
//! // `include!` generated code may appear anywhere in the crate.
//! include!(concat!(env!("OUT_DIR"), "protobuf.rs"));
//! ```
#![warn(missing_debug_implementations, missing_docs)]

use std::{
    env, fs, io,
    path::{Path, PathBuf},
};

use prost_reflect::DescriptorPool;

/// Configuration builder for prost-reflect code generation.
///
/// The simplest way to generate prost APIs deriving [`prost_reflect::ReflectMessage`]:
///
/// ```no_run
/// # use prost_reflect_build::Builder;
/// Builder::new()
///     .compile_protos(&["path/to/protobuf.proto"], &["path/to/include"])
///     .unwrap();
/// ```
#[derive(Debug, Clone)]
pub struct Builder {
    file_descriptor_set_path: PathBuf,
    descriptor_pool_expr: Option<String>,
    file_descriptor_set_bytes_expr: Option<String>,
}

impl Default for Builder {
    fn default() -> Self {
        let file_descriptor_set_path = env::var_os("OUT_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."))
            .join("file_descriptor_set.bin");

        Self {
            file_descriptor_set_path,
            descriptor_pool_expr: None,
            file_descriptor_set_bytes_expr: None,
        }
    }
}

impl Builder {
    /// Create a new builder with default parameters.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the path where the encoded file descriptor set is created.
    /// By default, it is created at `$OUT_DIR/file_descriptor_set.bin`.
    ///
    /// This overrides the path specified by
    /// [`prost_build::Config::file_descriptor_set_path`].
    pub fn file_descriptor_set_path<P>(&mut self, path: P) -> &mut Self
    where
        P: Into<PathBuf>,
    {
        self.file_descriptor_set_path = path.into();
        self
    }

    /// Set the file descriptor expression for reflection.
    ///
    /// This should resolve to an instance of `DescriptorPool`. For example, if this
    /// value is set to `crate::DescriptorPool`, then `lib.rs` should contain the following
    ///
    /// ```ignore
    /// static DESCRIPTOR_POOL: Lazy<DescriptorPool> = Lazy::new(||
    ///     DescriptorPool::decode(include_bytes!(
    ///         concat!(env!("OUT_DIR"), "/file_descriptor_set.bin")
    ///     ).as_ref()).unwrap()
    /// );
    /// ```
    pub fn descriptor_pool<P>(&mut self, expr: P) -> &mut Self
    where
        P: Into<String>,
    {
        self.descriptor_pool_expr = Some(expr.into());
        self
    }

    /// Set the file descriptor bytes to use for reflection.
    ///
    /// This should typically be the contents of the file at `file_descriptor_set_path`. For example,
    /// if this value is set to `crate::FILE_DESCRIPTOR_SET_BYTES`, then `lib.rs` should contain the following
    ///
    /// ```ignore
    /// const FILE_DESCRIPTOR_SET_BYTES: &'static [u8] = include_bytes!(concat!(env!("OUT_DIR"), "/file_descriptor_set.bin"));
    /// ```
    pub fn file_descriptor_set_bytes<P>(&mut self, expr: P) -> &mut Self
    where
        P: Into<String>,
    {
        self.file_descriptor_set_bytes_expr = Some(expr.into());
        self
    }

    /// Configure `config` to derive [`prost_reflect::ReflectMessage`] for all messages included in `protos`.
    /// This method does not generate prost-reflect compatible code,
    /// but `config` may be used later to compile protocol buffers independently of [`Builder`].
    /// `protos` and `includes` should be the same when [`prost_build::Config::compile_protos`] is called on `config`.
    ///
    /// ```ignore
    /// let mut config = Config::new();
    ///
    /// // Customize config here
    ///
    /// Builder::new()
    ///     .configure(&mut config, &["path/to/protobuf.proto"], &["path/to/include"])
    ///     .expect("Failed to configure for reflection");
    ///
    /// // Custom compilation process with `config`
    /// config.compile_protos(&["path/to/protobuf.proto"], &["path/to/includes"])
    ///     .expect("Failed to compile protocol buffers");
    /// ```
    pub fn configure(
        &mut self,
        config: &mut prost_build::Config,
        protos: &[impl AsRef<Path>],
        includes: &[impl AsRef<Path>],
    ) -> io::Result<()> {
        config
            .file_descriptor_set_path(&self.file_descriptor_set_path)
            .compile_protos(protos, includes)?;

        let buf = fs::read(&self.file_descriptor_set_path)?;
        let descriptor = DescriptorPool::decode(buf.as_ref()).expect("Invalid file descriptor");

        let pool_attribute = if let Some(descriptor_pool) = &self.descriptor_pool_expr {
            format!(
                r#"#[prost_reflect(descriptor_pool = "{}")]"#,
                descriptor_pool,
            )
        } else if let Some(file_descriptor_set_bytes) = &self.file_descriptor_set_bytes_expr {
            format!(
                r#"#[prost_reflect(file_descriptor_set_bytes = "{}")]"#,
                file_descriptor_set_bytes,
            )
        } else {
            return Err(io::Error::other(
                "either 'descriptor_pool' or 'file_descriptor_set_bytes' must be set",
            ));
        };

        for message in descriptor.all_messages() {
            let full_name = message.full_name();
            config
                .type_attribute(full_name, "#[derive(::prost_reflect::ReflectMessage)]")
                .type_attribute(
                    full_name,
                    format!(r#"#[prost_reflect(message_name = "{}")]"#, full_name,),
                )
                .type_attribute(full_name, &pool_attribute);
        }

        Ok(())
    }

    /// Compile protocol buffers into Rust with given [`prost_build::Config`].
    pub fn compile_protos_with_config(
        &mut self,
        mut config: prost_build::Config,
        protos: &[impl AsRef<Path>],
        includes: &[impl AsRef<Path>],
    ) -> io::Result<()> {
        self.configure(&mut config, protos, includes)?;

        config.skip_protoc_run().compile_protos(protos, includes)
    }

    /// Compile protocol buffers into Rust.
    pub fn compile_protos(
        &mut self,
        protos: &[impl AsRef<Path>],
        includes: &[impl AsRef<Path>],
    ) -> io::Result<()> {
        self.compile_protos_with_config(prost_build::Config::new(), protos, includes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config() {
        let mut config = prost_build::Config::new();
        let mut builder = Builder::new();
        let tmpdir = std::env::temp_dir();
        config.out_dir(tmpdir.clone());

        builder
            .file_descriptor_set_path(tmpdir.join("file_descriptor_set.bin"))
            .descriptor_pool("crate::DESCRIPTOR_POOL")
            .compile_protos_with_config(config, &["src/test.proto"], &["src"])
            .unwrap();

        assert!(tmpdir.join("my.test.rs").exists());

        let buf = fs::read_to_string(tmpdir.join("my.test.rs")).unwrap();
        let num_derive = buf
            .lines()
            .filter(|line| line.trim_start() == "#[derive(::prost_reflect::ReflectMessage)]")
            .count();

        assert_eq!(num_derive, 3);
    }
}
