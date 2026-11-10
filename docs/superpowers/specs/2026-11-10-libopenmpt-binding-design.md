# libopenmpt binding and crate foundation — design (#2)

## Intent

Issue #2 builds the base that every later epic stands on.
It gives a Rust workspace that compiles a pinned copy of stock libopenmpt from source, links it statically, and wraps the parts of the C API that `docs/design.md` needs behind a safe Rust type.

A release binary must run on a machine that has no libopenmpt installed.
That is the reason for the static build.
The playback must stay stock upstream libopenmpt, because the fidelity contract in `docs/design.md` §1 depends on it.

No existing crate covers this need.
The `openmpt`, `openmpt-sys`, and `libopenmpt-sys` crates bind only `libopenmpt.h`, so they have no `interactive` interface (channel mute, channel volume, instrument mute).
The full survey is in the comment on #2.

The two binding crates must be publishable to crates.io later, under names that are different from those three crates.
Publishing is not part of #2.

## Goals

1. A Cargo workspace that builds on macOS (Apple Silicon) locally and in CI, and on Linux in CI.
2. libopenmpt source from a git submodule, pinned to an upstream release tag.
3. A `build.rs` that compiles libopenmpt with the `cc` crate and links it statically, with no system libopenmpt, pkg-config, zlib, mpg123, ogg, or vorbis.
4. Raw bindings for `libopenmpt.h` and `libopenmpt_ext.h`, generated with bindgen and committed, in the crate `openmpt-ext-sys`.
5. A safe wrapper crate, `openmpt-ext`, that covers every capability in the #2 checklist and in `docs/design.md` §3.
6. Library warnings and errors captured as data that a caller can read, instead of text on stderr.
7. CI that runs the full Rust gate and caches the C++ build.
8. `docs/design.md` updated to describe the static vendored build.

## Non-goals

- Windows. This is #14.
- Release artifacts, packaging, signing, and notarization. This is #15.
- Publishing to crates.io. The crates get publishable metadata, but they keep `publish = false` until a later decision.
- The fixture corpus and the shared acceptance harness. This is #3. Issue #2 uses only small modules that the tests generate.
- Profile resolution (#5), isolation logic (#6), and CLI behavior (#10). The wrapper gives the primitives. It does not choose a silencing method, a profile, or an end policy.
- Detection of the module playback type (S3M rules). The stock API does not expose it directly. #4 and #6 own that question. The wrapper exposes the metadata that they will need.
- A dynamic-link or system-libopenmpt build option. Add one only when a real need appears.
- Automatic updates of the libopenmpt pin. A bump is a manual script run (see "Bumping libopenmpt").

## Anti-goals

These results are failures, even if the build is green:

- A patched or forked libopenmpt. The submodule stays at an unmodified upstream tag (`docs/design.md` §1.2).
- `cargo build` that touches the network.
- A wrapper call that hides a library failure, such as a setter whose result is discarded, or a library string that leaks.
- Bindings in the repository that do not match the pinned headers.
- A safe API that lets a caller cause undefined behavior, such as a read into uninitialized memory (the defect in the `openmpt` crate).
- Anything in `openmpt-ext-sys` or `openmpt-ext` that depends on mod2stems. They must stand alone, so that they can be published.

## Decisions from the user

1. The source is a git submodule of `https://github.com/OpenMPT/openmpt`, at tag `libopenmpt-0.8.10` (commit `dc4ddcf68`). The GitHub mirror has the release tags, and upstream does not tag its release tarballs.
2. `build.rs` compiles libopenmpt with the `cc` crate, and the result links statically.
3. Compressed formats use the decoders bundled in the libopenmpt tree: miniz, minimp3, and stb_vorbis.
4. The license notices ship with the binary: libopenmpt BSD-3-Clause, plus the notices of the bundled decoders.
5. CI caches the compiled library.
6. CI runs macOS and Linux. Windows is #14.
7. Local cross-compile is not a goal.
8. The binding crates get their own names, different from the existing public openmpt crates, so that they can be published later.

## Upstream facts at the pinned tag

All citations are `openmpt/<path>:<line>` at `libopenmpt-0.8.10` (`dc4ddcf68`).

- The tag commit message carries `git-svn-id: https://source.openmpt.org/svn/openmpt/branches/OpenMPT-1.32@25840`. The upstream Makefile reads the revision from this line (`Makefile:1478-1484`). With no revision, the version string ends in `+rUNKNOWN`.
- The library source set is in `Makefile:1523-1551`:
  - `src/openmpt/{all,base,logging,random,fileformat_base,soundbase,soundfile_data}/*.cpp`
  - `common/*.cpp`
  - `soundlib/*.cpp`, `soundlib/plugins/*.cpp`, `soundlib/plugins/dmo/*.cpp`
  - `sounddsp/*.cpp`
  - `libopenmpt/*.cpp`, which includes `libopenmpt_ext_impl.cpp`
  - `unarchiver/*.cpp` is built only with `HACK_ARCHIVE_SUPPORT=1` (`Makefile:1543-1546`). Leave it out.
- The bundled decoders are in `Makefile:1553-1633`:
  - `include/miniz/miniz.c` with `-DMPT_WITH_MINIZ`
  - `include/minimp3/minimp3.c` with `-DMPT_WITH_MINIMP3`
  - `include/stb_vorbis/stb_vorbis.c` with `-DMPT_WITH_STBVORBIS -DSTB_VORBIS_NO_PULLDATA_API -DSTB_VORBIS_NO_STDIO`
  - All three need `-Iinclude`.
- Every library object needs `-DLIBOPENMPT_BUILD` (`Makefile:1520`).
- A static build defines neither `LIBOPENMPT_BUILD_DLL` nor `LIBOPENMPT_USE_DLL`. `LIBOPENMPT_API` then resolves to the public-visibility helper (`libopenmpt/libopenmpt_config.h`, the selector after the helper definitions).
- The minimum compilers are C++17, GCC 7.1, and Clang 6 (`doc/libopenmpt/dependencies.md:24-32`).
- The `interactive` interface id is `"interactive"` (`libopenmpt/libopenmpt_ext.h:144`). The struct is at `libopenmpt_ext.h:147`. The interface has `set_channel_volume`, `set_channel_mute_status`, `set_instrument_mute_status`, and their getters (`:233-280`). The `interactive2` and `interactive3` interfaces also exist (`:312`, `:394`).
- `openmpt_module_ext_get_interface` returns an `int`, and it fills a caller-supplied struct of a given size (`libopenmpt_ext.h:101`).
- `get_ctls` lists `load.skip_patterns`, `load.skip_plugins`, `load.skip_samples`, `load.skip_subsongs_init`, `play.at_end`, `play.pitch_factor`, `play.tempo_factor`, `render.opl.volume_factor`, `render.resampler.emulate_amiga`, `render.resampler.emulate_amiga_type`, `seek.sync_samples`, `subsong`, and `dither` (`libopenmpt/libopenmpt_impl.cpp:1721-1734`).
- Initial ctls with an unknown key are dropped with no error (`libopenmpt_impl.cpp:483-485`, `throw_if_unknown=false`). The typed ctl setters reject a value of the wrong type (`:1814`, `:1860`, `:1898`).
- `set_channel_volume` rejects values outside 0..1, and it stores `round(v * 64)` (`libopenmpt_ext_impl.cpp:186`). The getter returns the quantized value.
- In a module with no instruments, `set_instrument_mute_status` checks the index against the sample count (`libopenmpt_ext_impl.cpp:221-223`).
- `read_interleaved_float_stereo` returns 0 at the song end and also on an error. The `double` getters return 0.0 on an error.
- libopenmpt is thread-aware, and its individual objects are not thread-safe (`libopenmpt/libopenmpt.h:104-109`).
- Errors: `openmpt_error_func_store` keeps the last error on the module, and `openmpt_module_error_get_last` and `openmpt_module_error_get_last_message` read it (`libopenmpt.h:465`, `:769-801`).

## Workspace layout

The crate names are the user's decision.
The rest of the layout is the builder's call, written down here:

```
Cargo.toml                    workspace root + the mod2stems binary crate
src/main.rs                   stub: prints version info only (see "main.rs")
crates/openmpt-ext-sys/
  Cargo.toml                  links = "openmpt", publish = false
  build.rs                    compiles vendor/openmpt with cc
  src/lib.rs                  includes the committed bindings
  src/bindings.rs             bindgen output, committed
  wrapper.h                   includes libopenmpt.h and libopenmpt_ext.h
  vendor/openmpt/             git submodule, shallow
  LICENSES/                   third-party notices
crates/openmpt-ext/
  Cargo.toml                  depends on openmpt-ext-sys, publish = false
  src/lib.rs                  safe wrapper; the only crate with unsafe code besides -sys
scripts/regen-bindings.sh     regenerates bindings.rs
scripts/bump-libopenmpt.sh    moves the pin to a new upstream tag
```

Names: `openmpt-ext-sys` and `openmpt-ext` were free on crates.io on 2026-11-10.
The "ext" names the difference from the other crates: they bind `libopenmpt_ext.h` too.

The reasons for the split:

1. The C++ compile and the raw `unsafe` surface stay in one crate. A change to mod2stems then does not re-run the C++ build.
2. Each crate can be published on its own later.

`links = "openmpt"` stays the same as in the other `-sys` crates.
Cargo then refuses a build graph that contains two crates that link libopenmpt, which would fail at link time anyway.

Both crates get full publish metadata now: description, license, repository, and `include` lists that ship the vendored source the build needs.

License (user decision): all of our own code is `MIT OR Apache-2.0`. This covers mod2stems, `openmpt-ext`, and our code in `openmpt-ext-sys`. Both licenses are compatible with libopenmpt BSD-3-Clause.
The packaged `openmpt-ext-sys` crate also holds the vendored libopenmpt source and its bundled decoders. So its `license` field is `(MIT OR Apache-2.0) AND BSD-3-Clause`, plus the bundled decoders: miniz is MIT (`include/miniz/LICENSE`), minimp3 is CC0-1.0 (`include/minimp3/LICENSE`), and stb_vorbis is MIT OR Unlicense (the end of `include/stb_vorbis/stb_vorbis.c`). The full expression is `(MIT OR Apache-2.0) AND BSD-3-Clause AND MIT AND CC0-1.0 AND (MIT OR Unlicense)`, and `cargo deny` must allow each one.
The repository root gets `LICENSE-MIT` and `LICENSE-APACHE`.
`cargo package --list` for `openmpt-ext-sys` must include every file that `build.rs` reads, and it must stay under the crates.io size limit (the builder checks this, and files an issue if it does not fit).

mod2stems itself uses `#![forbid(unsafe_code)]`.

## Build script

1. Fail with a clear message when `vendor/openmpt` is empty. The message names the fix: `git submodule update --init --depth 1`.
2. Collect the source files from the directory list above, with a sorted glob. Use a list in code, not a parse of the upstream Makefile.
3. Compile the C++ files with `-std=c++17` (`/std:c++17` is for #14). Compile the three C decoder files as C.
4. Set `LIBOPENMPT_BUILD`, `MPT_WITH_MINIZ`, `MPT_WITH_MINIMP3`, `MPT_WITH_STBVORBIS`, `STB_VORBIS_NO_PULLDATA_API`, and `STB_VORBIS_NO_STDIO`.
5. Set `MPT_SVNURL`, `MPT_SVNVERSION`, and `MPT_SVNDATE` from named constants in `build.rs`. Upstream sets these values from the command line (`Makefile:1470`, `:1484`). The build has no `git` call.
6. Keep three pin constants together in `build.rs`: the upstream tag name, the full commit SHA, and the SVN revision.
7. Add the include paths `vendor/openmpt`, `vendor/openmpt/src`, `vendor/openmpt/common`, and `vendor/openmpt/include`. The builder confirms the exact set against the upstream Makefile and the compile errors.
8. Turn off the warnings of third-party code in this build. They are upstream code, and the zero-warnings rule covers our code. The builder must not use these flags to hide a warning in our own code.
9. Use parallel compile (the `cc` `parallel` feature).
10. Emit `cargo:rerun-if-changed` for `build.rs` and `wrapper.h`. Do not watch every vendored file. A pin change also changes the pin constants in `build.rs`, so it rebuilds.
11. Link the C++ standard library through `cc`'s default for the target.
12. Make the build fully offline.

**Pin check.** A test compares the `build.rs` commit constant with the submodule `HEAD` SHA (`git -C crates/openmpt-ext-sys/vendor/openmpt rev-parse HEAD`). It also compares the revision constant with the `git-svn-id` line in the `HEAD` commit message.
The check must not use `git describe --tags`, because a shallow submodule checkout has no tags.
When the submodule is not a git checkout (for example, a packaged crate), the test skips with a message that names the reason.

## Bumping libopenmpt

Dependabot cannot follow upstream release tags. Its submodule updater follows a branch, and the commits between upstream tags are not releases.
So the submodule has no Dependabot entry.

`scripts/bump-libopenmpt.sh <tag>` does the bump:

1. Fetch the tag into the submodule and check it out.
2. Read the commit SHA and the `git-svn-id` revision.
3. Write the three pin constants in `build.rs`.
4. Run `scripts/regen-bindings.sh`.
5. Print what changed.

The script validates the tag shape (`libopenmpt-<major>.<minor>.<patch>`), and it fails when the commit has no `git-svn-id` line.
It follows the repository bash rules (`set -euo pipefail`, shellcheck, shfmt, `--help`).

## Bindings

- `wrapper.h` includes `libopenmpt/libopenmpt.h` and `libopenmpt/libopenmpt_ext.h`.
- `scripts/regen-bindings.sh` runs bindgen (the CLI, pinned version) over `wrapper.h` with an allowlist of `openmpt_.*` and `OPENMPT_.*`, and writes `crates/openmpt-ext-sys/src/bindings.rs`. The script follows the repository bash rules.
- Commit the bindings. The normal build then needs no libclang.
- Keep the layout tests that bindgen generates. They check the size and alignment of `openmpt_module_ext_interface_interactive` and the other structs.
- A CI job runs the script and fails when the result differs from the committed file.

## Safe wrapper surface (`openmpt-ext`)

The wrapper is one owning type, called `Module` here (the name is the builder's call).
The list below is the required capability. The exact Rust signatures are the builder's call.

**Ownership**

- `Module::from_bytes(bytes, initial_ctls)` calls `openmpt_module_ext_create_from_memory`. It connects the log callback and the store error callback.
- Libopenmpt ignores an unknown initial ctl key. So `from_bytes` checks each initial ctl key itself, and it returns an error for an unknown key. The builder can either append the `!` suffix (the upstream syntax for "fail on unknown key"), or create the module and then check each key against `get_ctls`. Use whichever one gives a clear error that names the key.
- On failure, `from_bytes` returns an error with the libopenmpt error code, the message, and the captured log lines. Libopenmpt frees its own error message with `openmpt_free_string`.
- `Module` keeps the `openmpt_module_ext` pointer and the borrowed core pointer from `openmpt_module_ext_get_module`.
- `Drop` calls `openmpt_module_ext_destroy` exactly once. The core pointer is never destroyed on its own.
- `from_bytes` also gets the `interactive` interface. When the call fails, `from_bytes` fails with an error that names the missing interface. A module without channel muting is not usable.
- `Module` is `Send` and is not `Sync`. The `SAFETY` comment cites `libopenmpt.h:104-109`.
- The log sink is a heap value that the module owns. Its pointer is the callback user pointer. `Drop` destroys the libopenmpt object before it drops the sink.
- The callbacks are `extern "C"` functions. They must not panic (an unwind out of `extern "C"` aborts the process). They copy the message into the sink, and they do nothing else.

**Strings**

Every `const char *` that libopenmpt returns is copied into a `String` (lossy UTF-8) and then freed with `openmpt_free_string`. A small guard type does this, so that no early return can leak a string.

**Errors**

- Every setter and every call with an `int` result returns `Result<_, OpenmptError>`.
- Some calls cannot signal an error in their return value: `read_interleaved_float_stereo` (0 means the song end or an error) and the `double` getters (0.0 means a value or an error). After each of these calls, the wrapper reads the stored error too. It returns an error when one is stored.
- On a failure, the wrapper reads `openmpt_module_error_get_last` and `openmpt_module_error_get_last_message`, then clears the error.
- `OpenmptError` holds the operation, the input (for example the channel index or the ctl key), the libopenmpt code and message when they exist, and a fix hint where one is known (`CLAUDE.md` error rule).

**Capabilities**

| Group | Wrapped calls |
|-------|---------------|
| Library info | `openmpt_get_library_version`, `openmpt_get_string` (`library_version`, `build`, `source_url`, `source_revision`) |
| Metadata | metadata keys and values, channel names, instrument names, sample names, and their counts; number of subsongs, orders, and patterns; subsong names |
| Song position | `select_subsong`, `get_selected_subsong`, `set_repeat_count`, `get_current_order`, `get_current_row`, `get_current_pattern`, `get_current_speed`, `get_current_tempo`, `get_duration_seconds`, `get_position_seconds` |
| Pattern data | `get_order_pattern`, `get_pattern_num_rows`, `get_pattern_row_channel_command` (with a Rust enum for the `OPENMPT_MODULE_COMMAND_*` indices) |
| Render params | `set_render_param` and `get_render_param`, with a Rust enum for the `OPENMPT_MODULE_RENDER_*` ids |
| Ctls | `get_ctls`, and typed get and set for boolean, integer, floating-point, and text ctls |
| Float render | `read_interleaved_float_stereo(rate, &mut [f32]) -> frames`. The slice must have an even length (the wrapper returns an error otherwise). The frame count is `len / 2`. The call writes only into initialized memory. |
| Interactive | `set_channel_mute_status`, `get_channel_mute_status`, `set_channel_volume`, `get_channel_volume`, `set_instrument_mute_status`, `get_instrument_mute_status` |
| Diagnostics | `take_log() -> Vec<String>`, which drains the captured log lines |

Before each call, the wrapper checks the channel, instrument, pattern, row, and subsong indices against the counts that libopenmpt reports. It returns an error that names the index and the valid range. Libopenmpt must never get an index that is out of range.
For `set_instrument_mute_status` and its getter, the valid range is the instrument count. In a module with no instruments, it is the sample count, which matches libopenmpt.

`set_channel_volume` checks that the value is finite and in 0..1 before the call. Its documentation says that libopenmpt quantizes the value to steps of 1/64.

Optional controls: the wrapper reads `get_ctls` once, at load. A typed ctl setter for a key that is not in that list returns an error, and it does not call libopenmpt. Profiles (#5) depend on this check.

## main.rs

`main.rs` prints the tool version and the libopenmpt `library_version`, `source_url`, and `source_revision`, and then exits.
This proves that the static link works in the final binary.
It is not the CLI. #10 replaces it.

## Licenses

- Copy `vendor/openmpt/LICENSE` into `crates/openmpt-ext-sys/LICENSES/libopenmpt.txt`, together with the license text of each bundled decoder (miniz, minimp3, stb_vorbis). The builder takes each notice from the file headers in the submodule, and records the source path of each one.
- Issue #2 adds the files only. The binary does not embed them. #15 puts them into the release archives. BSD-3-Clause asks for the notice in the materials that come with the distribution, and a file in the archive meets that.
- The `cargo deny` license allowlist covers the Rust dependencies. A test checks that the `LICENSES/` files exist, so that a submodule bump cannot remove them by accident.

## Testing

Unit and integration tests run with `cargo nextest run`. The tests do not use the fixture corpus (#3).

- **Module generator.** A test helper in `openmpt-ext` builds a minimal 4-channel ProTracker MOD (`M.K.`) in memory. The parameters are the channel notes, the instrument numbers, and the effects. The sample data is short, with an exact pattern. A MOD is enough to exercise every wrapped call. A MOD has no instruments, so it also tests the sample-count range of the instrument mute.
- **Load and lifetime:**
  - a load from valid bytes
  - empty bytes and random bytes, which must return an error with a message
  - an unknown initial ctl key, which must return an error that names the key
  - a drop with no double free (also under `cargo miri` if the FFI allows, otherwise skip with a reason)
  - log lines that `take_log` returns after a load that warns
- **Strings:** metadata, channel names, and instrument names round-trip through the guard.
- **Errors:**
  - out-of-range channel, instrument, subsong, and pattern indices
  - an unknown ctl key, and a ctl value of the wrong type
  - an odd-length read buffer
  - a channel volume outside 0..1, and a non-finite channel volume
  - each one returns the specific error
- **Render:**
  - a render of the generated MOD that gives a nonzero frame count
  - finite samples
  - a read count that drops to 0 at the song end with `set_repeat_count(0)`, with no error stored
- **Interactive:**
  - muting every channel gives digital silence
  - muting all channels but one gives a nonzero output that differs from the full render
  - `set_channel_volume(ch, 0.0)` acts the same way
  - mute getters return what the setters set; the volume getter returns the value quantized to 1/64 (test with values on and between the steps)
- **Position:** `get_current_order` and `get_current_row` advance through the generated pattern in the expected order when the read blocks are short.
- **Pattern data:** `get_pattern_row_channel_command` returns the note and the instrument that the generator wrote.
- **Pin check:** the commit and revision check from "Build script".
- **Property test:** for random byte input, `from_bytes` never crashes. It returns either a module or an error. Use `proptest` with a bounded case count.

## CI

- Workflow on `push` and `pull_request`, with jobs on `macos-latest` and `ubuntu-latest`.
- Checkout with `submodules: true` and a shallow fetch.
- Steps: `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo nextest run`, `cargo deny check`, `cargo package --list -p openmpt-ext-sys` (to prove the package holds the vendored source), and a run of the built binary (the `main.rs` version print).
- A separate Linux job does the bindings freshness check. It installs libclang and the pinned bindgen CLI.
- Cache: `openmpt-ext-sys` is a workspace member, and the usual Rust cache action does not cache workspace crates by default. Turn on workspace crate caching (for `Swatinem/rust-cache`, `cache-workspace-crates: true`), or cache `target/` with `actions/cache`. Key the cache on `Cargo.lock` and the `build.rs` pin constants. A CI run with a warm cache must not recompile libopenmpt. The builder confirms this in the job log.
- Actions pinned to a full SHA with a version comment. Scan with `zizmor` and `actionlint`.
- Dependabot for Cargo and GitHub Actions, with a 7-day cooldown and grouped updates. No submodule entry (see "Bumping libopenmpt").
- Pin the toolchain with `rust-toolchain.toml` (stable).
- The builder looks up current stable versions for every crate and action. Pin each version exactly.

## Local tooling

- `prek` hooks: fmt, clippy, shellcheck and shfmt on `scripts/`, actionlint, and zizmor.
- `README.md` gets a build section: `git clone --recurse-submodules`, or `git submodule update --init --depth 1`, then `cargo build`. It names the compiler need (a C++17 compiler, which Xcode command line tools provide on macOS).

## Changes to docs/design.md

Make these changes in the same PR:

1. **Purpose:** "system-installed `libopenmpt`" becomes the vendored, statically linked stock libopenmpt.
2. **§1.2:** add one sentence. The vendored copy is unmodified upstream source at a pinned tag, so it is not a custom fork.
3. **§3:** replace "Link dynamically to system `libopenmpt`, discovered through pkg-config … No vendored renderer or C++ library build is required" with the static vendored build: the pinned submodule tag, the `cc` compile, the bundled decoders, and no system dependency. Name the `openmpt-ext-sys` and `openmpt-ext` crates.
4. **§6:** the `openmpt.rs` row becomes the `openmpt-ext` crate.
5. **§8:** the "minimum libopenmpt version" item goes away, because the build pins one version. Keep the profile capability detection item, because the runtime `get_ctls` check still applies.

Also update:

- the #2 issue checklist (it still says pkg-config)
- the #4 issue checklist (remove the minimum version item)
- the #4 reference in the #2 issue

## Acceptance

1. A fresh clone with submodules runs `cargo build` on macOS with no Homebrew libopenmpt, and with no network after the clone.
2. `otool -L` (macOS) and `ldd` (Linux) on the binary show no libopenmpt, zlib, mpg123, ogg, or vorbis.
3. The binary prints libopenmpt `0.8.10` and revision `25840`.
4. Every capability in the wrapper table has at least one test, and every error path in "Testing" has a test.
5. CI is green on macOS and Linux, including the bindings freshness job, and a warm-cache run does not recompile libopenmpt.
6. `cargo package --list -p openmpt-ext-sys` lists the vendored source that the build needs.
7. The `docs/design.md` changes, the `LICENSES/` files, and the issue checklist updates are done.

## Left to the builder

- Rust names and signatures in the wrapper.
- Error enum shape, and whether to use `thiserror` or a manual `Display` (follow `rules/dependencies-and-reuse.md`).
- The exact include path set and the warning-suppression flags for vendored code.
- The method for the unknown initial ctl check.
- The cache action and the dependency versions (look up current stable versions).

## Calls made without the user

These choices were not discussed with the user. They are easy to change at review:

1. The crate names `openmpt-ext-sys` and `openmpt-ext`, and the split of the safe wrapper into its own crate.
2. `links = "openmpt"`, which keeps the binding crates exclusive with the other libopenmpt `-sys` crates.
3. A manual bump script instead of Dependabot for the submodule.
4. Third-party license files in the repository, with no embedded copy in the binary.
5. `main.rs` as a version-print stub.
6. The minimum libopenmpt version item removed from the design and from #4.
7. Tests with a generated MOD, not the fixture corpus.
