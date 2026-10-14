# Using Whispering

Install the MSI, then open `C:\Program Files\Whispering\Whispering.exe`. For a portable release, extract the whole ZIP and open `Whispering.exe` inside it. Keep the bundled models and DLLs together.

1. Choose your microphone in Settings.
2. Place the cursor in the destination text field.
3. Press **Ctrl+Shift+Space**, speak, then press it again.
4. The transcript is typed into the destination and appears in Dictation and History.

Automatic typing is enabled for new profiles. Existing profiles retain their choice. Disable **Type into the active app** in Settings to keep transcripts inside Whispering. Insertion is skipped if focus changes; elevated applications may block it.

Keep the app open or minimized for the shortcut to work. Closing it quits. Recordings and imported audio can be up to 60 minutes long. Read aloud uses an installed Windows voice.

On failure, use History → Retry or Export audio. By default, the latest ten recordings are retained for up to 24 hours, including failures. The eleventh replaces the oldest. Expiry is configurable; cleanup runs while open or on next launch.

Data lives in `%LOCALAPPDATA%\Whispering`. Exports to synced folders may be uploaded by other software. Enable desktop microphone access in Windows privacy settings if recording is blocked. See README.md for privacy and recovery limits.
