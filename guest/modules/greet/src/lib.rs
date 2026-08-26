#[cfg(feature = "component")]
mod bindings;
mod handlers;

#[cfg(feature = "component")]
mod component_export {
    use crate::bindings;
    use crate::handlers;
    use bindings::exports::app::greeting::greeting::Guest;

    struct Component;

    impl Guest for Component {
        fn hello(name: String) { handlers::greet(name); }
    }

    bindings::export!(Component with_types_in bindings);
}

// Plain core-wasm export for hosts that don't speak the component model
// (e.g. host/go via wazero, which only understands wasm32-wasip1). There's
// no wit-bindgen equivalent for this target, so the string argument is
// marshaled by hand: the host calls `alloc` to get a buffer inside this
// module's own linear memory, writes the name's UTF-8 bytes into it, then
// calls `hello(ptr, len)`.
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

    #[export_name = "hello"]
    pub extern "C" fn hello(ptr: *const u8, len: u32) {
        let bytes = unsafe { core::slice::from_raw_parts(ptr, len as usize) };
        let name = core::str::from_utf8(bytes).unwrap_or("").to_string();
        handlers::greet(name);
    }
}
