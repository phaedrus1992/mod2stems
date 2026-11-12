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

### Fixed

- Refuse the `clean` profile for an MT2 module, because a drum in that format can keep the nearest resampling mode and the render would not use windowed-sinc [fix:profile]
- Report a module with no type as an error for the `clean` profile, instead of treating it as an unknown format [fix:profile]
- Report a length mismatch between two plain renders as `NondeterministicLength`, which names both lengths [fix:profile]

[Unreleased]: https://github.com/phaedrus1992/mod2stems/commits/main
