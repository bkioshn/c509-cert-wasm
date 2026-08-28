// host-go loads greet.wasm and c509.wasm — the wasm32-wasip1 builds of
// guest/modules/greet and guest/modules/c509 — as plain core wasm modules
// via wazero, since there is currently no mature Go library that can
// instantiate/call a wasm32-wasip1 *component* (see host/rust and host/js
// for that). There is no wit-bindgen equivalent for wasip1, so every
// argument is marshaled by hand: call `alloc` to get a buffer inside the
// guest's own linear memory, write the bytes into it, then call the export
// with (ptr, len). Return values are marshaled by hand too: decode/encode
// exports return a single u64 packing (ptr, len) of a guest-allocated
// buffer whose first byte is a tag (1 = Ok, 0 = Err) and the rest is the
// payload — see guest/modules/c509/src/lib.rs's wasip1_export::write_tagged.
package main

import (
	"context"
	"encoding/hex"
	"fmt"
	"log"
	"os"
	"strings"

	"github.com/tetratelabs/wazero"
	"github.com/tetratelabs/wazero/api"
	"github.com/tetratelabs/wazero/imports/wasi_snapshot_preview1"
)

// Appendix A.1.1: "CBOR re-encoded X.509 v3 Certificate" (c509CertificateType
// = 3), 140-byte unwrapped ~C509Certificate CBOR Sequence — same bytes the
// Rust, JS, and Python hosts decode with decode-sequence.
const a1_1DerReencoded = `
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
`

// Appendix A.1.2: "Natively Signed C509 Certificate" (c509CertificateType =
// 2) corresponding to the same X.509 certificate, array-wrapped (leading
// `8B` = CBOR array(11) header) so plain `decode` (not decode-sequence)
// applies.
const a1_2Native = `
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
`

// C509 certificate JSON string — round-trips onto a1_1DerReencoded via
// encode-sequence (and onto that plus an array header via encode).
const c509CertJSON = `
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
`

// loadModule reads and instantiates a wasm32-wasip1 reactor module, wiring
// it up the same way the Rust host's WasiCtxBuilder does: real wall clock,
// stdout/stderr passthrough, and "." preopened for the guest's log file.
func loadModule(ctx context.Context, runtime wazero.Runtime, name, path string) api.Module {
	wasmBytes, err := os.ReadFile(path)
	if err != nil {
		log.Fatal(err)
	}

	config := wazero.NewModuleConfig().
		WithName(name).
		WithStdout(os.Stdout).
		WithStderr(os.Stderr).
		// wazero defaults to a fake/deterministic wall clock; use real time
		// so timer::now_secs() matches the Rust/JS hosts.
		WithSysWalltime().
		// mirror the Rust host's WasiCtxBuilder::preopened_dir(".", ".", ...)
		// so the guest's log::append("greetings.log", ...) can write.
		WithFSConfig(wazero.NewFSConfig().WithDirMount(".", ".")).
		// this is a "reactor": no fn main, so run wasi-libc's reactor init
		// hook instead of the command entrypoint (_start).
		WithStartFunctions("_initialize")

	mod, err := runtime.InstantiateWithConfig(ctx, wasmBytes, config)
	if err != nil {
		log.Fatal(err)
	}
	return mod
}

// writeBytes calls the guest's `alloc` export to get a buffer inside its own
// linear memory, writes `data` into it, and returns (ptr, len).
func writeBytes(ctx context.Context, mod api.Module, data []byte) (uint32, uint32) {
	results, err := mod.ExportedFunction("alloc").Call(ctx, uint64(len(data)))
	if err != nil {
		log.Fatal(err)
	}
	ptr := uint32(results[0])
	if !mod.Memory().Write(ptr, data) {
		log.Fatalf("failed to write %d bytes into guest memory", len(data))
	}
	return ptr, uint32(len(data))
}

// callTagged invokes a decode/encode export (ptr, len) -> u64, unpacks the
// packed (ptr, len) result, and splits off the leading ok/err tag byte —
// see the wasip1_export::write_tagged doc comment in lib.rs for the format.
func callTagged(ctx context.Context, mod api.Module, fn string, argPtr, argSize uint32) (ok bool, payload []byte) {
	results, err := mod.ExportedFunction(fn).Call(ctx, uint64(argPtr), uint64(argSize))
	if err != nil {
		log.Fatal(err)
	}
	packed := results[0]
	ptr := uint32(packed >> 32)
	size := uint32(packed)
	buf, readOk := mod.Memory().Read(ptr, size)
	if !readOk {
		log.Fatalf("failed to read %d bytes of %s result from guest memory", size, fn)
	}
	return buf[0] == 1, buf[1:]
}

func main() {
	ctx := context.Background()

	runtime := wazero.NewRuntime(ctx)
	defer runtime.Close(ctx)

	// provide the wasi_snapshot_preview1 host functions the guests import
	// (stdout, filesystem, clocks, ...).
	wasi_snapshot_preview1.MustInstantiate(ctx, runtime)

	// Using target-wasip1 to avoid header conflicts with target-wasip2.
	// When compile to wasip1 use
	//```bash
	// cargo build --target wasm32-wasip1 --target-dir target-wasip1 --release --no-default-features --features wasip1
	// ```
	greet := loadModule(ctx, runtime, "greet", "../../guest/target-wasip1/wasm32-wasip1/release/greet.wasm")
	hello := greet.ExportedFunction("hello")
	for _, name := range []string{"Blue", "Butter", "Ngummy"} {
		ptr, size := writeBytes(ctx, greet, []byte(name))
		if _, err := hello.Call(ctx, uint64(ptr), uint64(size)); err != nil {
			log.Fatal(err)
		}
	}

	// Using target-wasip1 to avoid header conflicts with target-wasip2.
	// When compile to wasip1 use
	//```bash
	// cargo build --target wasm32-wasip1 --target-dir target-wasip1 --release --no-default-features --features wasip1
	// ```
	c509 := loadModule(ctx, runtime, "c509", "../../guest/target-wasip1/wasm32-wasip1/release/c509.wasm")

	clean := func(hexStr string) []byte {
		cleaned := strings.Map(func(r rune) rune {
			if r == ' ' || r == '\t' || r == '\n' || r == '\r' {
				return -1
			}
			return r
		}, hexStr)
		data, err := hex.DecodeString(cleaned)
		if err != nil {
			log.Fatal(err)
		}
		return data
	}

	bytesA1_1 := clean(a1_1DerReencoded)
	ptr, size := writeBytes(ctx, c509, bytesA1_1)
	ok, payload := callTagged(ctx, c509, "decode_sequence", ptr, size)
	fmt.Printf("decoded sequence cert (A1.1): ok=%v %s\n", ok, payload)

	bytesA1_2 := clean(a1_2Native)
	ptr, size = writeBytes(ctx, c509, bytesA1_2)
	ok, payload = callTagged(ctx, c509, "decode", ptr, size)
	fmt.Printf("decoded cert (A1.2): ok=%v %s\n", ok, payload)

	ptr, size = writeBytes(ctx, c509, []byte(c509CertJSON))
	ok, payload = callTagged(ctx, c509, "encode", ptr, size)
	if ok {
		fmt.Printf("encoded cert: ok=%v %v\n", ok, payload)
	} else {
		fmt.Printf("encoded cert: ok=%v %s\n", ok, payload)
	}

	ptr, size = writeBytes(ctx, c509, []byte(c509CertJSON))
	ok, payload = callTagged(ctx, c509, "encode_sequence", ptr, size)
	if ok {
		fmt.Printf("encoded sequence cert: ok=%v %v\n", ok, payload)
	} else {
		fmt.Printf("encoded sequence cert: ok=%v %s\n", ok, payload)
	}
}
