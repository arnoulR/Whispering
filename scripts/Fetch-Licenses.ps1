# Release preparation: recover upstream license texts omitted from some crate archives.
$ErrorActionPreference='Stop'
$ProgressPreference='SilentlyContinue'
$root=Split-Path $PSScriptRoot -Parent
Set-Location $root
if (Test-Path '.tools/cargo/bin/cargo.exe') {
    $env:RUSTUP_HOME=Join-Path $root '.tools/rustup';$env:CARGO_HOME=Join-Path $root '.tools/cargo';$env:PATH=(Join-Path $root '.tools/cargo/bin')+';'+$env:PATH
}
$metadata=cargo metadata --locked --format-version 1 --filter-platform x86_64-pc-windows-msvc | ConvertFrom-Json
if ($LASTEXITCODE -ne 0) {throw 'Cannot read dependency metadata'}
$cache=@{}
foreach ($package in $metadata.packages) {
    if ($package.name -eq 'whispering') {continue}
    $dir=Split-Path $package.manifest_path -Parent
    $existing=Get-ChildItem -LiteralPath $dir -File | Where-Object Name -match '^(LICENSE|LICENCE|COPYING|NOTICE)'
    if ($existing -or $package.name -eq 'epaint_default_fonts') {continue}
    if ($package.repository -notmatch '^https://github.com/') {throw "Review license source for $($package.name)"}
    $repo=$package.repository -replace '^https://github.com/','' -replace '\.git$',''
    $repo=($repo.Split('/')[0..1] -join '/')
    if ($repo -notmatch '^[^/]+/[^/]+$') {throw "Review license source for $($package.name): $repo"}
    $vcs=Get-Content (Join-Path $dir '.cargo_vcs_info.json') -Raw | ConvertFrom-Json
    $commit=$vcs.git.sha1
    $key="$repo/$commit"
    if (!$cache.ContainsKey($key)) {
        $tree=Invoke-RestMethod "https://api.github.com/repos/$repo/git/trees/$commit"
        $licensePaths=@($tree.tree | Where-Object { $_.type -eq 'blob' -and $_.path -match '(?i)license|licence|copying|copyright|notice' } | Select-Object -ExpandProperty path)
        if (!$licensePaths) {throw "No root license files at $key"}
        $cache[$key]=@{}
        foreach ($licensePath in $licensePaths) {
            $cache[$key][$licensePath]=(Invoke-WebRequest "https://raw.githubusercontent.com/$repo/$commit/$licensePath").Content
        }
    }
    $out=Join-Path $root "licenses/$($package.name)-$($package.version)"
    New-Item -ItemType Directory -Force $out | Out-Null
    foreach ($name in $cache[$key].Keys) { [IO.File]::WriteAllText((Join-Path $out $name),$cache[$key][$name]) }
    Write-Host "License: $($package.name)"
}
