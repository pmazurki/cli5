# cli5 na Windows

## 📥 Instalacja

### Metoda 1: Automatyczna (PowerShell)

```powershell
# Pobierz i uruchom installer
Invoke-WebRequest -Uri "https://raw.githubusercontent.com/pmazurki/cli5/main/install-windows.ps1" -OutFile install-windows.ps1
.\install-windows.ps1
```

### Metoda 2: Ręczna

1. Pobierz `cli5-windows-x86_64.exe` z [Releases](https://github.com/pmazurki/cli5/releases)
2. Zmień nazwę na `cli5.exe`
3. Umieść w folderze (np. `C:\tools\cli5\`)
4. Dodaj do PATH:
   ```powershell
   [Environment]::SetEnvironmentVariable("Path", "$env:Path;C:\tools\cli5", "User")
   ```

### Metoda 3: Chocolatey (jeśli masz)

```powershell
choco install cli5
```

---

## 🚀 Użycie

### Admin Mode (z CF_API_TOKEN)

```powershell
# Ustaw zmienne środowiskowe
$env:CF_API_TOKEN = "your-token-here"

# Utwórz tunel + DNS
cli5.exe tunnel start support.example.com --port 22 --background

# Output:
# ✅ Tunnel configured: support.example.com
# 🔐 Token for users: TUNNEL_TOKEN='eyJhIjoiM...'
```

### User Mode (tylko TUNNEL_TOKEN)

```powershell
# Ustaw token tunelu
$env:TUNNEL_TOKEN = "eyJhIjoiM..."

# Uruchom tunel
cli5.exe tunnel start --port 22 --background

# Output:
# 🔑 User mode: Running tunnel with token
# 🟢 Tunnel started
```

### Zatrzymaj tunel

```powershell
cli5.exe tunnel stop
```

### Status

```powershell
cli5.exe tunnel status
```

---

## 🔧 Użycie w skryptach PowerShell

```powershell
# Przykład: Uruchomienie tunelu w skrypcie
$env:TUNNEL_TOKEN = "eyJhIjoiM..."  # Token od admina

# Uruchom tunel w tle
Start-Process -FilePath "cli5.exe" -ArgumentList "tunnel", "start", "--port", "22", "--background" -WindowStyle Hidden

# Lub bezpośrednio:
cli5.exe tunnel start --port 22 --background
```

---

## 📋 Przykłady

### Tworzenie tunelu (Admin)

```powershell
# 1. Ustaw API token
$env:CF_API_TOKEN = "your-cloudflare-api-token"

# 2. Utwórz tunel
cli5.exe tunnel start support.example.com --port 22 --background

# 3. Skopiuj token dla usera
# TUNNEL_TOKEN='eyJhIjoiM...'
```

### Uruchomienie tunelu (User)

```powershell
# 1. Ustaw token tunelu
$env:TUNNEL_TOKEN = "eyJhIjoiM..."

# 2. Uruchom
cli5.exe tunnel start --port 22 --background

# 3. Zatrzymaj
cli5.exe tunnel stop
```

---

## ⚠️ Troubleshooting

### "cli5: command not found"

```powershell
# Sprawdź PATH
$env:Path -split ';' | Select-String "cli5"

# Dodaj ręcznie
$env:Path += ";C:\tools\cli5"
```

### "cloudflared not found"

```powershell
# cli5 automatycznie pobierze cloudflared
# Lub zainstaluj ręcznie:
cli5.exe tunnel install-client
```

### Uruchomienie w tle

```powershell
# Użyj --background
cli5.exe tunnel start --port 22 --background

# Lub Start-Process
Start-Process -FilePath "cli5.exe" -ArgumentList "tunnel", "start", "--port", "22", "--background" -WindowStyle Hidden
```

---

## 📝 Notatki

- **cli5.exe** działa natywnie na Windows (nie wymaga Rust)
- **cloudflared** jest automatycznie pobierany przy pierwszym użyciu
- **Token tunelu** = wszystko co user potrzebuje (bez CF API access)
- **Admin** tworzy tunele, **User** tylko je uruchamia

