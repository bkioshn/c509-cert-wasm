#[cfg(feature = "component")]
mod bindings;
mod handlers;

#[cfg(feature = "component")]
mod component_export {
    use crate::bindings;
    use crate::handlers;
    use bindings::exports::app::c509::c509::Guest;

    struct Component;

    impl Guest for Component {
        fn decode(bytes: Vec<u8>) -> Result<String, String> {
            handlers::decode(bytes)
        }
        fn decode_sequence(bytes: Vec<u8>) -> Result<String, String> {
            handlers::decode_sequence(bytes)
        }
        fn encode(json_string: String) -> Result<Vec<u8>, String> {
            handlers::encode(json_string)
        }
        fn encode_sequence(json_string: String) -> Result<Vec<u8>, String> {
            handlers::encode_sequence(json_string)
        }
    }

    bindings::export!(Component with_types_in bindings);
}

// Plain core-wasm export for hosts that don't speak the component model
// (e.g. host/go via wazero, which only understands wasm32-wasip1). There's
// no wit-bindgen equivalent for this target, so the byte-buffer arguments
// are marshaled by hand: the host calls `alloc` to get a buffer inside this
// module's own linear memory, writes the bytes into it, then calls the
// function with (ptr, len).
#[cfg(feature = "wasip1")]
mod wasip1_export {
    use crate::handlers;

    // `#[no_mangle]` alone isn't enough to force a wasm-level export on
    // wasm32-wasip1 (unlike wasm32-unknown-unknown, where it usually is) —
    // `export_name` explicitly tells the linker to create the export.
    #[export_name = "alloc"]
    pub extern "C" fn alloc(size: u32) -> *mut u8 {
        let mut buf = Vec::with_capacity(size as usize);
        let ptr = buf.as_mut_ptr();
        core::mem::forget(buf);
        ptr
    }

    unsafe fn read_bytes(ptr: *const u8, len: u32) -> Vec<u8> {
        unsafe { core::slice::from_raw_parts(ptr, len as usize).to_vec() }
    }

    // Marshal a Result out to the host as a single (ptr, len) pair packed
    // into a u64 (ptr in the high 32 bits, len in the low 32 bits) — the
    // simplest return convention that doesn't need the wasm multi-value
    // proposal. The buffer's first byte is a tag (1 = Ok, 0 = Err); the rest
    // is the payload (UTF-8 text for the Ok(String)/Err(String) case, raw
    // bytes for the Ok(Vec<u8>) case).
    fn write_tagged(ok: bool, mut payload: Vec<u8>) -> u64 {
        let mut buf = Vec::with_capacity(1 + payload.len());
        buf.push(ok as u8);
        buf.append(&mut payload);
        let len = buf.len() as u32;
        let ptr = buf.as_mut_ptr();
        core::mem::forget(buf);
        ((ptr as u64) << 32) | (len as u64)
    }

    #[export_name = "decode"]
    pub extern "C" fn decode(ptr: *const u8, len: u32) -> u64 {
        match handlers::decode(unsafe { read_bytes(ptr, len) }) {
            Ok(s) => write_tagged(true, s.into_bytes()),
            Err(e) => write_tagged(false, e.into_bytes()),
        }
    }

    #[export_name = "decode_sequence"]
    pub extern "C" fn decode_sequence(ptr: *const u8, len: u32) -> u64 {
        match handlers::decode_sequence(unsafe { read_bytes(ptr, len) }) {
            Ok(s) => write_tagged(true, s.into_bytes()),
            Err(e) => write_tagged(false, e.into_bytes()),
        }
    }

    #[export_name = "encode"]
    pub extern "C" fn encode(ptr: *const u8, len: u32) -> u64 {
        let json_string = match String::from_utf8(unsafe { read_bytes(ptr, len) }) {
            Ok(s) => s,
            Err(e) => return write_tagged(false, e.to_string().into_bytes()),
        };
        match handlers::encode(json_string) {
            Ok(cbor) => write_tagged(true, cbor),
            Err(e) => write_tagged(false, e.into_bytes()),
        }
    }

    #[export_name = "encode_sequence"]
    pub extern "C" fn encode_sequence(ptr: *const u8, len: u32) -> u64 {
        let json_string = match String::from_utf8(unsafe { read_bytes(ptr, len) }) {
            Ok(s) => s,
            Err(e) => return write_tagged(false, e.to_string().into_bytes()),
        };
        match handlers::encode_sequence(json_string) {
            Ok(cbor) => write_tagged(true, cbor),
            Err(e) => write_tagged(false, e.into_bytes()),
        }
    }
}
