# Pre-implementation validation

This file records the evidence for the seven questions in `docs/design.md` section 8.
The evidence comes from the vendored libopenmpt 0.8.10 source (commit `dc4ddcf68`, SVN revision `25840`) and from temporary probe tests that rendered the vendor fixtures.
The probes are deleted. They were throwaway measurements, not a test suite.

Each section gives the question, the evidence, and the decision.
The decision changes `docs/design.md` where the evidence contradicts it.

## 1. Reconstruction envelope

Question: which playback decisions read random state, and what does that break?

Evidence:

- `soundlib/Sndfile.cpp:121` seeds each `CSoundFile` instance: `m_PRNG(mpt::make_prng<mpt::fast_prng>(mpt::global_prng()))`.
- `common/mptRandom.cpp:65` seeds the process-wide generator from `global_random_device()`.
- The stock C API has no seeding control. `Randomize` exists only in the tracker application, not in libopenmpt.
- The random state feeds instrument swing and random-vibrato waveforms.
  `soundlib/Snd_fx.cpp:2041-2062` uses the generator for volume, pan, cutoff, and resonance swing.
  `soundlib/Sndmix.cpp:847` and `1822` use it for a random vibrato or random panbrello waveform.
- Both files reach the state through `AccessPRNG()` (`soundlib/Sndfile.h:543-544`).
- Probe: two unmuted renders of each vendor module were byte-identical.
  This is not a guarantee. It is one sample from one process seed.

Decision: the design keeps the rule that an isolated render of a randomized module is a different performance.
Two unmuted renders detect the condition.
The export still writes the stems and reports their sum as not comparable.

The vendors carry no module that reads the generator, so a fixture now proves the condition.
`tests/support/it.rs` builds an IT module with one instrument, a volume swing of 64, and four note triggers.
The swing reads `mpt::random<int8>(AccessPRNG())` on every processed tick of the note.
Two instances of the fixture render differently, and `require_deterministic` reports the first differing sample.
`tests/profile_apply.rs` asserts both that report and the `Nondeterministic` error from the full Amiga check.
The builder writes the bytes in the test process, because the vendor tree is a git submodule and a file there would not survive an update.

## 2. Do playback decisions read sample values?

Question: can sample silencing keep playback state exactly?

Evidence: the mixer applies sample data to the output and never branches on it.
The probe renders agree: on a fixture with two overlapping samples, the sum of the per-sample solo renders equals the unmuted render with no sample differing.

One exception exists. `soundlib/InstrumentSynth.cpp:142-152` (`MEDEnvelopeFromSample`) reads sample bytes to build a synth envelope value.
It is used at line 303 for `m_volumeFactor` and line 354 for `m_medVibratoValue`.
Only `Load_med.cpp` emits the synth envelope command, so the exception is limited to MED modules with synth envelopes.
Outside that path, no playback decision reads a sample value.

Decision: sample silencing keeps playback state for every format except MED modules with synth envelopes.
The adapter for MED must reject such a module.
The completeness check catches a residual case, because the pair sum would not match the channel stem.

## 3. How often does native instrument mute fail?

Question: is native instrument mute a useful second strategy?

Evidence from the source, which is stronger than a measurement on four modules:

- `set_instrument_mute_status` sets `INS_MUTE` on the instrument, or `CHN_MUTE` on the sample when the module has no instruments (`libopenmpt/libopenmpt_ext_impl.cpp:220-233`).
- `INS_MUTE` has five readers. Three are inside `#ifndef NO_PLUGINS` and gate VSTi/MIDI output only (`soundlib/Sndmix.cpp:1056`, `1461`, `2671`). One gates plugin selection (`soundlib/Snd_fx.cpp:6732`).
  The only audio-path reader is `soundlib/Snd_fx.cpp:2223`: on note change, it sets `chn.nPeriod = 0`.
- `chn.nPeriod = 0` stops the note. `soundlib/Sndmix.cpp:2213` computes `samplePlaying = (chn.nPeriod && chn.nLength)`, so a zero period takes the channel out of sample playback.
  The mixer never reads `INS_MUTE` to mute the channel. It reads `chn.dwFlags[CHN_MUTE]` at `soundlib/Sndmix.cpp:2183`.
- Therefore instrument mute silences every note that starts after the mute, and leaves the channel otherwise live.
  Under ScreamTracker 3 rules that is strictly better than channel mute, because the channel keeps its effects.
- New-note-action voices are separate channel slots. `soundlib/Snd_fx.cpp:2374-2377` copies the source channel into the NNA slot and resets `CHN_MUTE` on the copy.
  Channel mute handles this explicitly: `set_channel_mute_status` also sets `CHN_MUTE` on every slot whose `nMasterChn` is the channel (`libopenmpt/libopenmpt_ext_impl.cpp:203-210`).
  Instrument mute does not need to. A muted channel is skipped before note processing (`soundlib/Sndmix.cpp:2183`), so it starts no NNA voice; a voice copied from a note that started after the mute inherits that note's `nPeriod`, which is 0.
- The failure modes are the ones the design already lists: a sample swap without a note, and a portamento note.
  The probe could not measure their frequency on the vendor modules, because most instrument slots in those modules carry no audio.
  In `test.mod`, every sample slot except slot 0 is silent, so the fixture exercises one voice only.

Decision: keep native instrument mute as the second strategy with its stated failures.
The finding that changes the design is the scope of the failure: it is confined to note-change events, so the row trace can find the two risky events.
The design already selects the strategy from the row trace, and it keeps that rule.
No measurement on real modules changed the choice, because the vendor fixtures carry too little audio to measure it. The source gives the answer instead.

## 4. Does zero channel volume cover every ScreamTracker 3 playback type?

Question: does the zero-volume method keep effects on a silenced channel for every loader that uses S3M rules?

Evidence:

- The gate that skips effects on a muted channel is `soundlib/Snd_fx.cpp:576-581`: `ignoreMutedChn = m_playBehaviour[kST3NoMutedChannels]` with `ChnSettings[nChn].dwFlags[CHN_MUTE]`.
  It reads the channel setting, not the channel volume. Zero channel volume is not the same flag, so it does not trigger the gate.
- `soundlib/Sndfile.cpp:1164-1300` sets `kST3NoMutedChannels` for `MOD_TYPE_S3M`. `Load_s3m.cpp:429-435` clears it when the module is not a compatible tracker.
  `Load_ftm.cpp:113` sets it unconditionally for FTM, so FTM is the fixture source for a non-S3M loader with S3M playback rules.
- Probe: on `test.mod`, `test.s3m`, and `test.xm`, channel mute and sample mute produced the same order/row sequence as the unmuted render.
  The probe records the sequence after every 256-frame block and finds no difference. The vendor modules do not exercise the ScreamTracker 3 gate, so the source is the authority here.
- `soundlib/Sndmix.cpp:505-527` restarts the song and resets `nGlobalVol` from `ChnSettings[i].nVolume`. That reset applies only at a real song restart.
  The export renders one pass with repeat count 0, so a restart does not occur in the rendered region.
- Three effect commands overwrite `chn.nGlobalVol`: `CMD_CHANNELVOLUME` at `soundlib/Snd_fx.cpp:3768`, `ChannelVolSlide` at `5094-5126`, and `ChannelVolumeDownWithDuration` at `5128-5149`.
  A module that uses any of them on a silenced channel defeats zero volume for that channel.
  FTM produces the third command (`Load_ftm.cpp:404`) and also sets `kST3NoMutedChannels` (`Load_ftm.cpp:113`), so that pair is reachable.

Decision: the design's method table is right, and it now lists all three channel-volume effects in the limit column.
The row trace alone does not detect the override. A channel-volume command does not change the order or row sequence, so the trace stays equal.
The sample-isolation method's completeness check catches it: the silenced channel keeps audio, so its stem is not isolated.
The design keeps the explicit failure for a type with neither guarantee.

## 5. End and tail policy

Question: which stock end control gives a distinct, complete export, with no surprise fade?

Evidence:

- `play.at_end` defaults to `fadeout` (`libopenmpt/libopenmpt_impl.cpp:476`).
- At song end, `soundlib/Sndmix.cpp:255-272` sets `SONG_ENDREACHED` with no fade when `IsRenderingToDisc()` is true.
  That flag is set from `m_ctl_play_at_end != fadeout` (`libopenmpt_impl.cpp:525`).
- Probe: `fadeout` added 4800 extra frames (100 ms at 48 kHz) to both `test.mod` and `test.xm`.
  `stop` and `continue` produced identical output on both fixtures.
- This matters more than the fade. At `test.mod`, `stop` ends the read at 2.040 s while audio is still playing, and `fadeout` at 2.140 s reaches the same point later only because it renders through the fade.
  So `stop` truncates a ringing tail, which the design forbids.
- `continue` does not solve it. It resets `SONG_ENDREACHED` and keeps replaying the module, which measures the same length as `stop` on both fixtures. No tail drain occurs.
- A long repeat count does the same. A one-pattern fixture with `set_repeat_count(8)` postponed the song end by 7.5 s of silence and then repeated the note; it did not drain the voice.
- The render loop stops on `SONG_ENDREACHED` without checking active voices (`soundlib/Sndmix.cpp:229`). The stock API has no dedicated tail-drain control.

Decision: set `play.at_end` to `stop`, so the render never carries the 100 ms fade and never replays the module.
The common timeline ends at the song end, so the export does not promise to keep audio the library stops playing at that point.
`continue` is rejected because it replays the song.
A fixture with a note that rings past the last order still needs a test, to state this limit explicitly rather than to hide it.
Maximum render duration stays a separate failure, as the design states.

## 6. Profile capability detection and Amiga applicability

Question: how does the wrapper know a profile applies, and when is Amiga emulation applicable?

Evidence:

- Two of the profile controls are ctls, and both appear in `get_ctl_infos()`: `render.resampler.emulate_amiga` and `render.resampler.emulate_amiga_type` (`libopenmpt/libopenmpt_impl.cpp:1731-1732`).
  The third is the render parameter `module::RENDER_INTERPOLATIONFILTER_LENGTH`, read at `libopenmpt_impl.cpp:946-947` and written at `971-977`.
  A ctl presence check does not cover it.
  The runtime check is still required for both kinds, because the build pins one version and a future bump could drop a control.
- `render.resampler.emulate_amiga` reads back the resolved setting, not an applicability result: `libopenmpt_impl.cpp:1829-1830` returns `emulateAmiga != Off`.
  A non-Amiga module accepts `true` and reports `true`, with no audible effect.
- The Amiga resampler applies only when `SONG_ISAMIGA` is set and emulation is on (`soundlib/Sndmix.cpp:2571`).
  `SONG_ISAMIGA` is an internal flag with no C API accessor.
- The flag has no single rule. Loaders set or clear it per format, and `Load_mod.cpp:621-648` sets it from note range and magic; `Load_med.cpp:1206` clears it for stereo samples.
- `render.resampler.emulate_amiga_type` values observed: `auto`, `a500`, `a1200`, `unfiltered`.
  Probe: `a1200` and `auto` are byte-identical on `test.mod`, because `auto` resolves to `a1200` for a MOD.
- Conflicting controls: libopenmpt accepts `interpolation_filter_length = 2` while Amiga emulation is on, and Amiga wins.
  The wrapper must reject the combination itself.

Decision: the wrapper cannot ask the library whether Amiga applies.
`metadata("type")` is the file format token, and it does not predict the flag:

- The `dsym` and `xmf` loaders clear the flag after they load as `MOD_TYPE_MOD`, so the token reads `mod` for a module that is not Amiga.
- The `med` loader clears the flag when any instrument uses stereo samples.
- The `gdm` and `mo3` loaders set the flag only for a 4-channel module with Amiga-limit notes, which the token does not express.
- The `puma` loader loads as `MOD_TYPE_MOD` and sets the flag directly (`Load_puma.cpp:185`), so the flag can also be stronger than the token suggests.

So no allowlist of type tokens is correct.
Detect applicability by comparison instead: render the module once with Amiga emulation on and once with it off, and compare the PCM.
The comparison uses the unmuted trace render as its "off" side and adds one unmuted render with emulation on, so the cost is one extra single-track render, not one per stem.
When the two renders are identical, the profile does not apply and the export fails explicitly for that profile.
This never reports a degraded render as an Amiga render.

The design must not claim that the `emulate_amiga` readback is an applicability check.
The profile code rejects a conflicting interpolation setting before it starts a render.

Probe: `test.mod` (type `mod`) differs only after 0.21 s of render, because the opening rows are silent.
A short prefix is therefore not a safe applicability test. Use the full comparison render.

Randomized modules break the comparison unless the code handles them.
`Sndfile.cpp:121` seeds each `CSoundFile` from `mpt::global_prng()`.
`mptRandom.cpp` seeds that generator from `global_random_device()`, and no C API sets the seed.
Two independent instances of one randomized module therefore render differently, and a plain comparison would report `Applies` every time.
The comparison makes two plain renders first and requires them to match bit for bit.
A mismatch is a `Nondeterministic` error, which names the first differing sample index.
The verdict then compares sample bits, so a NaN pair counts as identical and a signed-zero pair counts as different.

A second gap applies to the `clean` profile.
A per-instrument resampling setting wins over the global interpolation mode (`Sndmix.cpp:2565-2580`), and the MT2 drum loader sets the nearest mode on its drums (`Load_mt2.cpp:1062,1107`).
So `clean` cannot deliver windowed-sinc on those instruments.

Decision: `clean` refuses a module that it cannot render as named.
The C API exposes no per-instrument resampling read: `libopenmpt.h` gives only `openmpt_module_get_num_instruments` and `openmpt_module_get_instrument_name`.
The profile therefore cannot name the affected instruments, so the alternative behavior is not available.
`require_clean_applies` fails with `ProfileError::CleanDoesNotApply` when the `type` metadata key names the MT2 format.
The key is the only fact that the API exposes about the per-instrument decision, and `libopenmpt_impl.cpp:1247` returns the module type through it.
Every other module type passes.

The check does not cover every way to reach a per-instrument mode.
`InstrumentExtensions.cpp:544` also sets the field from the extended instrument property `R...`, and an IT or XM file saved by OpenMPT can carry that block.
A module of that kind still fails the profile's promise, and the type key does not reveal it.
The limit is the same one that blocks the better behavior: no per-instrument resampling read exists in the C API.
The MT2 case is the one that a stock module reaches without the extended property, so the check covers it.
The `R...` gap has an open issue (#25), and `src/profile/clean.rs` names it in its module doc.
That issue records the three options and the recommendation, so the limit is tracked work and not a comment.

A render comparison cannot close the `R...` gap, the way it closes the Amiga gap.
Probe: render each vendored fixture at interpolation filter length 8 and at length 1, and compare the samples.
`test.mod` differs at sample 23040 and `test.s3m` differs at sample 2.
`test.xm` and `test.mptm` are bit-identical, and both render silence, so the comparison has no signal to read.
A module that carries `R...` is an IT, XM, or MPTM file, and the two fixtures of those types give no difference.
The comparison would therefore report "no per-instrument mode" for a module that has one.
The check stays a type-token refusal, and the `R...` case stays open until upstream exposes a per-instrument read.

`Load_mt2.cpp` is the only loader that sets a non-default mode from the file structure itself, with no extended property.
It sets the nearest mode on a drum whose sample carries the "no interpolation" flag.

## 7. Defaults to name in configuration

Question: which values does the implementation own, after validation?

Evidence from probe reads on a fresh module:

| Setting | Default observed | Source |
|---------|------------------|--------|
| output rate | none | the C API takes the rate on every read |
| `interpolation_filter_length` | 8 | libopenmpt default |
| volume ramping | -1 | libopenmpt default |
| master gain | 0 | libopenmpt default |
| stereo separation | 100 | libopenmpt default |
| `dither` | 1 | libopenmpt default |

Decision: `DEFAULT_OUTPUT_RATE` is 48000 Hz.
The libopenmpt header recommends it: `libopenmpt/libopenmpt.h:91` says "Please prefer 48000Hz unless the user explicitly demands something else."
The other settings keep the library defaults, and the profiles change only what each profile names.
The value itself lives in implementation configuration, because the design forbids a numeric value in the document.

## Effect on the design document

These sections of `docs/design.md` changed:

1. Section 1.1 and 4.2: the tail is the render up to the library's song end, and the end policy is `stop` with no fade.
2. Section 2.1: the amiga profile proves applicability by comparing an emulated render with a plain one, and the profile code rejects a conflicting interpolation setting.
3. Section 4.1: the zero-volume method lists all three channel-volume effects in its limit column.
4. Section 4.5: the sample-silencing assumption holds except for MED synth envelopes, and the adapter must reject that case.
5. Section 7: the fixture corpus gains the ringing-tail case and the MED synth-envelope case.
6. Section 8: the questions close, and this file is the record.
