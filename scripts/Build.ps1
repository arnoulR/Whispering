param([switch]$Release, [switch]$Test)
$ErrorActionPreference = 'Stop'
Set-Location (Split-Path $PSScriptRoot -Parent)
if (Test-Path '.tools/cargo/bin/cargo.exe') {
    $env:RUSTUP_HOME = Join-Path (Get-Location) '.tools/rustup'
    $env:CARGO_HOME = Join-Path (Get-Location) '.tools/cargo'
    $env:PATH = (Join-Path (Get-Location) '.tools/cargo/bin') + ';' + $env:PATH
}
if ($Test) { cargo test --locked } elseif ($Release) { cargo build --release --locked } else { cargo build --locked }
if ($LASTEXITCODE -ne 0) { throw 'Rust build or tests failed.' }
