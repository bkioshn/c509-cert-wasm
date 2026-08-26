use wasmtime::component::{Component, Linker};
use wasmtime::{Config, Engine, Store};
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView, ResourceTable};

mod runtime_extensions;

// host bindings — one module per guest world, since greet.wasm and c509.wasm
// are now separate components each satisfying their own narrower world.
mod greeting_bindings {
    wasmtime::component::bindgen!({ world: "greeting-guest", path: "../../wit" });
}
mod c509_bindings {
    wasmtime::component::bindgen!({ world: "c509-guest", path: "../../wit" });
}
const A1_1_DER_REENCODED: &str = "
    03
    43 01 F5 0D
    00
    6B 52 46 43 20 74 65 73 74 20 43 41
    1A 63 B0 CD 00
    1A 69 55 B9 00
    D8 30 46 01 23 45 67 89 AB
    01
    58 21 FE B1 21 6A B9 6E 5B 3B 33 40 F5 BD F0 2E 69 3F 16 21 3A 04 52
    5E D4 44 50 B1 01 9C 2D FD 38 38 AB
    01
    58 40 D4 32 0B 1D 68 49 E3 09 21 9D 30 03 7E 13 81 66 F2 50 82 47 DD
    DA E7 6C CE EA 55 05 3C 10 8E 90 D5 51 F6 D6 01 06 F1 AB B4 84 CF BE
    62 56 C1 78 E4 AC 33 14 EA 19 19 1E 8B 60 7D A5 AE 3B DA 16
";

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

    // 4a. load the guest .wasm — one file per module now
    let greeting_component = Component::from_file(
        &engine,
        "../../guest/target/wasm32-wasip2/release/greet.wasm",
    )?;
    let c509_component = Component::from_file(
        &engine,
        "../../guest/target/wasm32-wasip2/release/c509.wasm",
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

    // 4b. instantiate — each component against its own world's bindings
    let greeting_guest =
        greeting_bindings::GreetingGuest::instantiate(&mut store, &greeting_component, &linker)?;
    let c509_guest =
        c509_bindings::C509Guest::instantiate(&mut store, &c509_component, &linker)?;

    // 5. call exports — the lifecycle
    for name in ["Blue", "Butter", "Ngummy"] {           // per name
        greeting_guest.app_greeting_greeting().call_hello(&mut store, name)?;
    }

    let cleaned: String = A1_1_DER_REENCODED.chars().filter(|c| !c.is_whitespace()).collect();
    let bytes = hex::decode(cleaned)?;

    // c509_guest.app_c509_c509() exposes decode/decode-sequence/encode/encode-sequence
    let _ = &c509_guest.app_c509_c509().call_decode_sequence(&mut store, &bytes)?;

    Ok(())
}
