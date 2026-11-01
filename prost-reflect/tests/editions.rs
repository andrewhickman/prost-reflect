//! Descriptors and expected bytes here come from `protoc`, see `data/editions/regenerate.sh`, so
//! each test checks a feature against what the reference compiler produces for it.

use prost::{bytes::Bytes, Message};
use prost_reflect::{
    Cardinality, DescriptorPool, DynamicMessage, FieldDescriptor, MessageDescriptor, Syntax, Value,
};

macro_rules! data {
    ($name:literal) => {
        include_bytes!(concat!("data/editions/", $name)).as_slice()
    };
}

fn pool(bytes: &[u8]) -> DescriptorPool {
    DescriptorPool::decode(bytes).unwrap()
}

fn message(pool: &DescriptorPool, name: &str) -> MessageDescriptor {
    pool.get_message_by_name(name).unwrap()
}

fn field(message: &MessageDescriptor, name: &str) -> FieldDescriptor {
    message.get_field_by_name(name).unwrap()
}

fn fields() -> MessageDescriptor {
    message(&pool(data!("features.binpb")), "editions.Fields")
}

#[test]
fn syntax_is_editions() {
    assert_eq!(fields().parent_file().syntax(), Syntax::Editions);
    let pool = pool(data!("edition_2024.binpb"));
    assert_eq!(
        message(&pool, "editions.e2024.Fields")
            .parent_file()
            .syntax(),
        Syntax::Editions
    );
}

#[test]
fn field_presence() {
    let m = fields();
    assert!(field(&m, "explicit_scalar").supports_presence());
    assert!(!field(&m, "implicit_scalar").supports_presence());
    assert!(field(&m, "required_scalar").supports_presence());
    assert!(field(&m, "prefixed").supports_presence());
    assert!(field(&m, "oneof_scalar").supports_presence());
    assert!(!field(&m, "packed").supports_presence());

    let pool = pool(data!("edition_2024.binpb"));
    let m = message(&pool, "editions.e2024.Fields");
    assert!(field(&m, "explicit_scalar").supports_presence());
    assert!(!field(&m, "implicit_scalar").supports_presence());
}

#[test]
fn legacy_required_is_required() {
    let m = fields();
    assert_eq!(
        field(&m, "required_scalar").cardinality(),
        Cardinality::Required
    );
    assert_eq!(
        field(&m, "explicit_scalar").cardinality(),
        Cardinality::Optional
    );
}

#[test]
fn repeated_field_encoding() {
    let m = fields();
    assert!(field(&m, "packed").is_packed());
    assert!(!field(&m, "expanded").is_packed());
}

#[test]
fn message_encoding() {
    let m = fields();
    assert!(field(&m, "delimited").is_group());
    assert!(field(&m, "delimited_list").is_group());
    assert!(!field(&m, "prefixed").is_group());
    assert!(!field(&m, "map_field").is_group());
    assert!(field(&m, "map_field").is_map());
}

#[test]
fn a_oneof_member_takes_its_own_features() {
    let m = fields();
    assert!(field(&m, "oneof_delimited").is_group());
    assert!(!field(&m, "oneof_prefixed").is_group());
}

#[test]
fn extensions_use_features() {
    let m = fields();
    let ext = |name: &str| {
        m.get_extension_by_full_name(&format!("editions.{name}"))
            .unwrap()
    };
    assert!(ext("ext_packed").is_packed());
    assert!(!ext("ext_expanded").is_packed());
    assert!(ext("ext_delimited").is_group());
    assert!(!ext("ext_scalar").is_group());
}

#[test]
fn file_features_are_inherited_and_a_field_overrides_them() {
    let pool = pool(data!("inherited.binpb"));
    let m = |name: &str| message(&pool, &format!("editions.inherited.{name}"));

    let file_level = m("FileLevel");
    assert!(!field(&file_level, "implicit_scalar").supports_presence());
    assert!(!field(&file_level, "expanded").is_packed());
    assert!(field(&file_level, "delimited").is_group());
    assert!(!field(&m("FileLevel.Nested"), "implicit_scalar").supports_presence());

    let overriding = m("Overriding");
    assert!(field(&overriding, "explicit_scalar").supports_presence());
    assert!(field(&overriding, "packed").is_packed());
    assert!(!field(&overriding, "prefixed").is_group());
}

#[test]
fn map_entries_stay_length_prefixed_when_delimited_is_inherited() {
    let pool = pool(data!("inherited.binpb"));
    let m = message(&pool, "editions.inherited.FileLevel");
    assert!(field(&m, "map_field").is_map());
    assert!(!field(&m, "map_field").is_group());
}

#[test]
fn closed_enum_may_start_at_a_non_zero_value() {
    let features = pool(data!("features.binpb"));
    assert!(features.get_enum_by_name("editions.ClosedEnum").is_some());
    let inherited = pool(data!("inherited.binpb"));
    assert!(inherited
        .get_enum_by_name("editions.inherited.InheritsClosed")
        .is_some());
}

#[test]
fn only_the_legacy_json_format_tolerates_conflicting_field_names() {
    let legacy = data!("legacy_json.binpb");
    pool(legacy);

    // Rewrites `json_format = LEGACY_BEST_EFFORT` to `ALLOW` in the message's options.
    let (legacy_option, allow_option) = ([0x62, 0x02, 0x30, 0x02], [0x62, 0x02, 0x30, 0x01]);
    let at = legacy
        .windows(4)
        .position(|w| w == legacy_option)
        .expect("the option is in the descriptor");
    let mut allow = legacy.to_vec();
    allow[at..at + 4].copy_from_slice(&allow_option);
    let err = DescriptorPool::decode(allow.as_slice()).unwrap_err();
    assert!(err.to_string().contains("foo_bar"), "{err}");
}

#[test]
fn decoding_and_reencoding_matches_protoc() {
    let m = fields();
    let expected = data!("values.binpb");
    let msg = DynamicMessage::decode(m.clone(), expected).unwrap();

    assert_eq!(
        msg.get_field_by_name("explicit_scalar").unwrap().as_i32(),
        Some(0)
    );
    assert!(msg.has_field(&field(&m, "explicit_scalar")));
    assert_eq!(
        msg.get_field_by_name("packed")
            .unwrap()
            .as_list()
            .unwrap()
            .len(),
        3
    );
    assert_eq!(
        msg.get_field_by_name("expanded")
            .unwrap()
            .as_list()
            .unwrap()
            .len(),
        3
    );
    let delimited = msg.get_field_by_name("delimited").unwrap();
    assert_eq!(
        delimited
            .as_message()
            .unwrap()
            .get_field_by_name("x")
            .unwrap()
            .as_i32(),
        Some(5)
    );
    assert_eq!(
        msg.get_field_by_name("delimited_list")
            .unwrap()
            .as_list()
            .unwrap()
            .len(),
        2
    );
    assert!(
        msg.unknown_fields().next().is_none(),
        "every field resolved"
    );
    assert_eq!(msg.encode_to_vec(), expected);
}

#[test]
fn encoding_from_scratch_matches_protoc() {
    let pool = pool(data!("inherited.binpb"));
    let m = message(&pool, "editions.inherited.FileLevel");
    let inner = message(&pool, "editions.inherited.Inner");
    let inner_with = |x: i32| {
        let mut v = DynamicMessage::new(inner.clone());
        v.set_field_by_name("x", Value::I32(x));
        Value::Message(v)
    };
    let mut msg = DynamicMessage::new(m.clone());
    msg.set_field_by_name("delimited", inner_with(2));
    msg.set_field_by_name(
        "map_field",
        Value::Map(
            [(prost_reflect::MapKey::String("k".into()), inner_with(1))]
                .into_iter()
                .collect(),
        ),
    );
    assert_eq!(msg.encode_to_vec(), data!("inherits_delimited.binpb"));
}

#[test]
fn implicit_presence_omits_a_default_and_explicit_keeps_it() {
    let m = fields();
    let mut msg = DynamicMessage::new(m.clone());
    msg.set_field_by_name("implicit_scalar", Value::I32(0));
    assert_eq!(msg.encode_to_vec(), Vec::<u8>::new());
    let mut msg = DynamicMessage::new(m);
    msg.set_field_by_name("explicit_scalar", Value::I32(0));
    assert_eq!(msg.encode_to_vec(), vec![0x08, 0x00]);
}

#[test]
fn an_unsupported_edition_is_an_error() {
    // Field 14 of each file, rewritten to EDITION_99999_TEST_ONLY.
    let set = prost_types::FileDescriptorSet::decode(data!("edition_2024.binpb")).unwrap();
    let mut raw = set.file[0].encode_to_vec();
    prost::encoding::int32::encode(14, &99_999, &mut raw);
    let err = DescriptorPool::new()
        .decode_file_descriptor_proto(Bytes::from(raw))
        .unwrap_err();
    assert!(err.to_string().contains("edition"), "{err}");
}

#[test]
fn an_editions_file_without_its_edition_is_an_error() {
    // `prost_types` has neither the edition nor features, so going through it loses them. That
    // must fail rather than read every field with the defaults.
    let set = prost_types::FileDescriptorSet::decode(data!("edition_2024.binpb")).unwrap();
    let err = DescriptorPool::new()
        .add_file_descriptor_set(set)
        .unwrap_err();
    assert!(err.to_string().contains("edition"), "{err}");
}
