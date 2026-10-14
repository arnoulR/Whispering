# Administration and releases

The x64 MSI installs per machine under Program Files and requires elevation. Users run the app without elevation. Preserve inherited Program Files ACLs: ordinary users must not be able to replace the executable, models, or DLLs. Profiles remain under each user's `%LOCALAPPDATA%\Whispering`; MSI removal does not erase user recordings or settings. There is no service or automatic startup task.

## Intune deployment

Wrap the signed MSI with Microsoft's Win32 Content Prep Tool and deploy as a system-context Win32 app. Install with `msiexec /i Whispering-windows-x64.msi /qn /norestart`. Use the MSI ProductCode from the built installer for detection and `msiexec /x {ProductCode} /qn /norestart` for removal. Validate install, upgrade, removal, microphone permissions and standard-user launch in a disposable Windows VM before rollout. The ProductCode changes on major upgrades; the UpgradeCode in installer/Package.wxs stays stable.

The app needs no network access. An administrator may add an outbound block rule for the installed executable:

```powershell
New-NetFirewallRule -DisplayName 'Whispering offline' -Direction Outbound -Program "$env:ProgramFiles\Whispering\Whispering.exe" -Action Block
```

Settings are per user; there is no enforced enterprise policy interface. New profiles enable automatic typing. Configure microphone, typing and retention in the app. Do not deploy real user profiles as templates.

## Build and sign

Use the pinned Rust toolchain, Visual Studio C++ Build Tools and Windows SDK. Run Prepare-Assets.ps1, Build.ps1 -Test, and Build.ps1 -Release. Install WiX with `dotnet tool install wix --version 5.0.2 --tool-path .tools/wix`.

Sign `target/release/whispering.exe` before packaging. Run Build-Package.ps1 with `-RequireSignature -SignerThumbprint <approved-thumbprint>`; its output is the new bundle directory. Pass that directory to Build-Installer.ps1 with `-BundleDirectory`. Sign the resulting MSI and verify its Authenticode status and publisher before distribution. ZIPs contain the already signed executable.

VC runtime 14.40.33807 files must match assets.lock.json. Build-Package.ps1 accepts `-VcRuntimeDirectory` or locates that exact Visual Studio redistribution version. These are licensed local build inputs, not downloaded by Prepare-Assets.ps1. Changing runtime versions requires reviewing and updating the lock.

The manual release workflow requires a dedicated trusted Windows runner labelled `whispering-release`, a protected `release` environment, its `SIGNER_THUMBPRINT` variable, and a runner environment variable `WHISPERING_SIGN_SCRIPT` pointing to a reviewed signing hook accepting `-Path`. Keep the hook outside the checkout. Configure signing with OIDC federation or environment-scoped credentials; never store keys in Git. The workflow fails closed without signing and creates a draft GitHub Release. This repository does not provision a signing account or runner.

Enable branch protection with required CI and code-owner review. CODEOWNERS alone does not require approval. Keep audit reports and host evidence internal; only sanitized design documentation belongs under docs/security.
