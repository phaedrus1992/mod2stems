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

Usage
-----

```sh
mod2stems [OPTIONS] [INPUT] [OUTPUT]
```

`INPUT` is the tracker module. `OUTPUT` is the directory to write into, and it must not exist.
A run with no arguments prints the version.

The command writes one audio file per tracker channel, silent channels included.
The default format is 32-bit float WAV, which never clips.
`--format flac16` and `--format flac24` write integer FLAC instead.
An integer export whose peak is above the format range fails before any file is written.
It reports the peak and the gain that would fit, and it never normalizes on its own.

| Option | Default | Meaning |
| --- | --- | --- |
| `--profile` | `compatible` | The rendering profile: `compatible`, `amiga`, or `clean` |
| `--amiga-model` | none | The Amiga filter model, for the `amiga` profile |
| `--sample-rate` | 48000 | The output rate in Hz |
| `--subsong` | 0 | The subsong to render |
| `--mono` / `--stereo` | stereo | The channel layout of every output |
| `--format` | `float` | `float`, `flac16`, or `flac24` |
| `--gain` | 1 | The one gain that every output receives |
| `--dither` | `none` | The dither of the final integer conversion |
| `--mix` | off | Also write the sum of the channel stems |
| `--reference-mix` | off | Also write the unmuted render of the whole song |

The export writes into a staging directory beside `OUTPUT`, checks that every file the manifest names is there and that the directory holds nothing else, and then renames the directory to `OUTPUT`.
A failure or a cancel never publishes a partial set, and it reports the staging directory and the files it kept.

The export directory holds:

- `manifest.json` — the resolved configuration, the gain, the module metadata, the library warnings, and one entry per file.
- `README.txt` — a short label of the export, for a person.
- One audio file per channel, named from the channel index and the sanitized channel name.
- The optional mix files.

Pair stems are not written yet. `--instrument-channel-stems` is refused with an error that names the reason.

License
-------

The code in this repository is Apache-2.0.
See [LICENSE](LICENSE).

The vendored libopenmpt and its decoders keep their own licenses.
See [crates/openmpt-ext-sys/LICENSES/](crates/openmpt-ext-sys/LICENSES/).
