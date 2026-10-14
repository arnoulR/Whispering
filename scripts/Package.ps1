# Compatibility entry point.
param([switch]$SkipZip, [string]$VcRuntimeDirectory, [switch]$RequireSignature, [string]$SignerThumbprint)
& (Join-Path $PSScriptRoot 'Build-Package.ps1') @PSBoundParameters
