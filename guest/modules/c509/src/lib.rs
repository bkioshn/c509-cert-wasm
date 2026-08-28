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

    #[export_name = "decode"]
    pub extern "C" fn decode(ptr: *const u8, len: u32) {
        handlers::decode(unsafe { read_bytes(ptr, len) });
    }

    #[export_name = "decode_sequence"]
    pub extern "C" fn decode_sequence(ptr: *const u8, len: u32) {
        handlers::decode_sequence(unsafe { read_bytes(ptr, len) });
    }

    #[export_name = "encode"]
    pub extern "C" fn encode(ptr: *const u8, len: u32) {
        handlers::encode(unsafe { read_bytes(ptr, len) });
    }

    #[export_name = "encode_sequence"]
    pub extern "C" fn encode_sequence(ptr: *const u8, len: u32) {
        handlers::encode_sequence(unsafe { read_bytes(ptr, len) });
    }
}
