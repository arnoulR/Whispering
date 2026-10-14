param([Parameter(Mandatory)][string]$Path, [switch]$RequireSignature, [string]$SignerThumbprint)
$ErrorActionPreference = 'Stop'
$base = (Resolve-Path -LiteralPath $Path).Path
$expected = @{}
foreach ($line in Get-Content (Join-Path $base 'SHA256SUMS.txt')) {
    if ($line -notmatch '^([a-f0-9]{64})  (.+)$') { throw "Invalid manifest line: $line" }
    $hash = $Matches[1]; $relative = $Matches[2]
    if ([IO.Path]::IsPathRooted($relative) -or $relative -match '(^|[\\/])\.\.([\\/]|$)' -or $relative.Contains(':')) { throw 'Unsafe manifest path' }
    if ($expected.ContainsKey($relative)) { throw "Duplicate manifest entry: $relative" }
    $expected[$relative] = $hash
    if ((Get-FileHash -LiteralPath (Join-Path $base $relative) -Algorithm SHA256).Hash.ToLowerInvariant() -ne $hash) { throw "Checksum mismatch: $relative" }
}
foreach ($file in Get-ChildItem -LiteralPath $base -Recurse -Force) {
    if ($file.Attributes -band [IO.FileAttributes]::ReparsePoint) { throw "Reparse point in package: $($file.Name)" }
    if ($file.PSIsContainer) { continue }
    $relative = $file.FullName.Substring($base.Length + 1).Replace('\','/')
    if ($relative -ne 'SHA256SUMS.txt' -and !$expected.ContainsKey($relative)) { throw "Unlisted package file: $relative" }
}
if (!$expected.ContainsKey('Whispering.exe')) { throw 'Application missing from manifest' }
if ($RequireSignature -or $SignerThumbprint) {
    if (!$SignerThumbprint) { throw 'Provide the approved signer thumbprint when requiring a signature.' }
    $signature = Get-AuthenticodeSignature -LiteralPath (Join-Path $base 'Whispering.exe')
    if ($signature.Status -ne 'Valid' -or $signature.SignerCertificate.Thumbprint -ne $SignerThumbprint) { throw 'Application signature is not valid for the approved publisher.' }
}
Write-Host "Package verified: $base"
