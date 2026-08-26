"""host-python loads greet.wasm and c509.wasm — the same wasm32-wasip2
*components* host/rust and host/js use — via wasmtime-py's component-model
API (Engine/Linker.add_wasip2()/Func with real value marshaling). Unlike
host/go, this needs no fallback build: wasmtime-py has full WASI Preview 2
support and can call component exports with real WIT-typed values.
"""

import re

from wasmtime import Config, Engine, Store, WasiConfig
from wasmtime.component import Component, Linker

# Appendix A.1.1 example from the draft (CBOR re-encoding of a DER-encoded
# RFC 7925 profiled X.509 certificate), as a CBOR Sequence — same bytes the
# Rust, JS, and Go hosts decode.
A1_1_DER_REENCODED = """
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
"""


def get_iface_func(store, instance, iface_name, func_name):
    """Look up a function nested inside a WIT interface export, e.g.
    `app:greeting/greeting`'s `hello`."""
    iface_index = instance.get_export_index(store, iface_name)
    func_index = instance.get_export_index(store, func_name, instance=iface_index)
    return instance.get_func(store, func_index)


def main():
    config = Config()
    config.wasm_component_model = True
    engine = Engine(config)

    store = Store(engine)
    wasi = WasiConfig()
    wasi.inherit_stdout()
    # mirror the Rust host's WasiCtxBuilder::preopened_dir(".", ".", ...) so
    # the guest's log::append("greetings.log", ...) can write.
    wasi.preopen_dir(".", ".")
    store.set_wasi(wasi)

    linker = Linker(engine)
    linker.add_wasip2()

    greet_component = Component.from_file(
        engine, "../../guest/target/wasm32-wasip2/release/greet.wasm"
    )
    greet_instance = linker.instantiate(store, greet_component)
    hello = get_iface_func(store, greet_instance, "app:greeting/greeting", "hello")

    for name in ["Blue", "Butter", "Ngummy"]:
        hello(store, name)

    c509_component = Component.from_file(
        engine, "../../guest/target/wasm32-wasip2/release/c509.wasm"
    )
    c509_instance = linker.instantiate(store, c509_component)
    decode_sequence = get_iface_func(
        store, c509_instance, "app:c509/c509", "decode-sequence"
    )

    cleaned = re.sub(r"\s+", "", A1_1_DER_REENCODED)
    decode_sequence(store, bytes.fromhex(cleaned))


if __name__ == "__main__":
    main()
