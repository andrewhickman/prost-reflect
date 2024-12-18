mod wkt;

pub(crate) use wkt::make_wkt_descriptor_pool;

use prost::Message;

use crate::{DynamicMessage, MessageDescriptor};

/// Trait for message types that support reflection.
pub trait ReflectMessage: Message {
    /// Gets a [`MessageDescriptor`] describing the type of this message.
    fn descriptor(&self) -> MessageDescriptor;

    /// Converts this message into an instance of [`DynamicMessage`] by going
    /// through the byte representation.
    fn transcode_to_dynamic(&self) -> DynamicMessage
    where
        Self: Sized,
    {
        let mut message = DynamicMessage::new(self.descriptor());
        // This can only fail if `self.descriptor` returns a descriptor incompatible with the
        // actual serialized bytes.
        message
            .transcode_from(self)
            .expect("error converting to dynamic message");
        message
    }
}

impl<M> ReflectMessage for Box<M>
where
    M: ReflectMessage,
{
    fn descriptor(&self) -> MessageDescriptor {
        (**self).descriptor()
    }
}

#[test]
fn assert_object_safe() {
    fn _foo(_: Box<dyn ReflectMessage>) {}
}
