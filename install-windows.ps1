# cli5 Windows Installer
# Usage: .\install-windows.ps1

Write-Host "=== cli5 Windows Installer ===" -ForegroundColor Cyan
Write-Host ""

# Check if running as Administrator
$isAdmin = ([Security.Principal.WindowsPrincipal] [Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole] "Administrator")

if (-not $isAdmin) {
    Write-Host "[!] Warning: Not running as Administrator" -ForegroundColor Yellow
    Write-Host "    Some steps may require elevation" -ForegroundColor Yellow
    Write-Host ""
}

# Determine architecture
$arch = if ([Environment]::Is64BitOperatingSystem) { "x86_64" } else { "i686" }
Write-Host "[*] Detected architecture: $arch" -ForegroundColor Cyan

# Download URL
$version = "latest"  # or specific version like "v0.5.0"
$url = "https://github.com/pmazurki/cli5/releases/$version/download/cli5-windows-x86_64.exe"
$dest = "$env:TEMP\cli5.exe"

Write-Host "[*] Downloading cli5..." -ForegroundColor Cyan
try {
    Invoke-WebRequest -Uri $url -OutFile $dest -UseBasicParsing -ErrorAction Stop
    Write-Host "[+] Download complete" -ForegroundColor Green
} catch {
    Write-Host "[-] Download failed: $($_.Exception.Message)" -ForegroundColor Red
    Write-Host ""
    Write-Host "Alternative: Download manually from:" -ForegroundColor Yellow
    Write-Host "  https://github.com/pmazurki/cli5/releases" -ForegroundColor Yellow
    exit 1
}

# Install location
$installDir = "$env:LOCALAPPDATA\cli5"
$installPath = "$installDir\cli5.exe"

Write-Host "[*] Installing to: $installPath" -ForegroundColor Cyan

# Create directory
if (-not (Test-Path $installDir)) {
    New-Item -ItemType Directory -Path $installDir -Force | Out-Null
}

# Copy binary
Copy-Item -Path $dest -Destination $installPath -Force
Remove-Item -Path $dest -Force

Write-Host "[+] Installation complete" -ForegroundColor Green
Write-Host ""

# Add to PATH
$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($userPath -notlike "*$installDir*") {
    Write-Host "[*] Adding to PATH..." -ForegroundColor Cyan
    [Environment]::SetEnvironmentVariable("Path", "$userPath;$installDir", "User")
    Write-Host "[+] Added to PATH (restart terminal to use)" -ForegroundColor Green
} else {
    Write-Host "[=] Already in PATH" -ForegroundColor DarkYellow
}

Write-Host ""
Write-Host "=== Installation Complete ===" -ForegroundColor Green
Write-Host ""
Write-Host "Usage:" -ForegroundColor Cyan
Write-Host "  cli5 tunnel start support.example.com --port 22" -ForegroundColor Yellow
Write-Host ""
Write-Host "Note: Restart PowerShell/CMD to use 'cli5' command" -ForegroundColor Yellow
Write-Host "      Or use full path: $installPath" -ForegroundColor Yellow
Write-Host ""

