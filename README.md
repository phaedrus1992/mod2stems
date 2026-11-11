Mod 2 Stems
===========

Exports per-channel stems from tracker modules supported by libopenmpt, prioritizing
faithful playback with explicit interpolation and hardware-emulation options.

Written in Rust, linking to C/C++ libraries as necessary.

See the [functional design](docs/design.md) for the planned behavior and fidelity
validation requirements.

Build
-----

Clone the repository with its submodules, or fill the submodule after a plain clone:

```sh
git clone --recurse-submodules https://github.com/phaedrus1992/mod2stems.git
git submodule update --init --depth 1
```

Then build the command:

```sh
cargo build --release
```

The build compiles the vendored libopenmpt with a C++17 compiler.
On macOS, install the Xcode command line tools for that compiler.
The first build takes a few minutes, because it compiles the whole library.

The build links libopenmpt and its decoders statically.
The released binary needs no libopenmpt, zlib, mpg123, ogg, or vorbis at run time.

Run `cargo run` to print the tool version and the libopenmpt version.

License
-------

The code in this repository is Apache-2.0.
See [LICENSE](LICENSE).

The vendored libopenmpt and its decoders keep their own licenses.
See [crates/openmpt-ext-sys/LICENSES/](crates/openmpt-ext-sys/LICENSES/).
