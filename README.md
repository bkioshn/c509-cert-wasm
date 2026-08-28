# c509-cert-wasm

A WebAssembly Component Model playground: Rust guest components (`greet`, `c509`)
called from four different host languages (Rust, JavaScript, Python, Go).

## Layout

```
wit/                      WIT interfaces/worlds shared by guest and host
<!-- guest/ -->
  shared/                 common guest-side utilities (log, timer)
  modules/
    greet/                exports app:greeting/greeting (hello) - an example
    c509/                 exports app:c509/c509 (decode/decode-sequence/encode/encode-sequence),
                          wraps the c509-cert crate
  justfile                
host/
  rust/                   wasmtime host — calls the wasm32-wasip2 components
  js/                     Node host (jco) — calls the wasm32-wasip2 components
  python/                 wasmtime-py host — calls the wasm32-wasip2 components
  go/                     wazero host — calls a wasm32-wasip1 fallback build (see below)
  justfile                
```

## Prerequisites

- Rust + `rustup target add wasm32-wasip1 wasm32-wasip2`
- Node.js + npm (for `host/js`)
- Python 3 (for `host/python`) — needs `wasmtime` ≥48 specifically; older
  releases don't ship the component-model API (`wasmtime.component`) this
  host uses
- Go (for `host/go`) — `go.mod` requires Go ≥1.25; Go's toolchain manager will
  auto-download a matching version on first build if your system `go` is older
- [`just`](https://github.com/casey/just) (optional) — runs the `justfile`s in
  `guest/` and `host/`; every recipe is just a thin wrapper around the plain
  `cargo`/`npm`/`go` commands documented below, so it's not required

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
  against this fallback build instead. Since there's no wit-bindgen-generated
  return marshaling on this path either, `decode`/`decode_sequence`/
  `encode`/`encode_sequence` also hand-roll their *return* values: each packs
  a `(ptr, len)` pair into a single `u64`, pointing at a guest-allocated
  buffer whose first byte is an ok/err tag — see
  `wasip1_export::write_tagged` in `guest/modules/c509/src/lib.rs`, and
  `callTagged` in `host/go/main.go` for the host side of that convention.

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
# wasm32-wasip2 components — used by host/rust, host/js, and host/python
cargo build --target wasm32-wasip2 --release

# wasm32-wasip1 fallback build — used by host/go
cargo build --target wasm32-wasip1 --target-dir target-wasip1 --release \
  --no-default-features --features wasip1
```

Or, with `just`: `just build`, `just wasip1`, or `just` (both). Re-run
whichever of these changed whenever you edit `guest/modules/*` or `wit/*`.

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

Or, with `just` (from `host/`): `just rust`, `just js`, `just python`,
`just go`, or `just run-all` for all four in turn. (`js` and `python` each
have a one-time `just js-install`/`just python-venv` setup recipe too,
matching the commands above.)

All four print the same three `Hello, <name>!` lines (appended to their own
local `greetings.log`), decode the same RFC 7925 example C509 certificate via
`c509` (both the array-wrapped form via `decode` and the bare-sequence form
via `decode-sequence`), and round-trip a sample certificate through
`encode`/`encode-sequence` back onto those same RFC test-vector bytes.

## Known gaps

- The wasip1 fallback path's `decode`/`decode_sequence`/`encode`/`encode_sequence`
  return values use a hand-rolled tagged `(ptr, len)` convention (see "Why two
  guest builds" above) instead of any standard ABI — it's only implemented in
  `guest/modules/c509/src/lib.rs`'s `wasip1_export` module and consumed by
  `host/go`; a new wasip1-only host would need to reimplement that unpacking
  logic by hand.
