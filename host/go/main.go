// host-go loads greet.wasm and c509.wasm — the wasm32-wasip1 builds of
// guest/modules/greet and guest/modules/c509 — as plain core wasm modules
// via wazero, since there is currently no mature Go library that can
// instantiate/call a wasm32-wasip1 *component* (see host/rust and host/js
// for that). There is no wit-bindgen equivalent for wasip1, so every
// argument is marshaled by hand: call `alloc` to get a buffer inside the
// guest's own linear memory, write the bytes into it, then call the export
// with (ptr, len).
package main

import (
	"context"
	"encoding/hex"
	"log"
	"os"
	"strings"

	"github.com/tetratelabs/wazero"
	"github.com/tetratelabs/wazero/api"
	"github.com/tetratelabs/wazero/imports/wasi_snapshot_preview1"
)

// Appendix A.1.1 example from the draft (CBOR re-encoding of a DER-encoded
// RFC 7925 profiled X.509 certificate), as a CBOR Sequence — same bytes the
// Rust and JS hosts decode.
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
	cleaned := strings.Map(func(r rune) rune {
		if r == ' ' || r == '\t' || r == '\n' || r == '\r' {
			return -1
		}
		return r
	}, a1_1DerReencoded)
	bytes, err := hex.DecodeString(cleaned)
	if err != nil {
		log.Fatal(err)
	}
	ptr, size := writeBytes(ctx, c509, bytes)
	if _, err := c509.ExportedFunction("decode_sequence").Call(ctx, uint64(ptr), uint64(size)); err != nil {
		log.Fatal(err)
	}
}
