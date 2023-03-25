use std::sync::Mutex;

use prost::bytes::Buf;
use prost_types::FileDescriptorProto;

use crate::{reflect::WELL_KNOWN_TYPES_BYTES, DescriptorError, DescriptorPool};

static INSTANCE: Mutex<Option<DescriptorPool>> = Mutex::new(None);

impl DescriptorPool {
    /// Gets a copy of the global descriptor pool. By default, this just contains the google well-known types.
    ///
    /// The global descriptor pool is typically used as a convenient place to store descriptors for `ReflectMessage` implementations.
    ///
    /// Note that modifications to the returned pool won't affect the global pool - use
    /// [`decode_global_file_descriptor_set`](decode_global_file_descriptor_set) or
    /// [`add_global_file_descriptor`](add_global_file_descriptor) to modify the global pool. Furthermore,
    /// modifications to the global pool won't be vis
    pub fn global() -> DescriptorPool {
        INSTANCE
            .lock()
            .unwrap()
            .get_or_insert_with(|| DescriptorPool::decode(WELL_KNOWN_TYPES_BYTES).unwrap())
            .clone()
    }

    /// Decodes and adds a set of file descriptors to the pool.
    ///
    /// See [`DescriptorPool::decode_file_descriptor_set`] for more details.
    pub fn decode_global_file_descriptor_set<B>(bytes: B) -> Result<(), DescriptorError>
    where
        B: Buf,
    {
        let mut instance = INSTANCE.lock().unwrap();
        let instance =
            instance.get_or_insert_with(|| DescriptorPool::decode(WELL_KNOWN_TYPES_BYTES).unwrap());
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
        let instance =
            instance.get_or_insert_with(|| DescriptorPool::decode(WELL_KNOWN_TYPES_BYTES).unwrap());
        instance.add_file_descriptor_proto(file)?;
        Ok(())
    }
}
