# Threat model

Whispering processes microphone input, imported audio and transcripts on the local Windows machine. Protected assets include audio, transcripts, settings, model/runtime integrity and release provenance.

Trust boundaries are the release download and installation, model and native-library loading, imported media parsing, the local profile directory, and intentional output into another application or exported file. A compromised Windows account or operating system is outside the protection offered by the application.

Repository controls pin dependencies and downloaded assets, restrict release packaging to selected inputs, and verify package manifests and the approved signing identity for releases. These controls do not establish that dependencies or the application are vulnerability-free. A SHA-256 manifest detects changes relative to that manifest; publisher authentication requires a trusted signature.

Profiles contain sensitive unencrypted local data. Retention uses ordinary deletion and is not secure erasure. Another application or synced export destination can transmit user-directed output. Imported media and native inference libraries remain attack surfaces requiring patching and review.

Security audits, assessment reports, test recordings and host-specific evidence are intentionally excluded from Git. This design note is not an audit or a claim that outstanding findings are resolved.
