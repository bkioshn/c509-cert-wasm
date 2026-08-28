use c509_cert::C509Certificate;

/// Decode a C509 certificate from bytes and return a JSON string
/// Expect a certificate wrap in array of 11 elements.
pub fn decode(bytes: Vec<u8>) -> Result<String, String> {
    let cert = C509Certificate::decode(&bytes);
    cert.map_err(|e| e.to_string())
        .and_then(|cert| serde_json::to_string(&cert).map_err(|e| e.to_string()))
}

/// Decode a C509 certificate sequence from bytes and return a JSON string
/// Expect a certificate sequence without a array wrapper with 11 elements.
pub fn decode_sequence(bytes: Vec<u8>) -> Result<String, String> {
    let cert = C509Certificate::decode_sequence(&bytes);
    cert.map_err(|e| e.to_string())
        .and_then(|cert| serde_json::to_string(&cert).map_err(|e| e.to_string()))
}

/// Encode a C509 certificate from a JSON string and return bytes
/// Return a certificate wrap in array with 11 elements.
pub fn encode(json_string: String) -> Result<Vec<u8>, String> {
    let cert: C509Certificate = serde_json::from_str(&json_string).map_err(|e| e.to_string())?;
    Ok(cert.encode())
}

/// Encode a C509 certificate sequence from a JSON string and return bytes
/// Return a certificate sequence without a array wrapper with 11 elements.
pub fn encode_sequence(json_string: String) -> Result<Vec<u8>, String> {
    let cert: C509Certificate = serde_json::from_str(&json_string).map_err(|e| e.to_string())?;
    Ok(cert.encode_sequence())
}
