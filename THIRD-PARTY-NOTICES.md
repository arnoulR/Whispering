# Third-party notices

## Model

NVIDIA **Parakeet TDT 0.6B v3**, https://huggingface.co/nvidia/parakeet-tdt-0.6b-v3

Copyright NVIDIA. Licensed under Creative Commons Attribution 4.0 International:
https://creativecommons.org/licenses/by/4.0/ . License text is bundled under `licenses/CC-BY-4.0.txt` in release packages.

The model was exported to ONNX and quantized to INT8 by **Ivan Stupakov**, https://huggingface.co/istupakov/parakeet-tdt-0.6b-v3-onnx . Whispering bundles that conversion at revision `8f23f0c03c8761650bdb5b40aaf3e40d2c15f1ce`, including its preprocessing model and vocabulary. Whispering does not retrain or fine-tune the weights. Model filenames and SHA-256 hashes are in `assets/model-hashes.json`. No endorsement by NVIDIA or the conversion author is implied.

## Inference and UI

- transcribe-rs, copyright its contributors, MIT: https://github.com/cjpais/transcribe-rs
- Microsoft ONNX Runtime 1.24.2, MIT: https://github.com/microsoft/onnxruntime
- egui/eframe, copyright its contributors, MIT or Apache-2.0: https://github.com/emilk/egui
- All Rust dependency license texts and package metadata are collected into the release `licenses` directory by `scripts/Build-Package.ps1`.
- Symphonia is licensed under MPL 2.0. The corresponding unmodified source for its included crates is supplied in `licenses/source/`; these crates are also available by version from https://crates.io and https://github.com/pdeljanov/Symphonia .
- Microsoft Visual C++ runtime redistributable DLLs are included from the installed Visual Studio redistribution directory under Microsoft's redistribution terms. They remain Microsoft software.

## Product reference

TypeWhisper for Windows (https://github.com/TypeWhisper/typewhisper-win) was consulted as a feature reference. Its source is GPLv3/commercially licensed. No source implementation, icons, or branding were copied into Whispering. The Whispering logo and UI are original.
## Font and icons

- Roboto, from https://github.com/google/fonts/tree/main/ofl/roboto, is embedded in the app. Licensed under SIL Open Font License 1.1; see `licenses/Roboto-OFL.txt`.
- Lucide icons 0.468.0, from https://github.com/lucide-icons/lucide/tree/0.468.0/icons, are embedded SVG assets. Their stroke color is set to white for theme tinting. See `licenses/Lucide-LICENSE.txt` for the ISC license and inherited Feather MIT notices.
