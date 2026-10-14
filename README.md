# Whispering

Local dictation for Windows, built in Rust. Whispering turns speech into text using NVIDIA Parakeet TDT 0.6B v3 on your CPU. Releases include the model: no account, Python, GPU, or runtime download is needed.

## Install

Get the Windows x64 package from [GitHub Releases](https://github.com/arnoulR/Whispering/releases).

- **Installer:** run `Whispering-windows-x64.msi`, then open `C:\Program Files\Whispering\Whispering.exe`. Installation requires administrator permission; running the app does not.
- **Portable:** extract the entire ZIP and open `Whispering.exe`. Keep the bundled models and DLLs beside it.

Windows 10/11 x64, a microphone, and an OpenGL-capable graphics driver are required. Allow approximately 750 MB for the app plus recording storage; 8 GB of RAM is recommended. Performance depends on your CPU. Windows ARM has not been validated.

## Use

1. Open Whispering and select your microphone in Settings if needed.
2. Place the cursor in the destination text field.
3. Press **Ctrl+Shift+Space**, speak, then press it again to transcribe. The shortcut is configurable.
4. The transcript appears in Whispering and is typed into the destination app.

**Type into the active app** is enabled for new profiles; existing profiles keep their setting. Disable it in Settings to keep transcripts inside Whispering. Automatic insertion is skipped if another window is focused and can be blocked by elevated applications or unsupported controls.

Keep Whispering open or minimized for the global shortcut to work. Closing the window quits the app; there is no tray process or startup service.

## Features

- Local speech recognition with automatic language recognition across Parakeet v3's 25 supported European languages.
- A draggable recording indicator with timer, microphone waveform, progress, and retry; English and German indicator labels.
- File transcription for WAV, MP3, FLAC, OGG/Vorbis, and supported AAC/M4A files. Recording and import are limited to 60 minutes; recording stops and transcribes automatically at the limit.
- History with retry, copy, text export, WAV export, and delete; dictionary corrections and local dictation statistics.
- Read-aloud using an installed Windows voice, plus light, dark, and system themes.

## Privacy and storage

Speech recognition runs locally. The app has no accounts, cloud services, telemetry, update checks, or runtime downloads. The bundled model and ONNX runtime are checked against SHA-256 hashes before inference.

Settings, audio, transcripts, and statistics live in `%LOCALAPPDATA%\Whispering`. History keeps the **latest ten recordings for up to 24 hours** by default, including failures; the eleventh replaces the oldest. Settings offer 1 hour, 24 hours, or 7 days. Cleanup runs while the app is open or on its next launch. Export recordings you need to keep. Uninstalling does not erase this user data.

Statistics count completed microphone dictations independently of history. Retrying, editing, deleting, or expiring recordings does not add another count or erase statistics; imports and failures are excluded. Statistics retain recording identifiers, dates, word counts, durations, and available destination executable names, without audio, transcripts, full paths, or window titles. They remain until `statistics.json` is removed with the app closed.

Local data is not encrypted by Whispering, and expiry uses ordinary deletion. Copying, exporting, and automatic typing send content to destinations you choose; other apps or cloud-synced folders may transmit it independently.

## Troubleshooting

- **Microphone blocked:** enable desktop microphone access in Windows Settings → Privacy & security → Microphone.
- **Transcription failed:** use Retry on the indicator or in History, or export the audio. Restore the complete release package if model or runtime files are missing or fail integrity checks.
- **Missing recent audio after interruption:** recording audio is saved during capture, but a crash can lose the last buffer and a full disk can prevent saving.
- **Long recordings:** audio is split near quieter boundaries to limit memory usage; words at those boundaries can occasionally be imperfect.

Report reproducible bugs through [GitHub Issues](https://github.com/arnoulR/Whispering/issues). Include your app version, Windows version, and steps to reproduce; keep private recordings and transcripts out of reports.

## Development and credits

See [building and releases](https://github.com/arnoulR/Whispering/blob/main/docs/build.md) for source builds and packaging. Changes are described in [GitHub Releases](https://github.com/arnoulR/Whispering/releases).

Whispering is an independent implementation inspired by [TypeWhisper for Windows](https://github.com/TypeWhisper/typewhisper-win). It uses NVIDIA Parakeet v3 with Ivan Stupakov's INT8 ONNX conversion and transcribe-rs for inference. See [third-party notices](THIRD-PARTY-NOTICES.md) for attribution and licenses. Whispering's code is [MIT licensed](LICENSE).
