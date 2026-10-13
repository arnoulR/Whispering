# Developer/release preparation only. This script is never launched by Whispering.
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
$root = Split-Path $PSScriptRoot -Parent
$dest = Join-Path $root 'models/parakeet-v3'
New-Item -ItemType Directory -Force $dest | Out-Null
$base = 'https://huggingface.co/istupakov/parakeet-tdt-0.6b-v3-onnx/resolve/8f23f0c03c8761650bdb5b40aaf3e40d2c15f1ce'
$files = @{
    'encoder-model.int8.onnx' = '6139d2fa7e1b086097b277c7149725edbab89cc7c7ae64b23c741be4055aff09'
    'decoder_joint-model.int8.onnx' = 'eea7483ee3d1a30375daedc8ed83e3960c91b098812127a0d99d1c8977667a70'
    'nemo128.onnx' = 'a9fde1486ebfcc08f328d75ad4610c67835fea58c73ba57e3209a6f6cf019e9f'
    'vocab.txt' = ''
}
foreach ($name in @($files.Keys)) {
    $path = Join-Path $dest $name
    if (!(Test-Path $path)) {
        Write-Host "Downloading $name"
        Invoke-WebRequest "$base/$name" -OutFile "$path.partial"
        Move-Item -LiteralPath "$path.partial" -Destination $path -Force
    }
    $actual = (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($files[$name] -and $actual -ne $files[$name]) { throw "Model checksum mismatch: $name" }
    if ($name -eq 'vocab.txt') {
        # Git blob identity published by the pinned Hugging Face revision.
        $bytes = [IO.File]::ReadAllBytes($path)
        $header = [Text.Encoding]::UTF8.GetBytes("blob $($bytes.Length)`0")
        $sha = [Security.Cryptography.SHA1]::Create()
        $gitHash = ([BitConverter]::ToString($sha.ComputeHash($header + $bytes))).Replace('-','').ToLowerInvariant()
        if ($gitHash -ne 'fc43e1c723e262df60b70e1919614417162d1fe2') { throw 'Vocabulary checksum mismatch' }
    }
    $files[$name] = $actual
}
$files | ConvertTo-Json | Set-Content (Join-Path $root 'assets/model-hashes.json') -Encoding utf8
Write-Host 'Parakeet v3 is ready to bundle.'
