use c509_cert::C509Certificate;
use shared::utils::{log};

pub fn decode(bytes: Vec<u8>) {
    let cert = C509Certificate::decode(&bytes);
    log::log(&format!("decoded cert: {:?}", cert));
}

pub fn decode_sequence(bytes: Vec<u8>) {
    let cert = C509Certificate::decode_sequence(&bytes);
    log::log(&format!("decoded sequence cert: {:?}", cert));
}

// TODO: decide what these input bytes represent (JSON text? something
// else?) — WIT declares `list<u8>` here since a full C509Certificate can't
// be represented in WIT, so this can't just take a C509Certificate directly.
pub fn encode(_bytes: Vec<u8>) {
    log::log("encode: not yet implemented");
}

pub fn encode_sequence(_bytes: Vec<u8>) {
    log::log("encode_sequence: not yet implemented");
}
