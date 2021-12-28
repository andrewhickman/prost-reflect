use prost::Message;

use crate::MessageDescriptor;

/// Trait for message types that support reflection.
pub trait ReflectMessage: Message {
    /// Gets a [`MessageDescriptor`] describing the type of this message.
    fn descriptor(&self) -> MessageDescriptor;
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
