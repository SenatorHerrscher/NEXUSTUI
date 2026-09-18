# ⚡ NexusTUI

<div align="center">

![Go](https://img.shields.io/badge/Go-1.24-00ADD8?style=for-the-badge&logo=go&logoColor=white)
![Rust](https://img.shields.io/badge/Rust-2021-DEA584?style=for-the-badge&logo=rust&logoColor=black)
![Ratatui](https://img.shields.io/badge/Ratatui-0.29-2b3137?style=for-the-badge&logo=terminal&logoColor=green)
![PostgreSQL](https://img.shields.io/badge/PostgreSQL-16-4169E1?style=for-the-badge&logo=postgresql&logoColor=white)
![Docker](https://img.shields.io/badge/Docker-Compose-2496ED?style=for-the-badge&logo=docker&logoColor=white)
![Linux](https://img.shields.io/badge/Linux-CachyOS%20%7C%20Wayland-1793D1?style=for-the-badge&logo=archlinux&logoColor=white)

**Zero-GUI footprint, terminal-native, ultra-low latency local network (LAN) device sync & wireless command hub.**  
*A lightweight, high-performance terminal alternative to KDE Connect designed for Tiling Window Managers (Hyprland / Sway) and Arch / CachyOS users.*

</div>

---

## 🎯 Why NexusTUI?

Traditional cross-device suites (KDE Connect, GSConnect, etc.):
* Consume 150–200 MB of RAM with heavy Qt/C++ background daemons.
* Force installing bloated 50 MB mobile apps that drain battery in the background.
* Lack hardware-accelerated 60 FPS screen mirroring and direct audio forwarding.

**The NexusTUI Philosophy:**
* **Zero Mobile APK:** No third-party apps required on your phone; leverages native Android wireless ADB and built-in socket tools.
* **< 15 MB Total Memory Footprint:** Go socket hub (~8 MB) + Rust TUI dashboard (~6 MB) for maximum efficiency.
* **Battery Saver Wireless Mirroring:** Physical phone screen remains completely **OFF** during mirroring (`--turn-screen-off`), streaming directly to your monitor while routing all phone audio straight to your PC speakers/headphones.
* **Single-Command Startup:** Launching the Rust TUI automatically detects and starts the Go server backend in the background if it is not already running.
* **ACID Persistence:** Socket history and device registries persist into **PostgreSQL** via high-throughput connection pools (`pgxpool`).

---

## 🏗️ Architecture

```text
               ┌──────────────────────────────────────────────┐
               │         POSTGRESQL (Docker Container)        │
               │  - devices & messages (ACID, pgx/v5)         │
               └──────────────────────▲───────────────────────┘
                                      │
                         pgxpool.Pool │ (127.0.0.1:5432)
                                      ▼
┌─────────────────────────────────────────────────────────────┐
│                 GO CENTRAL SERVER (:9000)                   │
│  - Raw TCP Socket Hub (Database, Hub, Handler)              │
│  - Non-blocking Broadcast & Async Database Persistence      │
└──────────────▲──────────────────────▲─────────────────────▲─┘
               │                      │                     │
      TCP      │             TCP      │            TCP      │
┌──────────────▼──────┐ ┌─────────────▼───────┐ ┌───────────▼───────────┐
│     RUST TUI        │ │ XIAOMI / ANDROID    │ │   TABLET / 2ND PC     │
│ (PC Terminal Panel) │ │ (192.168.1.50)      │ │ (nc / Raw Socket)     │
│ - Ratatui Dashboard │ └─────────────────────┘ └───────────────────────┘
│ - [↑/↓] Select Dev  │            │
│ - [F2] 60FPS Mirror │◄───────────┘
│ - [F3] Quick Send   │  Wireless ADB (:5555) H.265 60 FPS Video Stream
│ - [F4] Pull to PC   │  + Screen-Off Battery Saver + Direct PC Audio
└─────────────────────┘
```

---

## ✨ Features

* **🚀 60 FPS H.265 Mirroring + Screen-Off & PC Audio (`F2`):** Powered by `scrcpy 4.1` with hardware H.265 encoding. Automatically powers **off** the physical phone display to conserve battery while keeping the device active and streaming crystal-clear audio directly to PC speakers.
* **📁 Modal Quick File Sender (`F3`):** No manual file paths required. Press `F3` to pop up an interactive picker for `Downloads`, `Pictures`, `Documents`, and `Desktop` — press `[1-9]` to beam the file immediately to the phone.
* **📥 Pull Files from Phone (`F4`):** Displays recent files in the phone's download directory and pulls them directly into `~/Downloads/` on your PC with a single keystroke.
* **🖼️ Instant Android MediaStore Indexing:** Transferred photos and videos are stored in `/sdcard/Download/NexusTUI/` and instantly broadcast to Android's `MEDIA_SCANNER_SCAN_FILE` intent, showing up immediately in the phone Gallery under the **NexusTUI** album.
* **🔔 Native Android Push Notifications:** Messages and file alerts trigger audible and tactile system push notifications on Android (`cmd notification post`).
* **🚨 Isolated Diagnostics Window:** Subprocess outputs (`adb`, `scrcpy`) are piped directly to an isolated red system log panel, preventing terminal buffer flicker.

---

## ⌨️ Keybindings

| Key | Action |
| :--- | :--- |
| **`↑` / `↓`** | Navigate connected devices list |
| **`F2`** | Wireless mirror with phone screen OFF and PC audio routed |
| **`F3`** | Open Quick File Sender modal (`[1-4]` folder, `[1-9]` send file) |
| **`F4`** | Open Pull File modal (`[1-8]` download file to PC) |
| **`PgUp` / `PgDn`** | Scroll live messages and clipboard history |
| **`Enter`** | Send chat message (or execute `/mirror`, `/send <path>`, `/pull <file>`) |
| **`Esc`** | Close open modal popup or cleanly exit NexusTUI |

---

## 🚀 Quick Start

### 1. Clone Repository
```bash
git clone https://github.com/SenatorHerrscher/NexusTUI.git
cd NexusTUI
```

### 2. Start Database Backend
```bash
docker compose up -d
```

### 3. Launch NexusTUI
```bash
# Launch directly from repository root
cargo run --release
```
*The Rust client will automatically verify and start the Go server in the background if it is not already running.*

---

## 📄 License
MIT License © 2026 Arda Serbest (SenatorHerrscher)
