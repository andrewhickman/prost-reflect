use std::collections::HashMap;
use std::sync::RwLock;

use once_cell::sync::Lazy;

use crate::{DescriptorError, FileDescriptor};

#[doc(hidden)]
impl FileDescriptor {
    /// Get a shared [`FileDescriptor`], cached on the byte input.
    pub fn new_cached(bytes: &'static [u8]) -> Result<Self, DescriptorError> {
        type Cache = HashMap<usize, (&'static [u8], FileDescriptor)>;
        static CACHE: Lazy<RwLock<Cache>> = Lazy::new(Default::default);

        {
            let cache = CACHE.read().unwrap();

            // Fast path - look up by pointer equality. If the file descriptor set is included
            // in the binary (e.g. with include_bytes!()) we expect the compiler to deduplicate
            // any references to it.
            if let Some((_, desc)) = cache.get(&(bytes.as_ptr() as usize)) {
                return Ok(desc.clone());
            }

            // Fall back to comparing the whole byte slice
            for (cached_bytes, desc) in cache.values() {
                if bytes == *cached_bytes {
                    return Ok(desc.clone());
                }
            }
        }

        // The descriptor is not in the hashmap, build and add it.
        let mut cache = CACHE.write().unwrap();

        let desc = FileDescriptor::decode(bytes)?;
        cache.insert(bytes.as_ptr() as usize, (bytes, desc.clone()));
        Ok(desc)
    }
}
