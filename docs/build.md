# Building and releases

Run the commands below in PowerShell from the repository root.

## Prerequisites

- Rust with the MSVC toolchain. `rust-toolchain.toml` selects the pinned version and Windows x64 target; `Cargo.lock` pins dependencies.
- Visual Studio C++ Build Tools with a Windows SDK.
- For packaging, Visual C++ runtime **14.40.33807**, with files matching `assets.lock.json`. The packager looks in Visual Studio's redistribution directory; pass `-VcRuntimeDirectory <directory>` if it is elsewhere.
- For MSI creation, a .NET SDK and WiX 5.0.2.

If `.tools/cargo/bin/cargo.exe` exists, the build and packaging scripts automatically use the workspace's `.tools/cargo` and `.tools/rustup` directories. Otherwise they use Cargo from PATH.

## Build and test

```powershell
./scripts/Prepare-Assets.ps1
./scripts/Fetch-Licenses.ps1
./scripts/Build.ps1 -Test
./scripts/Build.ps1 -Release
```

Asset preparation downloads the fixed model and ONNX runtime using URLs and SHA-256 hashes in `assets.lock.json`. License preparation collects upstream license texts missing from crate archives. These are developer operations; the application does not download dependencies.

## Package and run

```powershell
$bundle = ./scripts/Build-Package.ps1
./scripts/Verify-Package.ps1 -Path $bundle
& (Join-Path $bundle 'Whispering.exe')
```

Each build creates a fresh `dist/staging-*/Whispering` bundle and `dist/Whispering-windows-x64.zip`. Use `-SkipZip` to create only the bundle. Launch from the bundle so that the executable can find its model and native DLLs.

To create an MSI from that bundle:

```powershell
dotnet tool install wix --version 5.0.2 --tool-path .tools/wix
./scripts/Build-Installer.ps1 -BundleDirectory $bundle
```

Install WiX once; omit the installation command when it is already present. The MSI output is `dist/Whispering-windows-x64.msi`. It installs per machine under Program Files; uninstalling leaves user data intact. These local commands do not sign the application or installer.

## Signed releases

Sign `target/release/whispering.exe` before packaging, then require the approved certificate when assembling the bundle:

```powershell
$bundle = ./scripts/Build-Package.ps1 -RequireSignature -SignerThumbprint 'APPROVED_CERTIFICATE_THUMBPRINT'
./scripts/Build-Installer.ps1 -BundleDirectory $bundle
```

Sign the resulting MSI and verify its Authenticode status and publisher before distribution. The ZIP contains the executable signed before packaging. Upload distributable packages and release notes to GitHub Releases.

The manual GitHub release workflow requires an existing `v<version>` tag matching `Cargo.toml`, a trusted Windows x64 runner labelled `whispering-release`, and a protected `release` environment. Set its `SIGNER_THUMBPRINT` variable and the runner's `WHISPERING_SIGN_SCRIPT` environment variable, pointing to an external signing hook accepting `-Path`. Use OIDC or environment-scoped signing credentials; keep private keys outside Git. The workflow verifies signatures and creates a draft release.

Generated tools, models, bundles, user data, and local validation reports are ignored. Keep them out of release inputs except for the assets explicitly collected by the packager. To regenerate the app icons, run `./scripts/Make-Icon.ps1`.
