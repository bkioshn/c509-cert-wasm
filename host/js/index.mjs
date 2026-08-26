import { _setPreopens } from "@bytecodealliance/preview2-shim/filesystem";
import { greeting } from "./generated/greet/greet.js";
import { c509 } from "./generated/c509/c509.js";

// grant the guest access to "." (for the log file), mirroring the Rust
// host's WasiCtxBuilder::preopened_dir(".", ".", ...)
_setPreopens({ ".": "." });

for (const name of ["Blue", "Butter", "Ngummy"]) {
    greeting.hello(name);
}

// Appendix A.1.1 example from the draft (CBOR re-encoding of a DER-encoded
// RFC 7925 profiled X.509 certificate), as a CBOR Sequence — same bytes the
// Rust host decodes.
const A1_1_DER_REENCODED = `
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
`;

const cleaned = A1_1_DER_REENCODED.replace(/\s+/g, "");
const bytes = Uint8Array.from(Buffer.from(cleaned, "hex"));

c509.decodeSequence(bytes);
