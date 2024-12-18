use std::sync::Mutex;

use once_cell::sync::Lazy;
use prost::bytes::Buf;
use prost_types::FileDescriptorProto;

use crate::{DescriptorError, DescriptorPool};

static INSTANCE: Lazy<Mutex<DescriptorPool>> =
    Lazy::new(|| Mutex::new(crate::reflect::make_wkt_descriptor_pool().unwrap()));

impl DescriptorPool {
    /// Gets a copy of the global descriptor pool. By default, this just contains the google well-known types.
    ///
    /// The global descriptor pool is typically used as a convenient place to store descriptors for `ReflectMessage` implementations.
    ///
    /// Note that modifications to the returned pool won't affect the global pool - use
    /// [`decode_global_file_descriptor_set`](DescriptorPool::decode_global_file_descriptor_set) or
    /// [`add_global_file_descriptor_proto`](DescriptorPool::add_global_file_descriptor_proto) to modify the global pool.
    pub fn global() -> DescriptorPool {
        INSTANCE.lock().unwrap().clone()
    }

    /// Decodes and adds a set of file descriptors to the global pool.
    ///
    /// See [`DescriptorPool::decode_file_descriptor_set`] for more details.
    pub fn decode_global_file_descriptor_set<B>(bytes: B) -> Result<(), DescriptorError>
    where
        B: Buf,
    {
        let mut instance = INSTANCE.lock().unwrap();
        instance.decode_file_descriptor_set(bytes)?;
        Ok(())
    }

    /// Adds a single file descriptor to the global pool.
    ///
    /// See [`DescriptorPool::add_file_descriptor_proto`] for more details.
    pub fn add_global_file_descriptor_proto<B>(
        file: FileDescriptorProto,
    ) -> Result<(), DescriptorError>
    where
        B: Buf,
    {
        let mut instance = INSTANCE.lock().unwrap();
        instance.add_file_descriptor_proto(file)?;
        Ok(())
    }
}
