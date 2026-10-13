$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
$root = Split-Path $PSScriptRoot -Parent
$toolsDir = Join-Path $root '.tools'
New-Item -ItemType Directory -Force $toolsDir | Out-Null
$archive = Join-Path $toolsDir 'onnxruntime.zip'
if (!(Test-Path $archive)) { Invoke-WebRequest 'https://github.com/microsoft/onnxruntime/releases/download/v1.24.2/onnxruntime-win-x64-1.24.2.zip' -OutFile $archive }
if ((Get-FileHash $archive -Algorithm SHA256).Hash.ToLowerInvariant() -ne '8e3e9c826375352e29cb2614fe44f3d7a4b0ff7b8028ad7a456af9d949a7e8b0') { throw 'ONNX Runtime archive checksum mismatch.' }
Expand-Archive $archive -DestinationPath (Join-Path $toolsDir 'onnxruntime') -Force
New-Item -ItemType Directory -Force (Join-Path $root 'licenses') | Out-Null
Invoke-WebRequest 'https://creativecommons.org/licenses/by/4.0/legalcode.txt' -OutFile (Join-Path $root 'licenses/CC-BY-4.0.txt')
