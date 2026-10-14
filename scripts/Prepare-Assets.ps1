param([switch]$Offline)
$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
$lock = Get-Content (Join-Path $root 'assets.lock.json') -Raw | ConvertFrom-Json
foreach ($asset in $lock.downloads) {
    $path = Join-Path $root $asset.path
    if (!(Test-Path -LiteralPath $path)) {
        if ($Offline) { throw "Missing asset: $($asset.path)" }
        New-Item -ItemType Directory -Force (Split-Path $path -Parent) | Out-Null
        Invoke-WebRequest -Uri $asset.url -OutFile "$path.partial"
        if ((Get-FileHash -LiteralPath "$path.partial" -Algorithm SHA256).Hash.ToLowerInvariant() -ne $asset.sha256) { throw "Downloaded asset checksum mismatch: $($asset.path)" }
        Move-Item -LiteralPath "$path.partial" -Destination $path -Force
    }
    if ((Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant() -ne $asset.sha256) { throw "Asset checksum mismatch: $($asset.path)" }
}
$models = Get-Content (Join-Path $root 'assets/model-hashes.json') -Raw | ConvertFrom-Json
foreach ($asset in $lock.downloads | Where-Object path -like 'models/*') {
    if ($models.((Split-Path $asset.path -Leaf)) -ne $asset.sha256) { throw 'Embedded model hashes differ from assets.lock.json' }
}
Expand-Archive -LiteralPath (Join-Path $root '.tools/onnxruntime.zip') -DestinationPath (Join-Path $root '.tools/onnxruntime') -Force
Write-Host 'Pinned assets verified and prepared.'
