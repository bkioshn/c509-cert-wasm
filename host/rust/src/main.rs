use wasmtime::component::{Component, Linker};
use wasmtime::{Config, Engine, Store};
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView, ResourceTable};

// host bindings — one module per guest world, since greet.wasm and c509.wasm
// are now separate components each satisfying their own narrower world.
mod greeting_bindings {
    wasmtime::component::bindgen!({ world: "greeting-guest", path: "../../wit" });
}
mod c509_bindings {
    wasmtime::component::bindgen!({ world: "c509-guest", path: "../../wit" });
}
// Appendix A.1.1: "CBOR re-encoded X.509 v3 Certificate" (c509CertificateType
// = 3), 140-byte unwrapped ~C509Certificate CBOR Sequence.
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

// Appendix A.1.2: "Natively Signed C509 Certificate" (c509CertificateType =
// 2) corresponding to the same X.509 certificate.
const A1_2_NATIVE: &str = "
    8B
    02
    43 01 F5 0D
    00
    6B 52 46 43 20 74 65 73 74 20 43 41
    1A 63 B0 CD 00
    1A 69 55 B9 00
    D8 30 46 01 23 45 67 89 AB
    01
    58 21 02 B1 21 6A B9 6E 5B 3B 33 40 F5 BD F0 2E 69 3F 16 21 3A 04 52
    5E D4 44 50 B1 01 9C 2D FD 38 38 AB
    01
    58 40 EB 0D 47 27 31 F6 89 BC 00 F5 88 0B 12 C6 8B 3F 9F D3 8B 23 FA
    DF CA 20 95 0F 3F 24 1B 60 A2 02 57 9C AC 28 CD 3B 74 94 D5 FA 5D 8B
    BA B4 60 03 57 E5 50 AB 9F A9 A6 5D 9B A2 B3 B8 2E 66 8C C6
";

/// C509 certificate JSON string
const C509_CERT_JSON: &str = r#"
    {
        "tbs": {
            "c509_certificate_type": 3,
            "certificate_serial_number": "01f50d",
            "issuer_signature_algorithm": { "Int": 0 },
            "issuer": [ { "Registered": { "id": 1, "printable_string": false, "value": { "Text": "RFC test CA" } } } ],
            "validity_not_before": "2023-01-01T00:00:00Z",
            "validity_not_after": "2026-01-01T00:00:00Z",
            "subject": [ { "Registered": { "id": 1, "printable_string": false, "value": { "Mac": "01:23:45:67:89:AB" } } } ],
            "subject_public_key_algorithm": { "Int": 1 },
            "subject_public_key": "feb1216ab96e5b3b3340f5bdf02e693f16213a04525ed44450b1019c2dfd3838ab",
            "extensions": [ { "id": { "Int": 2 }, "critical": false, "value": { "KeyUsage": 1 } } ]
        },
        "issuer_signature_value": "d4320b1d6849e309219d30037e138166f2508247dddae76cceea55053c108e90d551f6d60106f1abb484cfbe6256c178e4ac3314ea19191e8b607da5ae3bda16"
    }
"#;

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
    let bytes_a1_1 = hex::decode(cleaned)?;
    let cleaned_a1_2: String = A1_2_NATIVE.chars().filter(|c| !c.is_whitespace()).collect();
    let bytes_a1_2 = hex::decode(cleaned_a1_2)?;

    // c509_guest.app_c509_c509() exposes decode/decode-sequence/encode/encode-sequence
    let decoded_sequence_a1_1 = &c509_guest.app_c509_c509().call_decode_sequence(&mut store, &bytes_a1_1)?;
    println!("decoded sequence cert (A1.1): {:?}", decoded_sequence_a1_1);
    let decoded_a1_2 = &c509_guest.app_c509_c509().call_decode(&mut store, &bytes_a1_2)?;
    println!("decoded cert (A1.2): {:?}", decoded_a1_2);

    let encoded = c509_guest.app_c509_c509().call_encode(&mut store, C509_CERT_JSON)?;
    println!("encoded cert: {:?}", encoded);
    let encoded_sequence = c509_guest.app_c509_c509().call_encode_sequence(&mut store, C509_CERT_JSON)?;
    println!("encoded sequence cert: {:?}", encoded_sequence);
    Ok(())
}
