param([switch]$SkipZip, [string]$VcRuntimeDirectory, [switch]$RequireSignature, [string]$SignerThumbprint)
$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
Set-Location $root
$dest = Join-Path $root ('dist/staging-' + [guid]::NewGuid().ToString('N') + '/Whispering')
$lock = Get-Content (Join-Path $root 'assets.lock.json') -Raw | ConvertFrom-Json
& (Join-Path $PSScriptRoot 'Prepare-Assets.ps1') -Offline
function Copy-BundleDll($source) {
    $target = Join-Path $dest $source.Name
    # An open app locks its DLLs; identical bundled files need no replacement.
    if ((Test-Path -LiteralPath $target) -and
        (Get-FileHash -LiteralPath $source.FullName).Hash -eq (Get-FileHash -LiteralPath $target).Hash) { return }
    Copy-Item -LiteralPath $source.FullName -Destination $target -Force
}
New-Item -ItemType Directory -Force $dest,(Join-Path $dest 'models'),(Join-Path $dest 'licenses/source'),(Join-Path $dest 'assets') | Out-Null
if (!(Test-Path 'target/release/whispering.exe')) { throw 'Build the release first.' }
foreach ($name in @('encoder-model.int8.onnx','decoder_joint-model.int8.onnx','nemo128.onnx','vocab.txt')) {
    $hashes = Get-Content 'assets/model-hashes.json' -Raw | ConvertFrom-Json
    if ((Get-FileHash "models/parakeet-v3/$name" -Algorithm SHA256).Hash.ToLowerInvariant() -ne $hashes.$name) { throw "Model integrity error: $name" }
}
Copy-Item 'target/release/whispering.exe' (Join-Path $dest 'Whispering.exe') -Force
New-Item -ItemType Directory -Force (Join-Path $dest 'models/parakeet-v3') | Out-Null
foreach ($name in $hashes.PSObject.Properties.Name) { Copy-Item -LiteralPath "models/parakeet-v3/$name" -Destination (Join-Path $dest 'models/parakeet-v3') }
$runtime = '.tools/onnxruntime/onnxruntime-win-x64-1.24.2'
if ((Get-FileHash "$runtime/lib/onnxruntime.dll" -Algorithm SHA256).Hash.ToLowerInvariant() -ne '114947d633e6844ce3c4b51ef6678f776628571d08a5763859c61642c8dcca9c') { throw 'Runtime integrity error' }
foreach ($name in @('onnxruntime.dll','onnxruntime_providers_shared.dll')) { Copy-BundleDll (Get-Item "$runtime/lib/$name") }
Copy-Item "$runtime/LICENSE","$runtime/ThirdPartyNotices.txt" (Join-Path $dest 'licenses') -Force
Copy-Item 'licenses/Roboto-OFL.txt','licenses/Lucide-LICENSE.txt' (Join-Path $dest 'licenses') -Force
Copy-Item 'licenses/CC-BY-4.0.txt' (Join-Path $dest 'licenses') -Force
Copy-Item 'README.md','LICENSE','THIRD-PARTY-NOTICES.md' $dest -Force
Copy-Item 'assets/model-hashes.json' (Join-Path $dest 'assets') -Force
if (!$VcRuntimeDirectory) {
    $crt = Get-ChildItem "C:/Program Files (x86)/Microsoft Visual Studio/2022/*/VC/Redist/MSVC/$($lock.vcRuntime.version)/x64/Microsoft.VC143.CRT" -Directory | Select-Object -First 1
    if (!$crt) { throw 'Provide -VcRuntimeDirectory with the runtime pinned in assets.lock.json.' }
    $VcRuntimeDirectory = $crt.FullName
}
foreach ($file in $lock.vcRuntime.files) {
    $source = Get-Item -LiteralPath (Join-Path $VcRuntimeDirectory $file.name)
    if ((Get-FileHash -LiteralPath $source.FullName -Algorithm SHA256).Hash.ToLowerInvariant() -ne $file.sha256) { throw "VC runtime integrity error: $($file.name)" }
    Copy-BundleDll $source
}
if (Test-Path '.tools/cargo/bin/cargo.exe') {
    $env:RUSTUP_HOME=Join-Path $root '.tools/rustup';$env:CARGO_HOME=Join-Path $root '.tools/cargo';$env:PATH=(Join-Path $root '.tools/cargo/bin')+';'+$env:PATH
}
$metadata = cargo metadata --locked --offline --format-version 1 --filter-platform x86_64-pc-windows-msvc | ConvertFrom-Json
if ($LASTEXITCODE -ne 0) { throw 'Cannot collect dependency notices' }
$notices = @()
foreach ($package in $metadata.packages) {
    if ($package.name -eq 'whispering') { continue }
    $notices += "$($package.name) $($package.version) | $($package.license) | $($package.repository)"
    $packageDir=Split-Path $package.manifest_path -Parent
    $licenseFiles = Get-ChildItem -LiteralPath $packageDir -File | Where-Object { $_.Name -match '^(LICENSE|LICENCE|COPYING|NOTICE)' }
    $licenseDest=Join-Path $dest "licenses/$($package.name)-$($package.version)"
    New-Item -ItemType Directory -Force $licenseDest | Out-Null
    $licenseFiles | Copy-Item -Destination $licenseDest -Force
    if (Test-Path (Join-Path $packageDir 'licenses')) {Copy-Item (Join-Path $packageDir 'licenses') $licenseDest -Recurse -Force}
    if (Test-Path (Join-Path $packageDir 'fonts')) {
        Get-ChildItem (Join-Path $packageDir 'fonts') -File | Where-Object Extension -ne '.ttf' | Copy-Item -Destination $licenseDest -Force
    }
    $extra=Join-Path $root "licenses/$($package.name)-$($package.version)"
    if (Test-Path $extra) {Get-ChildItem -LiteralPath $extra -File | Copy-Item -Destination $licenseDest -Force}
    if (@(Get-ChildItem -LiteralPath $licenseDest -File -Recurse).Count -eq 0) {throw "License text missing for $($package.name). Run Fetch-Licenses.ps1 first."}
    if ($package.license -match 'MPL') {
        $sourceDest=Join-Path $dest "licenses/source/$($package.name)-$($package.version)"
        Copy-Item -LiteralPath $packageDir -Destination $sourceDest -Recurse -Force
    }
}
$notices | Set-Content (Join-Path $dest 'licenses/RUST-DEPENDENCIES.txt')
$sums=Get-ChildItem -LiteralPath $dest -File -Recurse | Where-Object Name -ne 'SHA256SUMS.txt' | ForEach-Object {
    $relative=$_.FullName.Substring($dest.Length+1).Replace('\','/')
    "$((Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant())  $relative"
}
$sums | Set-Content (Join-Path $dest 'SHA256SUMS.txt')
& (Join-Path $PSScriptRoot 'Verify-Package.ps1') -Path $dest -RequireSignature:$RequireSignature -SignerThumbprint $SignerThumbprint
if (!$SkipZip) {
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $zip=Join-Path $root 'dist/Whispering-windows-x64.zip'
    if (Test-Path $zip) { Remove-Item -LiteralPath $zip }
    [IO.Compression.ZipFile]::CreateFromDirectory($dest,$zip,[IO.Compression.CompressionLevel]::Fastest,$true)
}
Write-Host "Ready: $dest"
Write-Output $dest
