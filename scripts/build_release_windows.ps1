# Stop on any error
$ErrorActionPreference = "Stop"

Write-Host "cd into opennote-ui..."
Set-Location ./crates/opennote-desktop

Write-Host "Building desktop binary..."
cargo build --release

Set-Location ../..

# Create output directory if needed
$outDir = ".\target\release\windows-package"
New-Item -ItemType Directory -Force -Path $outDir | Out-Null

# Copy the executable
Copy-Item ".\target\release\opennote-desktop.exe" -Destination $outDir\ -Force

Write-Host "Creating ZIP archive..."
$zipPath = ".\target\opennote-windows-x86_64.zip"
Compress-Archive -Path "$outDir\*" -DestinationPath $zipPath -Force

Write-Host "ZIP created at $zipPath"