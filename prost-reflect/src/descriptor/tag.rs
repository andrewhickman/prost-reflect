#![allow(dead_code)]

pub(crate) const UNINTERPRETED_OPTION: i32 = 999;

pub(crate) mod file {
    pub(crate) const PACKAGE: i32 = 2;
    pub(crate) const DEPENDENCY: i32 = 3;
    pub(crate) const PUBLIC_DEPENDENCY: i32 = 10;
    pub(crate) const WEAK_DEPENDENCY: i32 = 11;
    pub(crate) const MESSAGE_TYPE: i32 = 4;
    pub(crate) const ENUM_TYPE: i32 = 5;
    pub(crate) const SERVICE: i32 = 6;
    pub(crate) const EXTENSION: i32 = 7;
    pub(crate) const OPTIONS: i32 = 8;
    pub(crate) const SYNTAX: i32 = 12;

    pub(crate) mod options {
        pub(crate) const JAVA_PACKAGE: i32 = 1;
        pub(crate) const JAVA_OUTER_CLASSNAME: i32 = 8;
        pub(crate) const JAVA_MULTIPLE_FILES: i32 = 10;
        pub(crate) const JAVA_GENERATE_EQUALS_AND_HASH: i32 = 20;
        pub(crate) const JAVA_STRING_CHECK_UTF8: i32 = 27;
        pub(crate) const OPTIMIZE_FOR: i32 = 9;
        pub(crate) const GO_PACKAGE: i32 = 11;
        pub(crate) const CC_GENERIC_SERVICES: i32 = 16;
        pub(crate) const JAVA_GENERIC_SERVICES: i32 = 17;
        pub(crate) const PY_GENERIC_SERVICES: i32 = 18;
        pub(crate) const PHP_GENERIC_SERVICES: i32 = 42;
        pub(crate) const DEPRECATED: i32 = 23;
        pub(crate) const CC_ENABLE_ARENAS: i32 = 31;
        pub(crate) const OBJC_CLASS_PREFIX: i32 = 36;
        pub(crate) const CSHARP_NAMESPACE: i32 = 37;
        pub(crate) const SWIFT_PREFIX: i32 = 39;
        pub(crate) const PHP_CLASS_PREFIX: i32 = 40;
        pub(crate) const PHP_NAMESPACE: i32 = 41;
        pub(crate) const PHP_METADATA_NAMESPACE: i32 = 44;
        pub(crate) const RUBY_PACKAGE: i32 = 45;
        pub(crate) const FILE_UNINTERPRETED_OPTION: i32 = 999;
    }
}

pub(crate) mod message {
    pub(crate) const NAME: i32 = 1;
    pub(crate) const FIELD: i32 = 2;
    pub(crate) const EXTENSION: i32 = 6;
    pub(crate) const NESTED_TYPE: i32 = 3;
    pub(crate) const ENUM_TYPE: i32 = 4;
    pub(crate) const EXTENSION_RANGE: i32 = 5;
    pub(crate) const OPTIONS: i32 = 7;
    pub(crate) const ONEOF_DECL: i32 = 8;
    pub(crate) const RESERVED_RANGE: i32 = 9;
    pub(crate) const RESERVED_NAME: i32 = 10;

    pub(crate) mod extension_range {
        pub(crate) const START: i32 = 1;
        pub(crate) const END: i32 = 2;
        pub(crate) const OPTIONS: i32 = 3;
    }

    pub(crate) mod reserved_range {
        pub(crate) const START: i32 = 1;
        pub(crate) const END: i32 = 2;
    }

    pub(crate) mod options {
        pub(crate) const MESSAGE_SET_WIRE_FORMAT: i32 = 1;
        pub(crate) const NO_STANDARD_DESCRIPTOR_ACCESSOR: i32 = 2;
        pub(crate) const DEPRECATED: i32 = 3;
        pub(crate) const MAP_ENTRY: i32 = 7;
        pub(crate) const UNINTERPRETED_OPTION: i32 = 999;
    }
}

pub(crate) mod field {
    pub(crate) const NAME: i32 = 1;
    pub(crate) const EXTENDEE: i32 = 2;
    pub(crate) const NUMBER: i32 = 3;
    pub(crate) const LABEL: i32 = 4;
    pub(crate) const TYPE: i32 = 5;
    pub(crate) const TYPE_NAME: i32 = 6;
    pub(crate) const DEFAULT_VALUE: i32 = 7;
    pub(crate) const JSON_NAME: i32 = 10;
    pub(crate) const OPTIONS: i32 = 8;

    pub(crate) mod options {
        pub(crate) const CTYPE: i32 = 1;
        pub(crate) const PACKED: i32 = 2;
        pub(crate) const JSTYPE: i32 = 6;
        pub(crate) const LAZY: i32 = 5;
        pub(crate) const DEPRECATED: i32 = 3;
        pub(crate) const WEAK: i32 = 10;
        pub(crate) const UNINTERPRETED_OPTION: i32 = 999;
    }
}

pub(crate) mod oneof {
    pub(crate) const NAME: i32 = 1;
    pub(crate) const OPTIONS: i32 = 2;
}

pub(crate) mod enum_ {
    pub(crate) const NAME: i32 = 1;
    pub(crate) const VALUE: i32 = 2;
    pub(crate) const OPTIONS: i32 = 3;
    pub(crate) const RESERVED_RANGE: i32 = 4;
    pub(crate) const RESERVED_NAME: i32 = 5;

    pub(crate) mod reserved_range {
        pub(crate) const START: i32 = 1;
        pub(crate) const END: i32 = 2;
    }

    pub(crate) mod options {
        pub(crate) const ALLOW_ALIAS: i32 = 2;
        pub(crate) const DEPRECATED: i32 = 3;
        pub(crate) const UNINTERPRETED_OPTION: i32 = 999;
    }
}

pub(crate) mod enum_value {
    pub(crate) const NAME: i32 = 1;
    pub(crate) const NUMBER: i32 = 2;
    pub(crate) const OPTIONS: i32 = 3;

    pub(crate) mod options {
        pub(crate) const DEPRECATED: i32 = 1;
        pub(crate) const UNINTERPRETED_OPTION: i32 = 999;
    }
}

pub(crate) mod service {
    pub(crate) const NAME: i32 = 1;
    pub(crate) const METHOD: i32 = 2;
    pub(crate) const OPTIONS: i32 = 3;

    pub(crate) mod options {
        pub(crate) const DEPRECATED: i32 = 33;
        pub(crate) const UNINTERPRETED_OPTION: i32 = 999;
    }
}

pub(crate) mod method {
    pub(crate) const NAME: i32 = 1;
    pub(crate) const INPUT_TYPE: i32 = 2;
    pub(crate) const OUTPUT_TYPE: i32 = 3;
    pub(crate) const OPTIONS: i32 = 4;
    pub(crate) const CLIENT_STREAMING: i32 = 5;
    pub(crate) const SERVER_STREAMING: i32 = 6;

    pub(crate) mod options {
        pub(crate) const DEPRECATED: i32 = 33;
        pub(crate) const IDEMPOTENCY_LEVEL: i32 = 34;
        pub(crate) const UNINTERPRETED_OPTION: i32 = 999;
    }
}
