use wasmtime::component::{bindgen, Component, Linker};
use wasmtime::{Config, Engine, Store};
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView, ResourceTable};

// 2. host bindings from the SAME wit the guest used
bindgen!({ world: "guest", path: "../wit" });

// state the host carries; must expose WASI so imports resolve
struct State {
    ctx: WasiCtx,
    table: ResourceTable,
}
impl WasiView for State {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView { ctx: &mut self.ctx, table: &mut self.table }
    }
}

fn main() -> wasmtime::Result<()> {
    // 1. engine
    let mut cfg = Config::new();
    cfg.wasm_component_model(true);
    let engine = Engine::new(&cfg)?;

    // 4a. load the guest .wasm
    let component = Component::from_file(
        &engine,
        "../guest/target/wasm32-wasip2/release/greet.wasm",
    )?;

    // 3. provide WASI imports into the linker
    let mut linker = Linker::new(&engine);
    wasmtime_wasi::p2::add_to_linker_sync(&mut linker)?;

    // sandbox policy: grant the guest one directory (for the log file)
    let ctx = WasiCtxBuilder::new()
        .preopened_dir(".", ".", wasmtime_wasi::DirPerms::all(), wasmtime_wasi::FilePerms::all())?
        .inherit_stdout()
        .build();
    let mut store = Store::new(&engine, State { ctx, table: ResourceTable::new() });

    // 4b. instantiate
    let guest = Guest::instantiate(&mut store, &component, &linker)?;

    // 5. call exports — the lifecycle
    for name in ["Blue", "Butter", "Ngummy"] {           // per name
        guest.app_greeting_greeting().call_hello(&mut store, name)?;
    }
    Ok(())
}