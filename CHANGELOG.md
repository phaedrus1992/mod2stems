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

[Unreleased]: https://github.com/phaedrus1992/mod2stems/commits/main
