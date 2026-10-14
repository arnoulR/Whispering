param([Parameter(Mandatory)][string]$BundleDirectory)
$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
$bundle = (Resolve-Path -LiteralPath $BundleDirectory).Path
& (Join-Path $PSScriptRoot 'Verify-Package.ps1') -Path $bundle
$toml = Get-Content (Join-Path $root 'Cargo.toml') -Raw
if ($toml -notmatch '(?m)^version = "(\d+\.\d+\.\d+)"') { throw 'Cannot read application version' }
$version = $Matches[1]
$wix = Join-Path $root '.tools/wix/wix.exe'
if (!(Test-Path -LiteralPath $wix)) { throw 'Install WiX: dotnet tool install wix --version 5.0.2 --tool-path .tools/wix' }
& $wix build (Join-Path $root 'installer/Package.wxs') -arch x64 -d "Version=$version" -d "BundleDirectory=$bundle" -o (Join-Path $root 'dist/Whispering-windows-x64.msi')
if ($LASTEXITCODE -ne 0) { throw 'WiX build failed' }
