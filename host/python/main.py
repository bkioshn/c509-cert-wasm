"""host-python loads greet.wasm and c509.wasm — the same wasm32-wasip2
*components* host/rust and host/js use — via wasmtime-py's component-model
API (Engine/Linker.add_wasip2()/Func with real value marshaling). Unlike
host/go, this needs no fallback build: wasmtime-py has full WASI Preview 2
support and can call component exports with real WIT-typed values.
"""

import re

from wasmtime import Config, Engine, Store, WasiConfig
from wasmtime.component import Component, Linker, Variant

# Appendix A.1.1: "CBOR re-encoded X.509 v3 Certificate" (c509CertificateType
# = 3), 140-byte unwrapped ~C509Certificate CBOR Sequence — same bytes the
# Rust, JS, and Go hosts decode with decode-sequence.
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

# Appendix A.1.2: "Natively Signed C509 Certificate" (c509CertificateType =
# 2) corresponding to the same X.509 certificate, array-wrapped (leading `8B`
# = CBOR array(11) header) so plain `decode` (not decode-sequence) applies.
A1_2_NATIVE = """
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
"""

# C509 certificate JSON string — round-trips onto A1_1_DER_REENCODED via
# encode-sequence (and onto that plus an array header via encode).
C509_CERT_JSON = """
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
    decode = get_iface_func(store, c509_instance, "app:c509/c509", "decode")
    decode_sequence = get_iface_func(
        store, c509_instance, "app:c509/c509", "decode-sequence"
    )
    encode = get_iface_func(store, c509_instance, "app:c509/c509", "encode")
    encode_sequence = get_iface_func(
        store, c509_instance, "app:c509/c509", "encode-sequence"
    )

    bytes_a1_1 = bytes.fromhex(re.sub(r"\s+", "", A1_1_DER_REENCODED))
    decoded_sequence_a1_1 = decode_sequence(store, bytes_a1_1)
    print(f"decoded sequence cert (A1.1): {decoded_sequence_a1_1}")

    bytes_a1_2 = bytes.fromhex(re.sub(r"\s+", "", A1_2_NATIVE))
    decoded_a1_2 = decode(store, bytes_a1_2)
    print(f"decoded cert (A1.2): {decoded_a1_2}")

    def as_byte_values(data):
        if isinstance(data, bytes):
            return list(data)
        return data  # err case: plain string

    encoded = encode(store, C509_CERT_JSON)
    print(f"encoded cert: {as_byte_values(encoded)}")

    encoded_sequence = encode_sequence(store, C509_CERT_JSON)
    print(f"encoded sequence cert: {as_byte_values(encoded_sequence)}")


if __name__ == "__main__":
    main()
