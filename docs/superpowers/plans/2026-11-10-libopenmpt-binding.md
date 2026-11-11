# libopenmpt binding and crate foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans (inline) for step tracking inside `nbl-dev:ship-issue`, entered through `dev-sprint`. Do not use superpowers:subagent-driven-development: the user's global rules forbid it. Steps use checkbox (`- [ ]`) syntax for tracking. After the last task, return to `ship-issue` Step 6. Do not run the `executing-plans` tail.

**Goal:** A Cargo workspace that statically builds pinned stock libopenmpt 0.8.10 from a git submodule, with committed bindgen bindings (`openmpt-ext-sys`), a safe wrapper (`openmpt-ext`), and a `mod2stems` version-print stub, green in CI on macOS and Linux.

**Architecture:** `crates/openmpt-ext-sys` owns the submodule, the `cc` build of libopenmpt plus miniz, minimp3, and stb_vorbis, and the raw bindings. `crates/openmpt-ext` owns all safe-wrapper `unsafe` code behind one `Module` type. The root `mod2stems` binary forbids `unsafe` and only prints version info for now.

**Tech Stack:** Rust stable (`rust-toolchain.toml`), `cc` (feature `parallel`), bindgen CLI (bindings regeneration only), `proptest` (dev), `cargo-nextest`, `cargo-deny`, GitHub Actions, prek.

**Spec:** `docs/superpowers/specs/2026-11-10-libopenmpt-binding-design.md`. Read it with this plan. Its "Upstream facts at the pinned tag" section has the source citations that every task relies on.

## Global Constraints

- Pin: submodule `crates/openmpt-ext-sys/vendor/openmpt` = `https://github.com/OpenMPT/openmpt.git` at tag `libopenmpt-0.8.10`, commit `dc4ddcf68…` (take the full SHA from `git rev-parse`), SVN revision `25840`, SVN URL `https://source.openmpt.org/svn/openmpt/branches/OpenMPT-1.32`.
- The vendored tree is unmodified upstream. Never edit a file under `vendor/openmpt`.
- `cargo build` must not touch the network. The build has no `git` call and no pkg-config.
- C++ is compiled with `-std=c++17`. Defines: `LIBOPENMPT_BUILD`, `MPT_WITH_MINIZ`, `MPT_WITH_MINIMP3`, `MPT_WITH_STBVORBIS`, `STB_VORBIS_NO_PULLDATA_API`, `STB_VORBIS_NO_STDIO`. Never define `LIBOPENMPT_BUILD_DLL` or `LIBOPENMPT_USE_DLL`.
- Crate names: `openmpt-ext-sys` (`links = "openmpt"`) and `openmpt-ext`. Both `publish = false`, with full publish metadata.
- Licenses: our code is `Apache-2.0`. The `openmpt-ext-sys` license field is `Apache-2.0 AND BSD-3-Clause AND MIT AND CC0-1.0 AND (MIT OR Unlicense)`.
- `mod2stems` uses `#![forbid(unsafe_code)]`. `openmpt-ext` is the only crate with safe-wrapper `unsafe`, and every `unsafe` block has a `// SAFETY:` comment.
- Every libopenmpt string is freed with `openmpt_free_string`. Every `int` result and every stored error is checked.
- Rust gate: `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo nextest run`, `cargo deny check`. Zero warnings in our code. The rust-dev bundle rules (`rust.md`) are canonical for lint policy.
- Look up the current stable version of every crate, CLI, and action at implementation time. Pin each one exactly. Pin actions to a full SHA with a version comment.
- Bash scripts: `set -euo pipefail`, `--help` with the usage at the top of the file, shellcheck and shfmt clean.
- Comments, docs, and error text follow ASD-STE100 (`rules/simplified-technical-english.md`). Markdown files use one sentence per line.
- Platforms: macOS and Linux only. Windows is #14. Release packaging is #15.

## Review Focus

1. Names that are not UTF-8: MOD names are raw bytes (often CP437). A name with byte `0xFF` must come back as a lossy `String` with no panic. Test in Task 5.
2. A bad sample rate: `read_interleaved_float_stereo` with rate `0` or `1_000_000` must return an `OpenmptError` that names the rate. It must not return `Ok(0)`, which reads as "song end". Test in Task 8.
3. Reads after the song ends: further reads return `Ok(0)`, and the position getters still return values with no error. Test in Task 8.
4. A `Module` moved to another thread: a render on a spawned thread works (`Send`). Test in Task 4.
5. A module with no instruments: the instrument mute range is the sample count, and index `0` or `count + 1` is an error with the valid range in the message. Test in Task 9.

---

## File Structure

| Path | Responsibility |
|------|----------------|
| `Cargo.toml` | Workspace root, `mod2stems` package, shared lint table |
| `rust-toolchain.toml` | Pinned stable toolchain |
| `deny.toml` | `cargo deny` licenses, bans, sources |
| `LICENSE` | Apache-2.0 text for our code |
| `src/main.rs` | Version-print stub |
| `crates/openmpt-ext-sys/Cargo.toml` | `links = "openmpt"`, `include` list for packaging |
| `crates/openmpt-ext-sys/build.rs` | Pin constants, source list, `cc` compile |
| `crates/openmpt-ext-sys/wrapper.h` | Includes both libopenmpt headers |
| `crates/openmpt-ext-sys/src/lib.rs` | `include!` of bindings, pin constants re-exported for tests |
| `crates/openmpt-ext-sys/src/bindings.rs` | bindgen output, committed |
| `crates/openmpt-ext-sys/tests/pin.rs` | Pin check and license-file check |
| `crates/openmpt-ext-sys/tests/link.rs` | Smoke test of the static link |
| `crates/openmpt-ext-sys/LICENSES/*.txt` | Third-party notices |
| `crates/openmpt-ext/src/lib.rs` | Public re-exports, crate docs |
| `crates/openmpt-ext/src/error.rs` | `OpenmptError` |
| `crates/openmpt-ext/src/cstr.rs` | `LibString` guard for library strings |
| `crates/openmpt-ext/src/module.rs` | `Module` ownership, log sink, interface fetch |
| `crates/openmpt-ext/src/info.rs` | Library info and metadata |
| `crates/openmpt-ext/src/ctl.rs` | Ctls and render params |
| `crates/openmpt-ext/src/position.rs` | Subsong, position, pattern data |
| `crates/openmpt-ext/src/render.rs` | Float read |
| `crates/openmpt-ext/src/interactive.rs` | Channel mute and volume, instrument mute |
| `crates/openmpt-ext/tests/support/mod.rs` | In-memory MOD generator |
| `crates/openmpt-ext/tests/*.rs` | One integration test file per wrapper file |
| `scripts/regen-bindings.sh` | Regenerate `bindings.rs` |
| `scripts/bump-libopenmpt.sh` | Move the pin to a new tag |
| `.github/workflows/ci.yml` | CI |
| `.github/dependabot.yml` | Cargo and Actions updates |
| `.pre-commit-config.yaml` | prek hooks |

Each `impl Module` block lives in the file for its group (Rust allows `impl` blocks in several modules of one crate).

---

### Task 1: Workspace scaffold, licenses, toolchain

**Files:**
- Create: `Cargo.toml`, `rust-toolchain.toml`, `deny.toml`, `LICENSE`, `src/main.rs`, `crates/openmpt-ext/Cargo.toml`, `crates/openmpt-ext/src/lib.rs`, `crates/openmpt-ext-sys/Cargo.toml`, `crates/openmpt-ext-sys/src/lib.rs`
- Modify: `.gitignore` (add `/target`)

**Interfaces:**
- Produces: workspace members `crates/openmpt-ext-sys` and `crates/openmpt-ext`; root package `mod2stems` (edition 2024) depending on `openmpt-ext` by path; workspace `[workspace.lints]` table that each crate inherits.

- [ ] **Step 1:** Look up current stable versions of `cc`, `proptest`, and the Rust toolchain. Write `rust-toolchain.toml` with `channel = "<that version>"` and components `rustfmt`, `clippy`.
- [ ] **Step 2:** Write the root `Cargo.toml`: `[workspace] members = ["crates/*"]`, `resolver = "3"`, package `mod2stems` with `license = "Apache-2.0"`, `publish = false`. Set `[workspace.lints.rust] unsafe_op_in_unsafe_fn = "deny"` and the clippy policy from the rust-dev bundle.
- [ ] **Step 3:** Write the two crate manifests with description, repository `https://github.com/phaedrus1992/mod2stems`, `publish = false`, and the license values from Global Constraints. `openmpt-ext-sys` gets `links = "openmpt"`. Empty `src/lib.rs` files have a crate doc comment only. `src/main.rs` has `#![forbid(unsafe_code)]` and an empty `main`.
- [ ] **Step 4:** Add the standard Apache-2.0 text as `LICENSE`. Write `deny.toml` with an allow list of `MIT`, `Apache-2.0`, `BSD-3-Clause`, `CC0-1.0`, and `Unlicense`, and only crates.io as a source. Add another license only when `cargo deny check` reports it for a real dependency.
- [ ] **Step 5:** Run `cargo build && cargo clippy --all-targets --all-features -- -D warnings && cargo fmt --check && cargo deny check`. Expected: all pass with no warnings.
- [ ] **Step 6:** Commit: `chore: add cargo workspace scaffold and licenses`.

### Task 2: Submodule, wrapper.h, committed bindings

**Files:**
- Create: `.gitmodules` (via `git submodule add`), `crates/openmpt-ext-sys/wrapper.h`, `crates/openmpt-ext-sys/src/bindings.rs`, `scripts/regen-bindings.sh`
- Modify: `crates/openmpt-ext-sys/src/lib.rs`

**Interfaces:**
- Produces: `openmpt_ext_sys::*`, the raw bindgen items, including `openmpt_module_ext_create_from_memory`, `openmpt_module_ext_get_interface`, `openmpt_module_ext_interface_interactive`, `LIBOPENMPT_EXT_C_INTERFACE_INTERACTIVE`, `OPENMPT_MODULE_COMMAND_*`, `OPENMPT_MODULE_RENDER_*`, and `OPENMPT_ERROR_*`.
- Produces: `scripts/regen-bindings.sh [--check]`. With `--check`, it exits 1 when the committed file differs.

- [ ] **Step 1:** Run `git submodule add --depth 1 https://github.com/OpenMPT/openmpt.git crates/openmpt-ext-sys/vendor/openmpt`. Check out tag `libopenmpt-0.8.10` in the submodule. Set `shallow = true` for it in `.gitmodules`. Verify that `git -C crates/openmpt-ext-sys/vendor/openmpt rev-parse HEAD` starts with `dc4ddcf68`.
- [ ] **Step 2:** Write `wrapper.h` with `#include <libopenmpt/libopenmpt.h>` and `#include <libopenmpt/libopenmpt_ext.h>`.
- [ ] **Step 3:** Look up the current bindgen CLI version. Write `scripts/regen-bindings.sh`. It checks that `bindgen` is that exact version and fails with an install hint when it is not. It runs bindgen on `wrapper.h` with `-I crates/openmpt-ext-sys/vendor/openmpt`, `--allowlist-function 'openmpt_.*'`, `--allowlist-type 'openmpt_.*'`, `--allowlist-var '(OPENMPT|LIBOPENMPT)_.*'`, and layout tests kept. It writes the output to `src/bindings.rs`. In `--check` mode, it writes to a temp file and diffs.
- [ ] **Step 4:** Run the script. In `src/lib.rs`, add `#![allow(non_camel_case_types, non_snake_case, non_upper_case_globals)]`, with a comment that the names come from the C headers, and `include!("bindings.rs");`.
- [ ] **Step 5:** Run `shellcheck scripts/regen-bindings.sh && shfmt -d scripts/regen-bindings.sh && scripts/regen-bindings.sh --check`. Expected: no output, exit 0. Do not run `cargo test` yet: the bindgen layout tests compile, but linking needs Task 3.
- [ ] **Step 6:** Commit: `feat(sys): vendor libopenmpt 0.8.10 and commit bindings`.

### Task 3: build.rs static compile, pin check, license files

**Files:**
- Create: `crates/openmpt-ext-sys/build.rs`, `crates/openmpt-ext-sys/tests/pin.rs`, `crates/openmpt-ext-sys/tests/link.rs`, `crates/openmpt-ext-sys/LICENSES/{libopenmpt,miniz,minimp3,stb_vorbis}.txt`
- Modify: `crates/openmpt-ext-sys/Cargo.toml` (`build-dependencies.cc` with feature `parallel`; `include` list), `crates/openmpt-ext-sys/src/lib.rs`

**Interfaces:**
- Produces: in `build.rs`, `const PIN_TAG: &str = "libopenmpt-0.8.10"`, `const PIN_COMMIT: &str = "<full sha>"`, `const PIN_SVN_REVISION: &str = "25840"`, `const PIN_SVN_URL: &str = "https://source.openmpt.org/svn/openmpt/branches/OpenMPT-1.32"`, `const PIN_SVN_DATE: &str = "<tag commit date, ISO 8601>"`. `build.rs` exports them as `cargo:rustc-env=OPENMPT_EXT_SYS_PIN_COMMIT=…` (and the same for the tag and revision). `src/lib.rs` re-exports them as `pub const PIN_COMMIT: &str = env!("OPENMPT_EXT_SYS_PIN_COMMIT");` and so on.

- [ ] **Step 1: Write the failing tests.** In `tests/link.rs`:

```rust
#[test]
fn library_version_string_is_pinned() {
    // read openmpt_get_string("library_version") through a CStr, free it with openmpt_free_string
    assert_eq!(get_string("library_version"), "0.8.10");
}
#[test]
fn source_revision_comes_from_pin() {
    assert_eq!(get_string("source_revision"), openmpt_ext_sys::PIN_SVN_REVISION);
}
```

In `tests/pin.rs`, write `pin_commit_matches_submodule_head`. It runs `git -C <CARGO_MANIFEST_DIR>/vendor/openmpt rev-parse HEAD` and asserts that the result equals `PIN_COMMIT`. When the directory has no `.git`, it prints "skip: vendor/openmpt is not a git checkout" and returns. Write `pin_revision_matches_git_svn_id`, which reads `git log -1 --format=%B` and asserts that it contains `@{PIN_SVN_REVISION} `. Write `license_files_exist`, which asserts that each of the four `LICENSES/*.txt` files exists and is not empty.

- [ ] **Step 2:** Run `cargo nextest run -p openmpt-ext-sys`. Expected: FAIL (link errors or missing constants).
- [ ] **Step 3: Implement `build.rs`.**
  - Fail with `"crates/openmpt-ext-sys/vendor/openmpt is empty. Run: git submodule update --init --depth 1"` when `vendor/openmpt/libopenmpt/libopenmpt.h` is missing.
  - Glob these directories with `std::fs::read_dir`, sort the result, and keep `.cpp` files: `src/openmpt/{all,base,logging,random,fileformat_base,soundbase,soundfile_data}`, `common`, `soundlib`, `soundlib/plugins`, `soundlib/plugins/dmo`, `sounddsp`, and `libopenmpt`.
  - Compile them with one `cc::Build` with `.cpp(true).std("c++17")` and the defines from Global Constraints. Also define `MPT_SVNURL`, `MPT_SVNVERSION`, and `MPT_SVNDATE` as quoted strings from the pin constants. The date comes from `PIN_SVN_DATE`.
  - Compile `include/miniz/miniz.c`, `include/minimp3/minimp3.c`, and `include/stb_vorbis/stb_vorbis.c` with a second `cc::Build` (C) that uses the same defines.
  - Include paths: `vendor/openmpt`, `vendor/openmpt/src`, `vendor/openmpt/common`, `vendor/openmpt/include`. Add more only when a compile error asks for them.
  - Turn off warnings for these two builds only (`.warnings(false)`), with a comment that the code is unmodified upstream code.
  - Emit `rerun-if-changed` for `build.rs` and `wrapper.h` only.
- [ ] **Step 4:** Copy `vendor/openmpt/LICENSE`, `include/miniz/LICENSE`, `include/minimp3/LICENSE`, and the license block at the end of `include/stb_vorbis/stb_vorbis.c` into `LICENSES/`. The first line of each file names its source path at the pinned tag.
- [ ] **Step 5:** In `Cargo.toml`, set `include` to `src/**`, `build.rs`, `wrapper.h`, `LICENSES/**`, and the vendored paths that `build.rs` reads (the directories above, `include/{miniz,minimp3,stb_vorbis}/**`, `libopenmpt/*.h`, and `LICENSE`). Run `cargo package --list -p openmpt-ext-sys --allow-dirty | wc -l`, then `cargo package -p openmpt-ext-sys --no-verify --allow-dirty`, and check that the `.crate` size is under 10 MiB. If it is over, file an issue (labels `area-openmpt`, `area-ci`) and continue.
- [ ] **Step 6:** Run `cargo nextest run -p openmpt-ext-sys`. Expected: PASS, including the bindgen layout tests. Also run `cargo clippy --all-targets --all-features -- -D warnings`. Expected: no warnings.
- [ ] **Step 7:** Run `cargo build && otool -L target/debug/mod2stems`. Expected: no libopenmpt, zlib, mpg123, ogg, or vorbis lines.
- [ ] **Step 8:** Commit: `feat(sys): build libopenmpt statically with cc`.

### Task 4: openmpt-ext core: error, string guard, Module ownership, MOD generator

**Files:**
- Create: `crates/openmpt-ext/src/{error.rs,cstr.rs,module.rs}`, `crates/openmpt-ext/tests/support/mod.rs`, `crates/openmpt-ext/tests/module.rs`
- Modify: `crates/openmpt-ext/src/lib.rs`, `crates/openmpt-ext/Cargo.toml` (dev-dependency `proptest`)

**Interfaces:**
- Produces:
  - `pub struct OpenmptError { pub op: &'static str, pub input: String, pub code: Option<i32>, pub message: String, pub hint: Option<&'static str>, pub log: Vec<String> }`, implementing `Display` and `std::error::Error`. `Display` writes `"{op} ({input}): {message}"`, then `". {hint}"` when a hint exists.
  - `pub type Result<T> = std::result::Result<T, OpenmptError>;`
  - `pub(crate) struct LibString(*const c_char)`, with `fn take(ptr) -> Option<String>`: lossy UTF-8, freed on drop.
  - `pub struct InitialCtl { pub key: String, pub value: String }`.
  - `pub struct Module`, with:
    - `pub fn from_bytes(bytes: &[u8], ctls: &[InitialCtl]) -> Result<Module>`
    - `pub fn take_log(&mut self) -> Vec<String>`
    - `pub(crate) fn check_last_error(&mut self, op: &'static str, input: impl Into<String>) -> Result<()>`
    - `pub(crate) fn core(&self) -> *mut openmpt_module`
    - `pub(crate) fn interactive(&self) -> &openmpt_module_ext_interface_interactive`
  - `Module: Send`, and `!Sync` through a `PhantomData<Cell<()>>` field.
  - Test helper: `support::ModSpec { rows: Vec<[Cell; 4]>, samples: Vec<SampleSpec>, orders: Vec<u8> }`, `support::Cell { period: u16, sample: u8, effect: u8, param: u8 }`, `support::SampleSpec { name: [u8; 22], data: Vec<i8>, volume: u8, loop_start: u16, loop_len: u16 }`, `support::build_mod(&ModSpec) -> Vec<u8>`, and `support::simple_mod() -> Vec<u8>` (4 channels, 1 pattern, 2 orders that both play pattern 0, default speed 6; at row 0, channel `c` plays sample `c + 1` with period 428; samples 1 to 4 have 256 bytes of non-zero data and names `b"kick"`, `b"snare"`, `b"hat"`, `b"bass"`).

- [ ] **Step 1: Write the MOD generator.** The ProTracker layout:
  - a 20-byte title
  - 31 sample headers of 30 bytes: name 22, length in words (u16 BE), finetune, volume, loop start in words, loop length in words (minimum 1)
  - song length u8, then 127, then 128 order bytes
  - `M.K.` at offset 1080
  - patterns of 64 rows × 4 channels × 4 bytes. Cell bytes: `[(sample & 0xF0) | (period >> 8), period & 0xFF, ((sample & 0x0F) << 4) | effect, param]`
  - signed 8-bit sample data, in order

  Pad rows to 64.
- [ ] **Step 2: Write the failing tests** in `tests/module.rs`:
  - `loads_simple_mod`: `from_bytes(&simple_mod(), &[])` is `Ok`.
  - `empty_bytes_is_error`: an error with `op == "load"` and a non-empty `message`.
  - `garbage_bytes_is_error`: same assertions for 4096 bytes of `0x5A`.
  - `unknown_initial_ctl_is_error`: `InitialCtl { key: "no.such.ctl", .. }` gives an error with `input == "no.such.ctl"`.
  - `drop_many_modules`: load and drop 1000 times with no crash.
  - `module_is_send`: move a `Module` into `std::thread::spawn` and call `take_log()` there.
  - `load_warning_is_logged`: load a Startrekker AM file (`support::startrekker_am()`, the `FLT8` magic) that names four samples in the pattern and gives no sample data. `take_log()` is not empty, and a second `take_log()` is empty. A truncated `M.K.` file does not log a warning at 0.8.10: `SampleIO::ReadSample` limits the length without a message, and the loader reports a short sample through the `warnings` metadata key instead.
- [ ] **Step 3:** Run `cargo nextest run -p openmpt-ext`. Expected: FAIL (`Module` is not defined).
- [ ] **Step 4: Implement.**
  - `from_bytes`: create a boxed log sink, a `RefCell<Vec<String>>`. Call `openmpt_module_ext_create_from_memory` with the log function, the sink pointer, `openmpt_error_func_store`, the `error` and `error_message` out-params, and a NUL-terminated `openmpt_module_initial_ctl` array. On null, build the error from the code, the message (freed), and the drained log.
  - Then get the core pointer, and call `openmpt_module_ext_get_interface` with `LIBOPENMPT_EXT_C_INTERFACE_INTERACTIVE` and `size_of::<openmpt_module_ext_interface_interactive>()`. A zero result gives an error with `op = "get interactive interface"`.
  - Then read `get_ctls` into a `HashSet<String>`, and reject any initial ctl key that is not in it.
  - `Drop`: destroy the ext object, then drop the sink (the field order and an explicit `Drop` keep this order).
  - The log callback is `extern "C" fn(*const c_char, *mut c_void)`. It pushes a lossy `String`, uses `try_borrow_mut`, and ignores a failed borrow, so that it cannot panic.
- [ ] **Step 5:** Run `cargo nextest run -p openmpt-ext`. Expected: PASS. Also run `cargo +nightly miri test -p openmpt-ext drop_many_modules`. If Miri cannot run FFI, write down the reason in the PR body and skip it. Do not leave it unstated.
- [ ] **Step 6:** Commit: `feat(ext): add Module ownership, errors, and log capture`.

### Task 5: Library info and metadata

**Files:**
- Create: `crates/openmpt-ext/src/info.rs`, `crates/openmpt-ext/tests/info.rs`

**Interfaces:**
- Consumes: `Module`, `LibString`, `check_last_error` from Task 4.
- Produces:
  - `pub fn library_info(key: LibraryInfo) -> String`, with `enum LibraryInfo { LibraryVersion, Build, SourceUrl, SourceRevision }`
  - on `Module`:
    - `fn metadata_keys(&self) -> Vec<String>`
    - `fn metadata(&self, key: &str) -> Result<String>`
    - `fn num_channels(&self) -> i32`, `num_instruments`, `num_samples`, `num_subsongs`, `num_orders`, `num_patterns`
    - `fn channel_name(&self, i: i32) -> Result<String>`, `instrument_name`, `sample_name`, `subsong_name`

- [ ] **Step 1: Write the failing tests:**
  - `library_version_is_0_8_10`: the string starts with `"0.8.10"`. It carries a source suffix, such as `"0.8.10+r25840"`, so an exact match fails.
  - `source_revision_is_pinned` (compare with `openmpt_ext_sys::PIN_SVN_REVISION`)
  - `metadata_keys_include_type_and_title`
  - `metadata_type_is_mod`
  - `counts_match_simple_mod`: 4 channels, 0 instruments, 31 samples, 1 subsong
  - `sample_name_round_trips`: the generator writes `b"kick"`, and sample index `0` gives `"kick"`. The sample getter takes a zero-based index into the name list, and the list starts at `SAMPLEINDEX` 1 in the loader.
  - `high_name_byte_is_transcoded`: a name byte `0xFF` gives `"\u{FF}ick"`. libopenmpt transcodes a module string to UTF-8 before the wrapper copies it, so the byte arrives as a valid character. The lossy path stays in `LibString::take` as a guard, and no module byte reaches it.
  - `channel_name_out_of_range`: index `4` gives an error with `input == "channel 4"`, and the message names `0..4`
  - `metadata_unknown_key_is_empty_string`: libopenmpt returns an empty string, so the wrapper returns `Ok("")`
- [ ] **Step 2:** Run. Expected: FAIL.
- [ ] **Step 3:** Implement. Each name getter checks the index against its count first. Write `fn check_index(op, kind: &str, i: i32, count: i32) -> Result<()>` here. Later tasks reuse it.
- [ ] **Step 4:** Run. Expected: PASS.
- [ ] **Step 5:** Commit: `feat(ext): wrap library info and module metadata`.

### Task 6: Ctls and render params

**Files:**
- Create: `crates/openmpt-ext/src/ctl.rs`, `crates/openmpt-ext/tests/ctl.rs`

**Interfaces:**
- Consumes: `Module`, `check_last_error`, and the ctl set loaded in Task 4 (store it as `ctls: HashSet<String>` on `Module`).
- Produces:
  - on `Module`:
    - `fn ctls(&self) -> &HashSet<String>`
    - `fn ctl_set_bool(&mut self, key: &str, v: bool) -> Result<()>`, with `ctl_set_int(i64)`, `ctl_set_float(f64)`, `ctl_set_text(&str)`
    - the matching getters `ctl_get_bool`, `ctl_get_int`, `ctl_get_float`, `ctl_get_text`, each `-> Result<_>`
    - `fn set_render_param(&mut self, p: RenderParam, v: i32) -> Result<()>`
    - `fn render_param(&mut self, p: RenderParam) -> Result<i32>`. It takes `&mut self` because it clears and then reads the stored error.
  - `enum RenderParam { MasterGainMillibel, StereoSeparationPercent, InterpolationFilterLength, VolumeRampingStrength }`, mapped to the `OPENMPT_MODULE_RENDER_*` ids

- [ ] **Step 1: Write the failing tests:**
  - `ctl_list_has_amiga_keys`: contains `render.resampler.emulate_amiga` and `render.resampler.emulate_amiga_type`
  - `ctl_list_has_subsong_and_dither`
  - `set_get_bool_round_trip` on `render.resampler.emulate_amiga`
  - `ctl_table_matches_library`: every key in the library ctl list has a type in the crate table, so a version bump that adds a ctl fails this test
  - `set_get_text_round_trip` on `render.resampler.emulate_amiga_type` with `"a500"`
  - `set_get_float_round_trip` on `play.tempo_factor` with `1.5`. The library stores the factor as a fixed-point value with 16 bits of fraction, so the read is within `1/65536` of the set value.
  - `set_get_int_round_trip` on `dither`
  - `unknown_ctl_is_error_without_library_call`: `ctl_set_bool("no.such", true)` gives an error with `input == "no.such"`
  - `wrong_type_ctl_is_error`: `ctl_set_bool("play.tempo_factor", true)` is an error
  - `render_param_round_trip`: `InterpolationFilterLength` set to 8 reads back as 8
  - `render_param_snaps_filter_length`: `InterpolationFilterLength` set to 3 reads back as 4 (libopenmpt snaps the length to a resampler mode, `libopenmpt_impl.cpp:334-350`)
  - `render_param_rejects_negative`: `InterpolationFilterLength` set to -1 is an error that names the value
- [ ] **Step 2:** Run. Expected: FAIL.
- [ ] **Step 3:** Implement. Every setter first checks that the key is in `ctls`. The `float` getter also calls `check_last_error`, because 0.0 is ambiguous.
- [ ] **Step 4:** Run. Expected: PASS.
- [ ] **Step 5:** Commit: `feat(ext): wrap ctls and render params`.

### Task 7: Subsong, position, and pattern data

**Files:**
- Create: `crates/openmpt-ext/src/position.rs`, `crates/openmpt-ext/tests/position.rs`

**Interfaces:**
- Consumes: `check_index` (Task 5), `check_last_error`.
- Produces:
  - on `Module`:
    - `fn select_subsong(&mut self, i: i32) -> Result<()>`
    - `fn selected_subsong(&self) -> i32`
    - `fn set_repeat_count(&mut self, n: i32) -> Result<()>`
    - `fn current_order(&self) -> i32`, with `current_row`, `current_pattern`, `current_speed`, and `current_tempo`
    - `fn duration_seconds(&mut self) -> Result<f64>`
    - `fn position_seconds(&mut self) -> Result<f64>`
    - `fn order_pattern(&self, order: i32) -> Result<i32>`
    - `fn pattern_num_rows(&self, pattern: i32) -> Result<i32>`
    - `fn pattern_cell(&self, pattern: i32, row: i32, channel: i32, cmd: Command) -> Result<u8>`
  - `enum Command { Note, Instrument, VolumeEffect, Effect, Volume, Parameter }`, mapped to `OPENMPT_MODULE_COMMAND_*`

- [ ] **Step 1: Write the failing tests:**
  - `select_subsong_out_of_range`: subsong `1` on `simple_mod` is an error that names `0..1`
  - `select_subsong_minus_one_is_rejected`: `-1` (all subsongs) is an error, because `docs/design.md` §4.2 does not allow the library default
  - `select_subsong_zero_is_accepted`: subsong `0` is accepted, and `selected_subsong` reads 0
  - `pattern_cell_reads_generator_values`: row 0, channel 2 has instrument `3` and a non-zero note
  - `pattern_cell_out_of_range_row`: row 64 is an error that names `0..64`
  - `pattern_cell_out_of_range_channel`: channel 4 is an error that names `0..4`
  - `pattern_cell_out_of_range_pattern`: pattern 9 is an error with `input == "pattern 9"`
  - `pattern_num_rows_is_64`
  - `pattern_num_rows_out_of_range`: pattern 9 is an error. The library returns 0 for a missing pattern, so the wrapper checks the index first.
  - `order_pattern_reads_the_order_list`: order 0 plays pattern 0
  - `duration_is_positive`
  - `position_starts_at_zero`: a fresh module reports position 0
  - `position_after_load_is_first_row`: order, row, and pattern are 0, and the speed and tempo are in range
  - `repeat_count_zero_is_accepted`: a repeat count of 0 is accepted
- [ ] **Step 2:** Run. Expected: FAIL.
- [ ] **Step 3:** Implement. `pattern_cell` checks the pattern, row, and channel ranges.
- [ ] **Step 4:** Run. Expected: PASS.
- [ ] **Step 5:** Commit: `feat(ext): wrap subsong, position, and pattern data`.

### Task 8: Float render

**Files:**
- Create: `crates/openmpt-ext/src/render.rs`, `crates/openmpt-ext/tests/render.rs`

**Interfaces:**
- Produces: on `Module`, `fn read_float(&mut self, rate: i32, buf: &mut [f32]) -> Result<usize>`. It reads interleaved stereo, returns frames, and needs an even `buf.len()`.

- [ ] **Step 1: Write the failing tests:**
  - `renders_frames`: a 4096-sample buffer gives `Ok(2048)` at the start
  - `samples_are_finite`
  - `render_is_not_silent`
  - `odd_buffer_is_error`: length 5 gives an error whose message names the even-length rule
  - `song_end_returns_zero`: with `set_repeat_count(0)`, read until `Ok(0)`, then one more read is `Ok(0)` and `current_row()` does not panic
  - `bad_rate_is_error`: rates `0` and `1_000_000` each give an `Err` that names the rate. Neither gives `Ok(0)`.
  - `position_advances_by_row`: read the song in 64-frame blocks at 48000 Hz, and record `(current_order(), current_row())` after each read. Deduplicate consecutive repeats. The sequence starts `(0,0), (0,1), (0,2)`. A 64-frame block at 48000 Hz is shorter than one row, so the loop reads 200 blocks and the start of the sequence is the value under test.
- [ ] **Step 2:** Run. Expected: FAIL.
- [ ] **Step 3:** Implement. Check that the length is even. Check the rate against `pub const SAMPLE_RATE_RANGE: RangeInclusive<i32> = 8000..=192000` (the range in the `libopenmpt.h` comment for the read functions). libopenmpt itself does not check the rate, and rate 0 reaches a division (`libopenmpt_impl.cpp:995`). Then call `openmpt_module_read_interleaved_float_stereo` with `count = len / 2`, then `check_last_error`.
- [ ] **Step 4:** Run. Expected: PASS.
- [ ] **Step 5:** Commit: `feat(ext): wrap float stereo read`.

### Task 9: Interactive mute and volume

**Files:**
- Create: `crates/openmpt-ext/src/interactive.rs`, `crates/openmpt-ext/tests/interactive.rs`

**Interfaces:**
- Produces: on `Module`:
  - `fn set_channel_mute(&mut self, ch: i32, mute: bool) -> Result<()>`
  - `fn channel_mute(&self, ch: i32) -> Result<bool>`
  - `fn set_channel_volume(&mut self, ch: i32, v: f64) -> Result<()>`
  - `fn channel_volume(&self, ch: i32) -> Result<f64>`
  - `fn set_instrument_mute(&mut self, i: i32, mute: bool) -> Result<()>`
  - `fn instrument_mute(&self, i: i32) -> Result<bool>`
  - Each calls through `self.interactive()` function pointers. A null pointer is an error that names the function.

- [ ] **Step 1: Write the failing tests.** Use a helper `render_all(module) -> Vec<f32>`.
  - `mute_all_is_silent`: every sample is `0.0`
  - `solo_channel_differs_from_full`: mute 1, 2, and 3. The output is not all zero, and it is not equal to the full render.
  - `zero_volume_acts_like_mute`: the same assertions with `set_channel_volume(ch, 0.0)`
  - `mute_getter_round_trip`
  - `volume_is_quantized`: set `0.5` and read `0.5`. Set `0.3` and read `(0.3_f64 * 64.0).round() / 64.0`.
  - `volume_out_of_range`: `1.5`, `-0.1`, and `f64::NAN` are each an error that names the value
  - `instrument_mute_uses_sample_count`: `simple_mod` has no instruments, so the valid range is the sample count. Index `0` and index `num_samples() + 1` are errors (the message names the range), and index `1` is `Ok`. The C wrapper checks `0 <= instrument < max` and then reads the 1-based slot `instrument + 1`, so the valid index starts at 0 (`libopenmpt_ext_impl.cpp:221-223`).
  - `channel_out_of_range`: channel `4` is an error on all four channel methods. The two instrument methods do not take a channel index, so they are not in this test. `instrument_out_of_range_is_not_a_channel_index` checks that sample slot 4 stays valid.
- [ ] **Step 2:** Run. Expected: FAIL.
- [ ] **Step 3:** Implement. `set_channel_volume` checks that the value is finite and in `0.0..=1.0` before the call. Its doc comment says that libopenmpt quantizes the value to 1/64 steps. `channel_volume` calls `check_last_error` after the call, because 0.0 is ambiguous.
- [ ] **Step 4:** Run. Expected: PASS.
- [ ] **Step 5:** Commit: `feat(ext): wrap channel and instrument mute and volume`.

### Task 10: Property test on load

**Files:**
- Create: `crates/openmpt-ext/tests/fuzz_load.rs`

- [ ] **Step 1:** Add `proptest = "=1.12.0"` to the crate's `[dev-dependencies]`. Write `from_bytes_never_crashes`: `proptest!` with `ProptestConfig { cases: 256, .. }` over `prop::collection::vec(any::<u8>(), 0..8192)`. The assertion: the call returns, either `Ok` or `Err`. Add a second strategy that takes `simple_mod()` and changes up to 16 random bytes, so that the input reaches the MOD loader. Add `an_empty_load_fails`, which checks that an empty buffer is an error. The property tests do not catch a missing null check, because the C wrapper frees the failed handle and returns null either way.
- [ ] **Step 2:** Run `cargo nextest run -p openmpt-ext --test fuzz_load`. Expected: PASS.
- [ ] **Step 3:** Break the code on purpose: remove the `ext.is_null()` guard from `from_bytes`. Confirm that `empty_bytes_is_error` and `garbage_bytes_is_error` fail. Revert, then touch the source file: a restored file with an older mtime leaves the broken test binary in place.
- [ ] **Step 4:** Commit: `test(ext): property test module loading`.

### Task 11: main.rs version print

**Files:**
- Modify: `src/main.rs`
- Create: `tests/version.rs`

- [ ] **Step 1:** Write the failing tests `prints_versions` and `prints_one_line`. `prints_versions` runs `env!("CARGO_BIN_EXE_mod2stems")` and asserts stdout contains `mod2stems <CARGO_PKG_VERSION>`, `libopenmpt 0.8.10`, and `r25840`, with exit code 0. `prints_one_line` checks that the output is one line.
- [ ] **Step 2:** Run. Expected: FAIL.
- [ ] **Step 3:** Implement `main` with `openmpt_ext::library_info`. Print one line: `mod2stems {ver} (libopenmpt {lib}, {url})`. The `library_version` string already holds the revision, as in `0.8.10+r25840`, so a separate `source_revision` field would print it twice. The workspace denies `print_stdout`, so `main` carries `#[expect(clippy::print_stdout, reason = "...")]`.
- [ ] **Step 4:** Run. Expected: PASS.
- [ ] **Step 5:** Commit: `feat: print tool and libopenmpt versions`.

### Task 12: Bump script

**Files:**
- Create: `scripts/bump-libopenmpt.sh`

**Interfaces:**
- Consumes: the five pin constants in `build.rs` (Task 3), `scripts/regen-bindings.sh` (Task 2).
- Produces: `scripts/bump-libopenmpt.sh <tag>`. It prints the old and new tag, commit, and revision on stdout. Diagnostics go to stderr.

- [ ] **Step 1:** Write the script.
  - Validate the tag shape `libopenmpt-[0-9]{1,3}\.[0-9]{1,3}\.[0-9]{1,3}` with `[[ =~ ]]`.
  - Run `git -C <submodule> fetch --depth 1 origin tag <tag>`, then check out the tag.
  - Read the SHA, the `git-svn-id` revision, the URL, and the commit date. Fail when no `git-svn-id` line exists.
  - Rewrite the constants with `perl -pi -e`, and check after each rewrite that the new value is in the file.
  - Run `scripts/regen-bindings.sh`.
- [ ] **Step 2:** Run `shellcheck` and `shfmt -d`. Expected: clean.
- [ ] **Step 3:** Test `scripts/bump-libopenmpt.sh libopenmpt-0.8.10`. Expected: exit 0 and no diff in `git status`, because a bump to the same tag is a no-op. Test `scripts/bump-libopenmpt.sh 0.8.10`. Expected: exit 1 with the tag-shape message. Test a tag that does not exist. Expected: exit 1, and the submodule stays on its pin.
- [ ] **Step 4:** Commit: `feat(scripts): add libopenmpt pin bump script`.

### Task 13: CI, Dependabot, prek, README

**Files:**
- Create: `.github/workflows/ci.yml`, `.github/dependabot.yml`, `.pre-commit-config.yaml`
- Modify: `README.md`

- [ ] **Step 1:** Look up current versions and SHAs of `actions/checkout`, `Swatinem/rust-cache`, `taiki-e/install-action` (for nextest, cargo-deny, and the bindgen CLI), and the prek hook repos.
- [ ] **Step 2:** Write `ci.yml`. Set `permissions: contents: read`. Trigger on `push` to `main` and on `pull_request`.
  - Job `test` with matrix `[macos-latest, ubuntu-latest]`:
    - checkout with `submodules: true`, `fetch-depth: 1`, `persist-credentials: false`
    - rust-cache with `cache-workspace-crates: true` and a key that includes `hashFiles('crates/openmpt-ext-sys/build.rs')`
    - fmt, clippy, nextest, `cargo deny check`, `cargo package --list -p openmpt-ext-sys --allow-dirty`, and `cargo run --release`
    - on Linux, `ldd target/release/mod2stems`; on macOS, `otool -L target/release/mod2stems`. Then a `! grep -Ei 'openmpt|libz|mpg123|ogg|vorbis'` on that output.
  - Job `bindings` on ubuntu: install libclang and the pinned bindgen CLI, then run `scripts/regen-bindings.sh --check`.
- [ ] **Step 3:** Write `dependabot.yml`: `cargo` and `github-actions`, weekly, `cooldown: default-days: 7`, one group per ecosystem, no `gitsubmodule` entry.
- [ ] **Step 4:** Write `.pre-commit-config.yaml` with hooks for cargo fmt, cargo clippy, shellcheck, shfmt, actionlint, and zizmor. Run `prek install && prek run --all-files`. Expected: pass.
- [ ] **Step 5:** Run `actionlint && zizmor .github/workflows/ci.yml`. Expected: no findings.
- [ ] **Step 6:** Add a "Build" section to `README.md` (one sentence per line): clone with `--recurse-submodules` or run `git submodule update --init --depth 1`, then `cargo build`. A C++17 compiler is required (on macOS, the Xcode command line tools). Also add a "License" section: `Apache-2.0`, with libopenmpt and the decoders under their own licenses in `crates/openmpt-ext-sys/LICENSES/`.
- [ ] **Step 7:** Commit: `ci: add macOS and Linux CI with cached libopenmpt build` and `docs(readme): add build and license sections` (two commits).
- [ ] **Step 8:** After the push, check the second CI run's log: the `openmpt-ext-sys` build step must show no C++ compile (the cache hit). If it compiles again, fix the cache key or the cache settings before the PR is done.
- [ ] **Step 9:** When Step 8 fails, the cause is the mtime check: a fresh checkout writes `build.rs` and `wrapper.h` with the current time, so cargo reruns the build script even with a warm `target/`. Add `mozilla-actions/sccache-action` and export `SCCACHE_GHA_ENABLED=true` and `RUSTC_WRAPPER=sccache`. Check the second run: the job log must show sccache hit counts for the C++ objects.

### Task 14: Design doc and issue updates

**Files:**
- Modify: `docs/design.md` (Purpose, §1.2, §3, §6, §8)

- [ ] **Step 1:** Make the five changes listed in the spec's "Changes to docs/design.md". Write each changed sentence on its own line.
- [ ] **Step 2:** Run `git grep -n -i 'pkg-config\|system-installed\|minimum libopenmpt' docs/design.md`. Expected: no output.
- [ ] **Step 3:** Commit: `docs(design): describe the static vendored libopenmpt build`.
- [ ] **Step 4:** Update the #2 issue body: replace the pkg-config item with the vendored build items, and replace the "#4 decides the minimum version" line with "pinned to libopenmpt-0.8.10". Update the #4 issue body: remove the minimum version part, and keep profile capability detection. Write each paragraph as one line.
