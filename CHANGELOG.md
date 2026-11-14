# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- Link libopenmpt and its decoders statically, so the binary needs no system library at run time [feat:sys]
- Print the tool version and the libopenmpt version [feat]
- Resolve a rendering profile (`compatible`, `amiga`, `clean`) and one output sample rate before any render [feat:profile]
- Check that an `amiga` profile applies by comparing a plain render with an emulated one, and fail the export when it does not [feat:profile]
- Cap a profile render at a maximum duration, so a module that never ends fails the export [feat:profile]
- Resolve one playback policy per export: the subsong, one pass of the order list, and the render duration limit, each read back from the module [feat]
- Align every stem and mix to one common end, the longest completed render, and zero-pad a shorter render at the tail [feat]
- Add an optional mono output, which is the mean of each rendered left and right pair [feat]
- Write one audio file per tracker channel, silent channels included, into a new directory that the tool publishes in one rename [feat]
- Write 32-bit float WAV by default, which keeps every finite sample, or integer FLAC at 16 or 24 bits [feat]
- Apply one common gain to every stem and to every mix, and record the resolved configuration in the manifest [feat]
- Write a JSON manifest that names the input by SHA-256, the resolved settings, the library warnings, the module metadata, and one entry per file [feat]
- Write the sum of the channel stems, and the unmuted render of the whole song, when the caller asks for them [feat]
- Refuse an integer export whose peak is above the format range, and name the gain that would fit, instead of normalizing on its own [feat]
- Refuse an output directory that already exists [feat]
- Refuse an input file above 2 GiB before the tool reads it [feat]
- Apply optional TPDF dither at the final integer conversion [feat]
- Cancel a run with one signal at any stage before the publish, and report the staging directory that the run kept [feat]

### Fixed

- Keep a failed publish report from claiming that nothing published [fix]
- Sum the mix from the stereo channel renders, so a mono mix is downmixed once [fix]
- Report the output frame count in the export README, not the render count [fix]
- Report a failed manifest listing with its staging path and its cause [fix]
- Check the cancel flag at every stage before the rename [fix]
- Name the sample index of a non-finite value, instead of reading a broken render as a quiet one [fix]
- Refuse a timeline that is longer than any render can reach [fix]
- Write a published export only when the manifest names every file and the directory holds nothing else [fix]
- Refuse the `clean` profile for an MT2 module, because a drum in that format can keep the nearest resampling mode and the render would not use windowed-sinc [fix:profile]
- Report a module with no type as an error for the `clean` profile, instead of treating it as an unknown format [fix:profile]
- Report a length mismatch between two plain renders as `NondeterministicLength`, which names both lengths [fix:profile]
- Fail a render that is longer than the common end, instead of reporting zero padding [fix]
- Fail a render duration limit whose frame count does not fit in memory, instead of saturating to a limit that never fires [fix]
- Read the selected subsong and the repeat count fallibly, so a failed read cannot pass as a value [fix]

[Unreleased]: https://github.com/phaedrus1992/mod2stems/commits/main
