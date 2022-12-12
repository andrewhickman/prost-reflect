use std::io::{self, Read, Write};

use once_cell::sync::Lazy;
use prost::{
    bytes::{Buf, BufMut},
    Message,
};
use prost_reflect::{text_format, DescriptorPool, DeserializeOptions, DynamicMessage};

use prost_reflect_conformance_tests::conformance::{
    conformance_request, conformance_response, ConformanceRequest, ConformanceResponse,
    TestCategory, WireFormat,
};

const TEST_MESSAGES_DESCRIPTOR_POOL_SET_BYTES: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/test_messages.bin"));

static TEST_MESSAGES_DESCRIPTOR_POOL: Lazy<DescriptorPool> =
    Lazy::new(|| DescriptorPool::decode(TEST_MESSAGES_DESCRIPTOR_POOL_SET_BYTES).unwrap());

fn main() -> io::Result<()> {
    env_logger::init();
    let mut bytes = Vec::new();

    loop {
        bytes.resize(4, 0);

        if io::stdin().read_exact(&mut bytes).is_err() {
            // No more test cases.
            return Ok(());
        }

        let len = bytes.as_slice().get_u32_le() as usize;

        bytes.resize(len, 0);
        io::stdin().read_exact(&mut bytes)?;

        let result = match ConformanceRequest::decode(&*bytes) {
            Ok(request) => handle_request(request),
            Err(error) => conformance_response::Result::ParseError(format!("{:?}", error)),
        };

        let response = ConformanceResponse {
            result: Some(result),
        };

        let len = response.encoded_len();
        bytes.clear();
        bytes.put_u32_le(len as u32);
        response.encode(&mut bytes)?;
        assert_eq!(len + 4, bytes.len());

        let mut stdout = io::stdout();
        stdout.lock().write_all(&bytes)?;
        stdout.flush()?;
    }
}

fn handle_request(request: ConformanceRequest) -> conformance_response::Result {
    let message_desc =
        match TEST_MESSAGES_DESCRIPTOR_POOL.get_message_by_name(&request.message_type) {
            Some(message_desc) => message_desc,
            None => {
                return conformance_response::Result::ParseError(format!(
                    "unknown message type: {}",
                    request.message_type
                ));
            }
        };

    let mut json_deserialize_options = DeserializeOptions::new();

    match request.test_category() {
        TestCategory::UnspecifiedTest => (),
        TestCategory::BinaryTest => (),
        TestCategory::JsonTest => (),
        TestCategory::TextFormatTest => (),
        TestCategory::JsonIgnoreUnknownParsingTest => {
            json_deserialize_options = json_deserialize_options.deny_unknown_fields(false);
        }
        TestCategory::JspbTest => {
            return conformance_response::Result::Skipped("unsupported test category".to_string())
        }
    }

    let output = request.requested_output_format();
    let dynamic_message = match request.payload {
        None => return conformance_response::Result::ParseError("no payload".to_string()),
        Some(conformance_request::Payload::ProtobufPayload(buf)) => {
            let mut dynamic_message = DynamicMessage::new(message_desc);
            match dynamic_message.merge(buf.as_ref()) {
                Ok(()) => (),
                Err(error) => return conformance_response::Result::ParseError(error.to_string()),
            }
            dynamic_message
        }
        Some(conformance_request::Payload::JsonPayload(json)) => {
            let mut deserializer = serde_json::de::Deserializer::from_str(&json);
            match DynamicMessage::deserialize_with_options(
                message_desc,
                &mut deserializer,
                &json_deserialize_options,
            ) {
                Ok(message) => message,
                Err(error) => return conformance_response::Result::ParseError(error.to_string()),
            }
        }
        Some(conformance_request::Payload::JspbPayload(_)) => {
            return conformance_response::Result::Skipped(
                "jspb payload is not supported".to_string(),
            );
        }
        Some(conformance_request::Payload::TextPayload(text)) => {
            match DynamicMessage::parse_text_format(message_desc, &text) {
                Ok(message) => message,
                Err(error) => return conformance_response::Result::ParseError(error.to_string()),
            }
        }
    };

    match output {
        WireFormat::Unspecified => {
            conformance_response::Result::ParseError("output format unspecified".to_string())
        }
        WireFormat::Jspb => {
            conformance_response::Result::Skipped("JSPB output is not supported".to_string())
        }
        WireFormat::TextFormat => {
            let options = text_format::FormatOptions::new()
                .skip_unknown_fields(!request.print_unknown_fields);
            conformance_response::Result::TextPayload(
                dynamic_message.to_text_format_with_options(&options),
            )
        }
        WireFormat::Json => match serde_json::to_string(&dynamic_message) {
            Ok(s) => conformance_response::Result::JsonPayload(s),
            Err(err) => conformance_response::Result::SerializeError(err.to_string()),
        },
        WireFormat::Protobuf => {
            conformance_response::Result::ProtobufPayload(dynamic_message.encode_to_vec())
        }
    }
}
