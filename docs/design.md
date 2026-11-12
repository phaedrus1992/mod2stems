# mod2stems — Functional Design

## Purpose

`mod2stems` exports per-channel stems from tracker modules supported by a
vendored, statically linked stock `libopenmpt`, with optional per-instrument/per-channel exports
and mixes. Its priority is faithful
playback: preserve the module's timing, pitch, instruments, envelopes, panning,
and supported effects while producing aligned audio suitable for a DAW.

The default stem is a **tracker channel** (a pattern column). An optional separate
export set splits each channel into **channel–instrument pairs**. For example, a
channel triggering three distinct instruments produces three pair stems, with
all occurrences of each instrument collected into its own stem. Repeated triggers
of the same instrument do not create additional files, and the same instrument
on different channels remains separate. Neither mode performs audio source
separation. A channel can contain overlapping voices from different instruments.

The pipeline is:

**Module → configured tracker playback → aligned floating-point stems → encoding**

The initial release has no AI upscaling, bandwidth extension, or sample replacement.
Traditional interpolation happens inside the tracker renderer, where playback
pitch, sample loops, and effect state are available. Increasing the output sample
rate does not recover information missing from the original samples.

## 1. Fidelity and scope

### 1.1 What faithful means

Faithfulness is relative to a selected playback configuration. A reconstruction
with smooth interpolation and a recreation of historical hardware can sound
different while both being useful. Do not advertise a smoother render as
necessarily more authentic, or claim bit-identical playback across library versions.

The export must preserve:

- Tracker timing, pitch changes, sample offsets, loops, envelopes, and supported
  effect commands.
- Stereo samples, channel panning, and panning automation in stereo output.
- Relative levels, leading silence, and common track origins.
  The tail is the render up to the library's song end, not a drained voice beyond it (section 4.2).
- Visibility of warnings about unsupported playback features, surfaced as
  diagnostics rather than hidden in a library log.

Successful decoding is not proof that every feature plays correctly. In
particular, libopenmpt does not host VST plugins. Report available loader/playback
warnings and describe known limitations; do not claim all unsupported content is
automatically detectable.

### 1.2 Excluded from the initial release

- AI models, GPU discovery, Python environments, engine subprocesses, and model
  downloads or caches.
- Sample enhancement/replacement, persistent module editing, custom libopenmpt
  forks, and reference-conditioned enhancement.
  The vendored libopenmpt is unmodified upstream source at a pinned tag, so it is not a custom fork.
  Sample silencing in a discarded private copy, for pair isolation only, is permitted under the constraints in section 4.5.
- Automatic denoising, excitation, compression, limiting, loudness normalization,
  stereo widening, or channel re-centering.
- Lossy output, merging an instrument across channels, a GUI, and speculative
  extension fields for future enhancement engines.

Quinlight-audio motivated the original exploration, but its sample-mutation and
upscaling pipeline is not part of this architecture.

## 2. Playback configuration and traditional resampling

### 2.1 Rendering profiles

The following profiles express the agreed distinction between compatibility,
hardware character, and clean interpolation. Their CLI names are proposed defaults.

| Profile | Behavior |
|---------|----------|
| `compatible` (default) | Use libopenmpt's recommended playback settings and format compatibility behavior. Record the library version and effective exposed settings; this is the baseline, not a guarantee of matching every historical player. |
| `amiga` | Explicitly enable the stock Amiga resampler and select an A500 or A1200 filter model. Preserve supported module filter commands. Reject modules to which the profile does not apply, and fail if required controls are unavailable. |
| `clean` | Disable Amiga emulation and select libopenmpt's windowed-sinc interpolation. Preserve module commands and recommended volume ramping. This changes sample reconstruction, not musical content, and is an explicit aesthetic choice. |

Libopenmpt also exposes linear and cubic interpolation. Advanced interpolation
controls can be added after the profiles are validated; they are not required for
the initial CLI. Amiga emulation overrides ordinary interpolation, so conflicting
settings must not be silently accepted.

The library does not report whether Amiga emulation applies.
`render.resampler.emulate_amiga` accepts `true` and reads back `true` on a module that never uses the Amiga resampler.
It reads back the resolved setting, not an applicability result.
The resampler applies only when the library's internal Amiga flag is set, and no C API exposes that flag.
The module type does not predict the flag either: two loaders clear it after they load as MOD, and two others set it per song content.
The amiga profile therefore proves applicability by comparison: render the module with emulation on and off, and require the two renders to differ.
An identical result means the profile does not apply, so the export fails explicitly.
The comparison uses the unmuted trace render as its "off" side and adds one more unmuted render with emulation on.
Each CSoundFile seeds its random generator from the process random device, and no C API sets that seed, so two instances of a randomized module render differently.
The comparison therefore makes two plain renders first and requires them to match bit for bit. A mismatch fails the export as nondeterministic instead of reporting a false verdict.
The verdict compares sample bits, not sample values, so two different bit patterns never count as identical.
The profile code also rejects an explicit interpolation setting when Amiga emulation is enabled, because the library accepts both and lets Amiga win.

Every applied setting is read back after the setter runs.
A render parameter that reads back a different value is a `SettingMismatch` error, and an Amiga model that reads back different text is a `SettingTextMismatch` error.
The library accepts a setting and ignores it in some combinations, so a setter that returns without an error does not show that the profile took effect.

The implementation owns profile settings and their defaults. Record the resolved
configuration in each export, including library-owned defaults where they cannot
be resolved further. Do not duplicate configurable numeric values in documentation.

### 2.2 Output sample rate

Render directly at the requested output sample rate using
`openmpt_module_read_interleaved_float_stereo`. There is no single module "base
rate": different samples and notes can have different effective playback rates.

Resolve an omitted rate through `DEFAULT_OUTPUT_RATE`, defined by the implementation.
Validate explicit rates against the renderer and encoder before starting work.
Use the same rate and playback settings for every stem and reference render.

Do not add a low-rate intermediate, preprocess instrument samples, or routinely
upsample an already rendered song. The initial pipeline needs no separate
resampling stage. If later output requirements require rate conversion, use a
conventional band-limited resampler with explicit delay compensation and common
frame accounting across tracks. Oversampling is a possible measured optimization,
not an assumed fidelity improvement or an initial-release requirement.

## 3. Stock libopenmpt integration

Build against a vendored libopenmpt, and link it statically.
A git submodule holds unmodified upstream source at the pinned tag `libopenmpt-0.8.10` (commit `dc4ddcf68`, SVN revision `25840`) under `crates/openmpt-ext-sys/vendor/openmpt`.
The `openmpt-ext-sys` crate compiles that source with `cc` under the C++17 standard.
The bundled decoders (miniz, minimp3, and stb_vorbis) compile in the same build.
The build needs no system library, no dependency-discovery tool, and no network access after the clone.
The `openmpt-ext` crate is the safe wrapper over that build.
Bump the pin with `scripts/bump-libopenmpt.sh`, which also regenerates the bindings.

The safe wrapper owns an `openmpt_module_ext` created from the input bytes and
obtains its borrowed core module with `openmpt_module_ext_get_module`. It destroys
the owning extension exactly once and frees library-returned strings using
`openmpt_free_string`.

Obtain the stock `interactive` interface using
`openmpt_module_ext_get_interface`: supply the interface identifier, a correctly
laid-out function-pointer structure, and its size; check the integer success
result. Fail clearly if channel muting is unavailable. This API does not return
an interface pointer. Check all setter results and surface library errors.

Required capabilities include channel muting, channel volume, floating-point rendering, metadata, subsong selection, repeat and end controls, and profile configuration.
Channel isolation also needs the current order and row, and pair discovery needs the pattern cell readers.
Query optional controls before using them.
A requested profile must either be applied or fail explicitly; it must not silently degrade to another profile.

The optional pair export can also use `interactive.set_instrument_mute_status`.
In modules with instruments, this call addresses instrument slots; otherwise it addresses sample slots.
Section 4.5 defines when this native strategy is acceptable.

Keep each module instance confined to its render worker. Callback state must
outlive the instance, and callbacks must not unwind through the C boundary.

## 4. Channel isolation and alignment

### 4.1 Render strategy

Create a fresh instance for each tracker channel, apply the same resolved playback
configuration, and silence every other channel before the first read.
Render from the beginning using the floating-point API.
Silencing must keep the execution of timing and global effect commands on the silenced channels.

Channel mute does not keep these commands for every format.
When a module plays with ScreamTracker 3 rules, libopenmpt skips all effects on a muted channel, including speed, tempo, pattern jump, pattern break, and pattern loop.
The S3M loader and other loaders that use the S3M playback type select these rules.
Two silencing methods exist in the stock interactive interface:

| Method | Keeps effects on the silenced channel | Limit |
|--------|---------------------------------------|-------|
| `set_channel_mute_status` | Yes, except under ScreamTracker 3 rules | Also mutes NNA voices that the channel owns. |
| `set_channel_volume` set to 0 | Yes | A channel-volume pattern command (IT and MPTM) overrides it. So does a channel volume slide, and the volume down with duration command (FTM). A song restart resets it. |

The ScreamTracker 3 effect gate reads the channel mute flag, not the channel volume, so zero channel volume does not trigger it.
Three channel-volume writes in the effect engine overwrite the value that the silencing set: the channel volume command, the channel volume slide, and the volume down with duration command.

Select the method from the module's playback type.
Use zero channel volume for modules with ScreamTracker 3 rules, and channel mute for the other types.
Fail explicitly for a type that has neither guarantee.

Detect a broken isolation directly, because the format type is only a proxy.
Record the order and row sequence of each render through `get_current_order` and `get_current_row`.
Compare that sequence with the sequence of the unmuted render.
A difference means the silencing changed the song flow, so the export fails for that channel.
The sequence also feeds pair discovery (section 4.5).

The stock API has no per-channel PCM tap before bus mixing. This requires a full
playthrough per channel. Use bounded worker concurrency and stream PCM to temporary
float files so memory does not grow with the complete set of song-length buffers.
Do not assume rendering many channels is always cheap.

Test ownership of overlapping/new-note-action voices and shared effects. Channel
muting alone is not proof that every supported format decomposes perfectly.

### 4.2 Common timeline

Resolve subsong selection and playback policy before launching render workers.
The proposed default is the first subsong; `--subsong` selects another explicitly.
Exporting all subsongs into separate sets can follow later. Do not inherit the
library's unspecified default selection.

The initial export plays the selected subsong without additional whole-song
repetitions, while honoring its internal pattern flow.
Set `play.at_end` to `stop` for every render.
The library default, `fadeout`, adds a 100 ms fade after the end of the song, and `stop` ends the render at the song end with no fade.
The render helper `read_pass` sets this control and holds a duration cap, so no caller can start a read without both.
Do not use `continue`: it replays the module instead of draining the voices.
The library stops the render at the song end even when a voice still rings, and no stock control drains that voice.
The common timeline therefore ends at the song end, and the export must state this limit rather than promise hidden tail audio.
Validation evidence: `docs/design/pre-implementation-validation.md` (section 5).

Preserve leading silence and render each track from the same origin. Determine a
common end from the longest completed render, including the unmuted render, and
zero-pad shorter tracks. Never independently trim leading silence, and never shorten
a render to an approximate duration estimate. A render ends where the library stops
it, so "longest" is the library's song end. Unexpected timing
or duration differences must be diagnosed before padding; padding is not a remedy
for a divergent playthrough.

Apply a configured maximum render duration as a guard against pathological input.
Reaching it fails the export as incomplete rather than reporting a truncated song
as success. `get_duration_seconds` is an estimate for progress reporting, not the
sole stopping condition or a reliable allocation bound.

### 4.3 Stereo and mono

Stereo is the default and preserves the selected renderer's panning and stereo
effects. Optional mono output is the arithmetic mean of the rendered left and
right channels, applied consistently to stems and mixes.

Mono is deliberately lossy: out-of-phase/surround content can cancel and stereo
placement cannot be reconstructed from it. It is not described as pan-neutral.
There is no `--center-mono`; setting channel panning once can be overridden by
later pattern commands and cannot provide that guarantee.

### 4.4 Reconstruction and randomized playback

Independent instances can produce different random instrument volume, pan, and
filter variations. Shared processing and voice allocation also require testing.
Therefore, unnormalized isolated renders are not automatically an exact
decomposition of a separately rendered full song.

`--mix` produces the sum of the final floating-point channel stems using their
common configuration and export gain. When pair export is enabled, also produce a
separate mix of that set in its own directory. Never sum channel stems together
with pair stems: they are alternative decompositions of the same audio. Optional
`--reference-mix` writes the unmuted render, which uses the same playback profile
and timeline, for listening and comparison. Neither mix is normalized independently.
The unmuted render always runs, because it supplies the row trace (section 4.1).
The option only controls whether its audio is written.

For deterministic fixtures, compare the stem sum against the unmuted reference
within a defined numerical tolerance. For randomized content, measure repeat-render
variation and distinguish that variation from isolation errors. Report validation
status as verified for the tested conditions, divergent, or unverified; do not
claim per-file verification unless that comparison actually ran.

Libopenmpt seeds the random generator of each module instance from a process-wide generator, and the stock API has no seed control.
Each isolated render of a randomized module is therefore a different performance.
Instrument volume, pan, cutoff, and resonance swing, and random vibrato waveforms, are the known sources.
Do not promise a seed option.
Detect randomness with two unmuted renders of the song.
For a randomized module, still export the stems, but report that their sum is not comparable with the unmuted render.

### 4.5 Optional per-instrument/per-channel export

Expose a boolean `instrument_channel_stems` setting, disabled by default. The
proposed CLI switches `--instrument-channel-stems` and
`--no-instrument-channel-stems` explicitly enable or disable it and override any
configured value. Disabled means no pair discovery, pair rendering, or pair files.
Enabled adds a separate export set alongside the ordinary channel stems; it does
not replace them or change their isolation settings.

Pair identity uses the original tracker channel and instrument slot, not display
names. For sample-only modules, use the sample slot as the instrument identity and
record that distinction. An instrument mapping notes to multiple samples remains
one instrument; distinct instruments sharing sample data remain distinct.

A voice belongs to the instrument whose sample data it plays.
This rule makes the pair set a linear decomposition of the channel stem: the pair stems of one channel sum to that channel stem.
Older voices can continue under newer instruments, so do not cut a rendered channel waveform at instrument changes.
Preserve all repeated notes and tails of the selected pair across the complete common timeline.

#### Pair discovery

Discover pairs from the played song flow, not from all pattern data.
Use the order and row sequence that the unmuted render records (section 4.1).
Read the instrument column of each played cell through `get_pattern_row_channel_command`.
Track instrument memory per channel for notes that omit the instrument.
An instrument number without a note also counts, because ProTracker sample swaps and instrument resets use it.
Unreachable pattern entries do not create stems.
Keep a discovered pair even if its render is silent, and record that it is silent.
Do not use an amplitude threshold as a substitute for discovery.

The row trace polls the position after each read, so a block that spans two rows hides one of them.
Read the trace render in blocks no longer than `TRACE_BLOCK_FRAMES`.
The implementation derives that constant from the shortest tick that libopenmpt can play at the output rate.

#### Isolation strategies

Each pair render uses a fresh module instance and silences every other channel as section 4.1 specifies.
It then silences every non-target instrument of the target channel.
Two strategies exist, and they attribute voices differently:

| Strategy | Mechanism | Attribution | Known failures |
|----------|-----------|-------------|----------------|
| Sample silencing (preferred) | Load a private copy of the module bytes in which the sample data of non-target instruments is silent. | By sample data, which matches the rule above. | Needs an adapter for each format. Cannot separate two instruments that share a sample on the same channel. Not applicable to OPL (AdLib) instruments. |
| Native instrument mute | `set_instrument_mute_status` on every non-target instrument. | By the instrument at each note event. | Libopenmpt checks the mute flag only when a note starts, and then sets the voice period to 0. A sample swap without a note escapes the check. A portamento note with a muted instrument stops the voice under it, and a later portamento starts from period 0. |

Sample silencing keeps every playback decision unchanged: periods, volumes, envelopes, NNA (new note action) handling, and voice stealing.
The mixer is linear in sample data, so the pair sum matches the channel stem within mixer rounding.
One playback path breaks the assumption: the MED synth envelope reads sample bytes to set a volume factor.
The MED adapter must reject a module that uses synth envelopes.
The completeness check below verifies the assumption for each export and catches any other case.
Validation evidence: `docs/design/pre-implementation-validation.md` (section 2).

Native instrument mute changes playback state, so a native render needs its completeness check to pass.
The mute flag has one audio-path reader: the note-start check that sets the voice period to 0.
Every other reader gates plugin or MIDI output, so a muted instrument still plays a voice that started before the mute.
That is why the risky events are the two that start or move a voice without a muting note event.
That check alone is not sufficient.
A sample swap puts the audio of one instrument into the render of another, and the pair sum still matches the channel stem.
The played cells of the row trace show the two risky events: an instrument number with no note, and an instrument change on a portamento note.
Select the strategy for each channel in this order:

1. Sample silencing, when an adapter supports the format and no sample of the target instrument is shared with another instrument on the same channel.
2. Native instrument mute, when the channel has no risky event and its completeness check passes.
3. Otherwise, fail the requested pair export explicitly. Never substitute a whole-channel stem for a pair stem.

#### Sample silencing adapters

Stock libopenmpt cannot write sample data or pattern data, so an adapter prepares the silenced bytes before the load.
Build the copy for each pair from the unchanged source bytes, and discard it after the render.
This needs no restore step, and no change can leak between jobs.
Never modify the user's input file.

An adapter writes the silent value of each sample encoding, which is not always the zero byte.
Unsigned 8-bit data (common in S3M) is silent at its midpoint value.
Delta-encoded data (XM) is silent when every delta is zero.
A compressed sample (IT compression, MO3) needs a re-encode or a new uncompressed copy, or the adapter rejects the module.
Preserve sample lengths, loops, instrument maps, and all pattern data.
Do not delete triggers or clear instrument cells, because an empty instrument cell reuses the previous instrument.
The adapter also owns the map from instruments to samples, because the stock API does not expose it.

#### Completeness and reuse

The completeness check depends on whether the module is randomized (section 4.4):

- For a deterministic module, the residual of a channel is its channel stem minus the sum of its pair stems. The residual must stay within the named reconstruction tolerance.
- For a randomized module, the residual is not usable. Render the channel with all of its discovered instruments silenced, and require digital silence.

A failed check shows a missed pair or a broken isolation, and the pair export fails for that channel.

A channel with exactly one discovered pair can reuse its channel stem as the pair stem.
Record the reuse in the manifest.
For a randomized module, the reuse also keeps the pair stem consistent with the channel stem.

Pair renders share the channel set's sample rate, origin, end policy, stereo/mono
choice, and common gain. Determine final frame counts and encoding headroom across
both sets. Bound discovery and render work; a large channel–instrument combination
count must not launch an unbounded number of workers or buffers.
One export runs one unmuted render, one render per channel, and one render per pair, less the reused pairs.
A randomized module adds one repeat render and one silence render for each channel with pairs.
Show this count before the render work starts, so that the cost is visible.

Record the strategy of each pair, its residual, and any silence check in the manifest.

## 5. Output and failure behavior

### 5.1 Containers and levels

Proposed default: floating-point WAV, preserving finite samples above nominal full
scale without hard clipping. Offer integer FLAC as an alternative. Keep PCM in
floating point through rendering, downmixing, summation, and any explicit gain;
never obtain intermediate audio through the integer reader.

No output is normalized automatically. For FLAC, preflight all requested outputs
against the selected integer encoding range. If they exceed it, fail with a clear
diagnostic or let the user explicitly choose a common attenuation. Apply that
same gain to every stem and requested mix, record it, and never clip silently or
normalize individual tracks. Float files can retain headroom, but playback through
a fixed-range device still requires an appropriate listening level.

Choose quantization and dither policy explicitly at final integer encoding. Apply
no dither during intermediate processing or float export. Encoded stem sums and
mixes are compared within the encoding/quantization tolerance, not promised to be
bit-identical. Bit depth, dither settings, and numerical tolerances belong in the
implementation's named configuration and are recorded in export metadata.

### 5.2 Files and provenance

Write an indexed file for each tracker channel, including silent channels, using a
sanitized channel name when present. Keep the channel index authoritative and
preserve the original name in metadata. Names must not introduce path separators,
traversal, platform-invalid filenames, or collisions.

When enabled, write pair stems under `instrument-channels/`, with filenames
containing both stable indices and sanitized optional names, for example
`ch_<channel>_ins_<instrument>_<name>.wav` (or the selected format's extension).
Use a sample identity marker for sample-only modules. Ordinary channel stems and
their mix remain in the main export directory; the optional pair-set mix stays
inside `instrument-channels/`. Label these sets clearly as alternatives for DAW
import so users do not accidentally double the audio by summing both.

An export manifest records:

- Input content hash, module metadata, subsong, and channel-to-file mapping.
- Whether pair export is enabled, each pair's original channel and instrument or
  sample identity, original names, isolation strategy, and export-set membership.
- Tool and libopenmpt versions, selected profile, and resolved playback settings.
- Output rate, channel layout, frame count, end policy, and common gain.
- Encoding/quantization settings, warnings, and any reconstruction measurements.

Write into a staging directory, validate every expected file and its metadata,
then publish the completed export directory on the same filesystem. Refuse an
existing destination in the initial release. Cancellation or failure must not
publish a partial set as complete; report any retained staging files explicitly.

## 6. Components and CLI

| Component | Responsibility |
|-----------|----------------|
| `openmpt-ext` crate | Safe stock-API ownership, capability checks, configuration, metadata, channel mute and channel volume, instrument mute, order/row trace, pattern cell reads, and float reads. |
| `profile.rs` | Resolve rendering profiles and validate supported combinations. |
| `stems.rs` | Bounded channel and pair render orchestration, subsong/end policy, timeline alignment, downmixing, and separate set summation. |
| `pairs.rs` | Pair identities, discovery from the row trace, strategy selection, and completeness checks. |
| `silence.rs` | Format adapters that write silent sample data into a private copy of the module bytes. |
| `render.rs` | Float WAV/FLAC encoding, level checks, explicit common gain, and final quantization. |
| `manifest.rs` | Provenance, channel/pair mapping, export-set membership, diagnostics, and validation results. |
| `main.rs` | CLI validation, progress, cancellation, staging, and final publication. |

Proposed CLI surface: input module, output directory, `--profile`, an Amiga model
selector, `--sample-rate`, `--subsong`, `--mono`/`--stereo`, `--format`, `--mix`,
`--reference-mix`, `--instrument-channel-stems`/`--no-instrument-channel-stems`,
and advanced resource/gain controls. Resolve defaults once and
share the resulting configuration across every render. There is no `--upscale`
or engine selection. Exact option spelling and supported encoding choices can be
settled during implementation without changing the fidelity contract.

## 7. Acceptance criteria

Build a small fixture corpus before expanding output options. Cover:

- Representative MOD, XM, IT, and S3M playback, with format-specific quirks.
- Pitch slides, sample offsets, sample loops, panning changes, stereo samples,
  envelopes, and new-note-action voices.
- Timing/global commands on muted channels and supported shared effects.
- Speed, tempo, pattern jump, and pattern loop commands on a silenced channel of an S3M module and of a module from another loader with S3M playback rules. The row trace of each channel render must match the unmuted trace.
- A channel-volume command (IT `Mxx`) on a silenced channel, which must not undo the silencing.
- Randomized instruments, subsongs, pattern loops, end-of-song behavior, and tails. The end policy test asserts that a rendered song ends without the 100 ms fade.
- A module whose last note rings past the song end, which must end at the library's song end and record that limit.
- A MED module with a synth envelope, which must fail pair export where the adapter rejects it.
- Leading silence, silent channels, mono phase cancellation, and float headroom.
- Pair export on/off behavior, including explicit disabling of an enabled setting
  and the absence of pair-specific work when disabled.
- A channel triggering distinct instruments, repeated triggers of the same
  instrument, and one instrument reused across different channels.
- Sample-only modules, multisample instruments, shared sample data, instrument
  memory, unreachable patterns, silent used pairs, and instrument-only commands.
- Overlapping tails across instrument changes, portamento, note cuts, retriggers,
  and pair sums compared with the corresponding ordinary channel stems.
- A ProTracker sample swap (an instrument number with no note), and an instrument change on a portamento note, under each isolation strategy.
- Rows faster than one trace block would allow, so that the trace block limit is tested.
- Sample silencing for each supported encoding: signed, unsigned, delta, 16-bit, stereo, and compressed samples. The silenced copy must play with the same row trace as the original, and the input file must not change.
- A shared sample on one channel, an OPL instrument, and an unsupported format, each of which must select the native strategy or fail explicitly.
- A deterministic module with a deliberately missed pair, which the residual check must catch. A randomized module, where the silence check replaces the residual check.
- Unsupported controls, library warnings, malformed input, cancellation, output
  collisions, and encoding failures.

Verify frame alignment and metadata after decoding the exported files. Compare
summed stems with an unmuted render for deterministic fixtures; also compare
against trusted tracker renders or hardware captures where available, so using
the same library on both sides is not the only fidelity check. Record the exact
profile used for every reference.

Use numerical checks for timing, reconstruction residuals, clipping, and
quantization. Use listening comparisons for playback character and audible tail
behavior. A smoother sound or a higher output sample rate is not itself a pass.

## 8. Decisions to validate before implementation is finalized

The product direction is settled: faithful stems, stock libopenmpt, explicit
playback character, and conventional resampling.

All seven technical questions are resolved.
The evidence and the decisions are in `docs/design/pre-implementation-validation.md`.
Their effect on this document is applied above.

- Reconstruction envelope: the random state feeds instrument swing and random-vibrato waveforms. Each instance seeds from a process-wide generator that has no C API control. Randomized modules stay supported, with a comparison status instead of a claim of exact isolation.
- Sample values: no playback decision reads sample values, except the MED synth envelope. Sample silencing therefore keeps playback state, and the MED adapter rejects synth envelopes.
- Native instrument mute: the mute flag has one audio-path reader, on note start. The strategy stays the second choice, with its known risky events.
- ScreamTracker 3 coverage: the effect gate reads the channel mute flag, not the channel volume. Zero channel volume therefore keeps effects for every loader with S3M playback rules. Three channel-volume effects can still overwrite the silencing, and the sample-isolation completeness check detects that.
- End policy: use `stop`. The default `fadeout` adds a 100 ms fade. Maximum-duration termination stays a separate failure.
- Profiles: the library does not report Amiga applicability, and the module type does not predict it. The amiga profile compares an emulated render with a plain one. The profile code rejects a conflicting interpolation setting.
- Defaults: the output rate decision is in the validation record, and the value lives in implementation configuration. The other settings keep the library defaults, and the profiles change what each profile names.

Resource limits, encoding choices, and comparison tolerances stay open. They are named implementation configuration, and no validation question blocks them.

Sample silencing is restricted to pair isolation and does not introduce sample enhancement or AI processing.
This document defines the design; it does not start implementation.

## References

- [libopenmpt C API](https://lib.openmpt.org/doc/group__libopenmpt__c.html): float
  rendering, subsongs, end controls, and Amiga resampler configuration.
- [Render parameters](https://lib.openmpt.org/doc/group__openmpt__module__render__param.html):
  interpolation, gain, stereo separation, and volume ramping.
- [Extension C API](https://lib.openmpt.org/doc/group__libopenmpt__ext__c.html):
  extension ownership and interface retrieval.
- [Interactive isolation](https://lib.openmpt.org/doc/structopenmpt__module__ext__interface__interactive.html):
  channel muting and instrument muting, including sample-only module semantics.
- [Interactive panning](https://lib.openmpt.org/doc/structopenmpt__module__ext__interface__interactive2.html):
  panning overrides and their limits.
- [libopenmpt FAQ](https://lib.openmpt.org/libopenmpt/faq/): compatibility and plugin
  support limitations.
- OpenMPT source, read at commit `dc4ddcf68`:
  - `libopenmpt/libopenmpt_ext_impl.cpp`: channel mute (also NNA voices), channel volume, and instrument or sample mute.
  - `soundlib/Snd_fx.cpp`, `CSoundFile::NoteChange`: the instrument and sample mute check at note start, which sets the period to 0.
  - `soundlib/Snd_fx.cpp`, `kST3NoMutedChannels`: muted channels skip all effects under ScreamTracker 3 rules.
  - `soundlib/Sndfile.cpp`: each instance seeds its random generator from the process-wide generator.
