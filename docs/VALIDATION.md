# Validation

Validated locally on Windows x64, 13 October 2026.

## Passed

- Release build with the checked-in Cargo.lock.
- `cargo clippy --locked --all-targets -- -D warnings`.
- Nine core tests: interrupted job recovery; five-file retention; expiry deletion; preserving audio when history is corrupt; literal dictionary replacement; WAV round-trip; resampling duration/amplitude; reading a flushed WAV before finalization; chunking without discarded samples.
- The additional Windows integration test synthesized speech with an installed Microsoft SAPI voice into a WAV file, without speaker playback. This test requires normal Windows speech registry access and passed outside the execution sandbox. It is marked ignored for CI machines without voices.
- Actual Parakeet v3 inference from the packaged executable on the transcribe-rs `jfk.wav` and `german.wav` reference samples. Outputs are saved under `docs/validation`. Both ran with the execution sandbox's network restriction.
- Native window startup and global-shortcut registration; app-owned screenshots of Dictation, History, Dictionary, Getting started, and Settings in an isolated preview profile. The redesigned interface was visually checked in light and dark themes, with additional Dictation, Dictionary, and Settings checks at 880 × 650. Screenshots in `docs/validation/redesign` use example recordings and dictionary entries.
- Model files match pinned source hashes. ONNX Runtime archive matches Microsoft's release digest, and the DLL hash is embedded in the app.
- Runtime Rust dependency tree inspected: no reqwest, ureq, async-openai, tokio, or native-tls runtime dependencies. Network-related build dependencies are used only in dependency preparation. ONNX telemetry is explicitly disabled in code.
- Release bundles model, runtime DLLs, Microsoft VC runtime DLLs, setup instructions, model attribution, and dependency license notices.

## Practical limits of these checks

Live microphone dictation, physically pressing the global shortcut, and automatic insertion into the user's preferred applications still need a user acceptance pass. The audio file inference and local speech synthesis paths have been exercised; those are not substitutes for testing each microphone, driver, and destination application.

This is an unsigned development release, not a signed installer or an independently audited security product. No packet-capture audit or broad hardware compatibility testing was performed. Local files are protected by the Windows user profile's normal permissions and any Windows disk encryption; Whispering does not add encryption at rest.

## Reproduce

```powershell
./scripts/Build.ps1 -Test
cargo test --locked microsoft_voice_synthesizes -- --include-ignored
cargo clippy --locked --all-targets -- -D warnings
./scripts/Build.ps1 -Release
./scripts/Package.ps1
```

The diagnostic command accepts a local audio path and writes the result to the specified local text file:

```powershell
./dist/Whispering/Whispering.exe --transcribe-test C:/audio/example.wav C:/audio/result.txt
```

It uses the same fixed bundled model and integrity checks as the application. `--preview <directory> dashboard|guide|settings|history|dictionary dark|light [compact]` uses a separate test data directory, captures the app's own rendered viewport, then exits.

## Floating indicator update — 14 October 2026

- Added a native always-on-top indicator, audio-driven level history, saved-audio retry, and English/German indicator labels. Recording/progress windows are click-through and open without requesting focus.
- Automatic typing defaults to enabled for new profiles; existing settings are preserved. Retry retains the destination and requests focus back after clicking Retry; insertion still checks the foreground window.
- Release build, all nine non-ignored tests, and clippy with warnings denied passed. The packaged executable hash matches the release executable.
- The isolated hidden indicator preview timed out without a screenshot. Visual layout, microphone response, focus behavior, retry interaction, and insertion into destination applications remain manual acceptance checks.
- Diagnostic preview modes also accept `overlay-recording`, `overlay-failed`, and `overlay-transcribing`; an optional seventh argument `de` selects German (`--preview <directory> overlay-failed dark normal de`). Preview waveforms are synthetic and do not access the microphone.
