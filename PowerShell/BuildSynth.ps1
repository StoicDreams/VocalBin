# Builds the speech synthesizer in synth/ as raw WebAssembly and copies it to webapp/root_files/vocal-synth.wasm.
# The page loads that file in an AudioWorklet. Commit it with the Rust change, the same as a .min.js file,
# because the site tests and the deploy use the committed file.
$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
Push-Location $root
try {
    cargo build -p vocal-synth --release --target wasm32-unknown-unknown
    if ($LASTEXITCODE -ne 0) { throw 'cargo build failed' }
    $targetDir = (cargo metadata --format-version 1 --no-deps | ConvertFrom-Json).target_directory
    $wasm = Join-Path $targetDir 'wasm32-unknown-unknown/release/vocal_synth.wasm'
    $out = Join-Path $root 'webapp/root_files/vocal-synth.wasm'
    Copy-Item $wasm $out -Force
    $size = [math]::Round((Get-Item $out).Length / 1KB, 1)
    Write-Host "Built webapp/root_files/vocal-synth.wasm ($size KB)" -ForegroundColor Green
}
finally {
    Pop-Location
}
