# Whispering

A small native Windows app written in Rust, inspired by TypeWhisper's dictation workflow. Local speech recognition is fixed to **NVIDIA Parakeet TDT 0.6B v3**, using an INT8 ONNX conversion on your CPU. The complete release includes the model. No Python, account, GPU, or model download is required to use it.

## Start here

From this project folder, double-click **Start Whispering.cmd**. You can also open `dist\Whispering\Whispering.exe` directly. Windows may hide the `.exe` extension and display it as **Whispering**, with type **Application**.

1. Extract the **entire** `Whispering` release folder to a local drive.
2. Double-click `Whispering.exe`. Keep the model and DLL files beside it.
3. Follow the short in-app guide. Select your microphone in Settings if needed.
4. Press **Ctrl+Shift+Space**, speak, then press it again. Place the cursor in your destination field first. The transcript appears there and in the app.
5. A floating indicator shows **Recording** with a live microphone waveform. If transcription fails, choose **Retry** on the indicator or open **History** to retry or export audio.

Keep Whispering open or minimized for its global shortcut to work. Closing the window quits the app. There is no tray process or startup service. If Windows blocks microphone access, enable desktop microphone access in Windows Settings → Privacy & security → Microphone.

**Type into the active app** is enabled by default for new profiles. Existing profiles keep their choice; change it in Settings. This sends keyboard events without touching the clipboard, and skips insertion if another window is focused. Leave it off to keep transcripts inside Whispering. You can always explicitly save text or audio.

## Included

- Configurable global start/stop shortcut and microphone selection.
- Draggable, always-on-top recording indicator with an elapsed timer, audio-driven waveform, fixed-size transcription progress, and retry. English/German indicator language is selectable in Settings.
- Local file transcription: WAV, MP3, FLAC, OGG/Vorbis, and supported AAC/M4A files, up to 60 minutes.
- Automatic language recognition using Parakeet v3's 25 supported European languages.
- Local read-aloud with an installed Microsoft Windows SAPI voice. Parakeet itself is speech-to-text only.
- Latest five recordings, including failures, with retry, WAV export, text export, and delete.
- Recovery after app interruption: audio is written during recording, with its WAV header refreshed on each buffer.
- Simple dictionary corrections and statistics for the retained recordings.
- Native interface with light, dark, and Windows system themes, a centered AI sparkle logo, and a short setup guide.

## Recovery and privacy

Recordings and transcripts are stored in `%LOCALAPPDATA%\Whispering`. Settings and history use atomic file replacement. Another instance cannot open the same data folder while the first is running.

The default keeps **the latest five recordings for up to 24 hours**, including failures. The sixth replaces the oldest. Settings offer 1 hour, 24 hours, or 7 days. Cleanup runs while idle and on the next launch; nothing runs while the app is closed. Export audio you need to retain. A crash or power loss can lose the last buffer; a full disk or hardware failure can prevent recording. A damaged history index is reported without deleting audio files.

There are no cloud providers, accounts, telemetry calls, update checks, plugin loaders, local HTTP servers, or runtime downloads in the app. ONNX telemetry is explicitly disabled. The model and ONNX DLL are SHA-256 checked before inference. The UI has no model selector or model import.

Export and optional automatic typing intentionally send content to a destination you choose. A cloud-synced destination folder or another app can transmit that content independently. Standard text selection/copy is also a user-directed clipboard action. Local history is not encrypted by Whispering; use Windows device encryption/BitLocker for data at rest. Expiry uses ordinary deletion, not forensic secure erasure. The application does not provide an OS security boundary against malware or a compromised Windows installation.

## Requirements and limits

Windows 10/11 **x64**, a working microphone, and an OpenGL-capable graphics driver for the native UI. Allow approximately 750 MB for the app bundle and additional space for recovery recordings. 8 GB of RAM is recommended; actual performance depends on your CPU. No discrete GPU is required. ARM and older Windows versions have not been validated.

Recordings and imported audio are limited to **60 minutes**. At the recording limit, Whispering automatically stops, saves the audio, and starts transcription. Long recordings are split near quieter boundaries to bound memory usage; words at boundaries can occasionally be imperfect. Microsoft voices already installed in Windows are used for read-aloud. There is no voice download facility. Automatic typing can be blocked by elevated applications and may not work in every control.

## Build from source

Install the Rust MSVC toolchain and Visual Studio C++ Build Tools with a Windows SDK. Then:

```powershell
./scripts/Fetch-Model.ps1       # developer-only internet download; pinned model and hashes
./scripts/Fetch-Runtime.ps1     # developer-only internet download; verified Microsoft runtime
./scripts/Fetch-Licenses.ps1    # license texts omitted from some crate archives
./scripts/Build.ps1 -Test
./scripts/Build.ps1 -Release
./scripts/Package.ps1
```

`dist/Whispering` is the complete portable app. `dist/Whispering-windows-x64.zip` is the distributable archive. `START-HERE.txt` is included. Build scripts access the internet only to prepare dependencies and assets; they are not run by the application. Cargo.lock fixes Rust dependencies. There is no custom model path or alternate inference provider.

For a local toolchain in this workspace, `Build.ps1` automatically uses `.tools/cargo` and `.tools/rustup`. These are development files, not part of the release. To regenerate the original icon, run `scripts/Make-Icon.ps1`.

## Attribution

Whispering is an independent Rust implementation inspired by [TypeWhisper for Windows](https://github.com/TypeWhisper/typewhisper-win). No TypeWhisper implementation code or branding is included. [NVIDIA Parakeet v3](https://huggingface.co/nvidia/parakeet-tdt-0.6b-v3) is CC BY 4.0; the bundled quantized [ONNX conversion by Ivan Stupakov](https://huggingface.co/istupakov/parakeet-tdt-0.6b-v3-onnx) is attributed in `THIRD-PARTY-NOTICES.md`. Rust inference uses [transcribe-rs](https://github.com/cjpais/transcribe-rs), with only the local ONNX feature enabled. See the included dependency notices and licenses.
