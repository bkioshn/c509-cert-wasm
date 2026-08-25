pub mod bindings {
    wit_bindgen::generate!({
        world: "app",
        path: "../wit",
        pub_export_macro: true,
        default_bindings_module: "shared::bindings",
    });
}
use wasmtime::component::bindgen;

bindgen!({
    path: "../wit",
});