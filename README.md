# c509-cert-wasm

A WebAssembly Component Model playground: Rust guest components (`greet`, `c509`)
called from four different host languages (Rust, JavaScript, Python, Go).

## Layout

```
wit/                      WIT interfaces/worlds shared by guest and host
guest/
  shared/                 common guest-side utilities (log, timer)
  modules/
    greet/                exports app:greeting/greeting (hello)
    c509/                 exports app:c509/c509 (decode/decode-sequence/encode/encode-sequence),
                          wraps the c509-cert crate
host/
  rust/                   wasmtime host — calls the wasm32-wasip2 components
  js/                     Node host (jco) — calls the wasm32-wasip2 components
  python/                 wasmtime-py host — calls the wasm32-wasip2 components
  go/                     wazero host — calls a wasm32-wasip1 fallback build (see below)
```

## Prerequisites

- Rust + `rustup target add wasm32-wasip1 wasm32-wasip2`
- Node.js + npm (for `host/js`)
- Python 3 (for `host/python`) — needs `wasmtime` ≥48 specifically; older
  releases don't ship the component-model API (`wasmtime.component`) this
  host uses
- Go (for `host/go`) — `go.mod` requires Go ≥1.25; Go's toolchain manager will
  auto-download a matching version on first build if your system `go` is older
- A local checkout of [`c509-cert`](https://github.com/bkioshn/c509-cert)
  alongside this repo (`../c509-cert` relative to this repo's parent
  directory) — `host/rust` and `guest/modules/c509` depend on it via a local
  `path` dependency

## Why two guest builds

Each guest module (`greet`, `c509`) can be compiled two ways, controlled by
Cargo features (`default = ["component"]` vs `wasip1`):

- **`component`** (default) — a real `wasm32-wasip2` wasm **component**, using
  `wit-bindgen` to implement the WIT world's `Guest` trait. This is what
  `host/rust` (via `wasmtime`'s `bindgen!`), `host/js` (via `jco transpile`),
  and `host/python` (via `wasmtime-py`'s `wasmtime.component` API, including
  `Linker.add_wasip2()` for real WASI Preview 2 support) all load — all three
  have full Component Model support.
- **`wasip1`** — a plain core wasm module with hand-rolled `extern "C"`
  exports (`alloc`, `hello`, `decode`, ...), no wit-bindgen involved. This
  exists because, as of writing, no mature Go library can instantiate or call
  exports on a wasip2 *component* — `wasmtime-go` has open TODOs for both
  WASI Preview 2 wiring and component function calling, and `wazero` has no
  component-model support at all. `host/go` uses `wazero` (pure Go, no cgo)
  against this fallback build instead.

This means `host/go` is calling a **different compiled artifact** than
`host/rust`/`host/js`/`host/python` — same source, same logic, but not "the
same file, every host" the way the other three are.

**Important:** the `wasip1` build's exports only exist if you pass the exact
feature flags below. A plain `cargo build --target wasm32-wasip1` (default
features) silently produces a file with none of the `alloc`/`hello`/etc.
exports `host/go` needs — it'll compile fine but crash `host/go` with a nil
pointer dereference when it tries to call `alloc`. The two builds are kept in
physically separate directories (`target/` vs `target-wasip1/`) so they can
never overwrite each other, but you can still build wasip1 wrong within its
own directory if you drop the feature flags.

## Build

From `guest/`:

```bash
# wasm32-wasip2 components — used by host/rust and host/js
cargo build --target wasm32-wasip2 --release

# wasm32-wasip1 fallback build — used by host/go
cargo build --target wasm32-wasip1 --target-dir target-wasip1 --release \
  --no-default-features --features wasip1
```

Re-run whichever of these changed whenever you edit `guest/modules/*` or `wit/*`.

## Run

**Rust host:**
```bash
cd host/rust
cargo run
```

**JS host** (one-time install + transpile, then run; re-run `transpile` whenever the guest `.wasm` changes):
```bash
cd host/js
npm install
npm run transpile
npm run start
```

**Python host** (one-time venv + install, then run):
```bash
cd host/python
python3 -m venv .venv
./.venv/bin/pip install -r requirements.txt
./.venv/bin/python main.py
```

**Go host:**
```bash
cd host/go
go run .
```

All four print the same three `Hello, <name>!` lines (appended to their own
local `greetings.log`) and decode the same RFC 7925 example C509 certificate
via `c509`.

## Known gaps

- `c509`'s `encode`/`encode-sequence` are unimplemented stubs on the guest
  side — undecided what the input bytes should represent (JSON text to build
  a certificate from? something else?), since a full `C509Certificate` can't
  be represented directly in WIT.
- `host/rust`'s `runtime_extensions/c509_cert` module (thin Rust wrappers
  around the `c509-cert` crate, plus a hand-rolled JSON→CBOR schema in
  `runtime_extensions/c509_cert/json.rs`) isn't wired into any WIT interface
  yet — it's host-side scaffolding for a future host-provided interface, not
  currently called from anywhere.
