# ⚡ NexusTUI

<div align="center">

![Go](https://img.shields.io/badge/Go-1.24-00ADD8?style=for-the-badge&logo=go&logoColor=white)
![Rust](https://img.shields.io/badge/Rust-2021-DEA584?style=for-the-badge&logo=rust&logoColor=black)
![Ratatui](https://img.shields.io/badge/Ratatui-0.29-2b3137?style=for-the-badge&logo=terminal&logoColor=green)
![PostgreSQL](https://img.shields.io/badge/PostgreSQL-16-4169E1?style=for-the-badge&logo=postgresql&logoColor=white)
![Docker](https://img.shields.io/badge/Docker-Compose-2496ED?style=for-the-badge&logo=docker&logoColor=white)
![Linux](https://img.shields.io/badge/Linux-CachyOS%20%7C%20Wayland-1793D1?style=for-the-badge&logo=archlinux&logoColor=white)

**Sıfır GUI ayak izine sahip, terminal tabanlı, ultra düşük gecikmeli yerel ağ (LAN) cihaz yönetim ve senkronizasyon merkezi.**  
*KDE Connect'in Tiling Window Manager (Hyprland / Sway) ve CachyOS / Arch Linux kullanıcıları için tasarlanmış yüksek performanslı terminal alternatifi.*

</div>

---

## 🎯 Neden NexusTUI?

Geleneksel araçlar (KDE Connect, GSConnect vb.):
* 150-200 MB RAM tüketir, arkada ağır Qt/C++ GUI servisleri çalıştırır.
* Telefona 50 MB'lık üçüncü parti uygulamalar kurdurur ve pil tüketir.
* Gerçek 60 FPS ekran yansıtma yapamaz.

**NexusTUI Felsefesi:**
* **Telefona Sıfır APK:** Telefona özel hiçbir uygulama kurulmaz; Android'in yerel ADB Wi-Fi ve dahili `nc` araçlarıyla çalışır.
* **15 MB Toplam Bellek:** Go santrali (~8 MB) + Rust TUI (~6 MB) ile minimum sistem kaynağı.
* **Tek Komutla Başlatma:** Sadece Rust istemcisini çalıştırmanız yeterlidir; Go sunucusunu arka planda kendisi uyandırır ve yönetir.
* **Kalıcı Hafıza:** Geçici bellekte kaybolan soket verileri yerine tüm cihaz ve mesaj trafiği **PostgreSQL** üzerinde saklanır.

---

## 🏗️ Mimari Şema

```text
               ┌──────────────────────────────────────────────┐
               │         POSTGRESQL (Docker Container)        │
               │  - devices & messages (ACID, pgx/v5)         │
               └──────────────────────▲───────────────────────┘
                                      │
                         pgxpool.Pool │ (127.0.0.1:5432)
                                      ▼
┌─────────────────────────────────────────────────────────────┐
│                 GO MERKEZ SUNUCUSU (:9000)                  │
│  - Raw TCP Soket Santrali (Database, Hub, Handler)          │
│  - Non-blocking Broadcast & Asenkron DB Persistansı         │
└──────────────▲──────────────────────▲─────────────────────▲─┘
               │                      │                     │
      TCP      │             TCP      │            TCP      │
┌──────────────▼──────┐ ┌─────────────▼───────┐ ┌───────────▼───────────┐
│     RUST TUI        │ │ XIAOMI / ANDROID    │ │     TABLET / 2. PC    │
│ (PC Terminal Paneli)│ │ (192.168.1.50)      │ │ (nc / Raw Socket)     │
│ - Ratatui Gösterge  │ └─────────────────────┘ └───────────────────────┘
│ - [↑/↓] Cihaz Seç   │            │
│ - [F2] 60FPS Stream │◄───────────┘
│ - [F3] Hızlı Gönder │  Kablosuz ADB (:5555) H.265 60 FPS Video Akışı
│ - [F4] Telefondan Al│
└─────────────────────┘
```

---

## ✨ Öne Çıkan Özellikler

* **🚀 60 FPS H.265 Kablosuz Ekran Yansıtma (`F2`):** `scrcpy 4.1` motoruyla telefonun GPU donanım encoder'ı tetiklenir, 30ms jitter buffer ile takılmasız Full HD yayın monitöre gelir.
* **📁 Açılır Pencereli Hızlı Dosya Gönderici (`F3`):** Terminalde elle dosya yolu yazmak yerine `F3` tuşuna basıp `Downloads`, `Pictures`, `Documents` klasörlerinden numaraya basarak dosyayı telefona fırlatın.
* **📥 Telefondan Dosya Çekme (`F4`):** Telefondaki en güncel dosyaları listeleyip tek tuşla PC `~/Downloads/` klasörüne indirin.
* **🖼️ Otomatik Android Galeri İndeksleme:** Gönderilen resimler telefonda `/sdcard/Download/NexusTUI/` klasörüne yazılır ve anında `MEDIA_SCANNER_SCAN_FILE` ile tetiklenerek Galeri uygulamasında **NexusTUI** albümünde belirir.
* **🔔 Android Sistem Bildirimleri:** PC'den gönderilen mesajlar telefonda sesli ve titreşimli resmi Android bildirimi olarak patlar.
* **🚨 İzole Hata Paneli:** Alt süreçlerin (scrcpy, adb) çıktıları terminal ekranını bozmaz; sağ alttaki özel teşhis logunda toplanır.

---

## ⌨️ Klavye Kısayolları

| Tuş | Fonksiyon |
| :--- | :--- |
| **`↑` / `↓`** | Bağlı cihazlar listesinde gezinme |
| **`F2`** | Seçili cihazın ekranını kablosuz olarak monitöre yansıt (`scrcpy`) |
| **`F3`** | Hızlı Dosya Gönderici modal menüsünü aç (`[1-4]` klasör seç, `[1-9]` dosya gönder) |
| **`F4`** | Telefondan dosya çekme modal menüsünü aç (`[1-8]` ile PC'ye indir) |
| **`PgUp` / `PgDn`** | Canlı akış ve pano geçmişini yukarı/aşağı kaydır |
| **`Enter`** | Mesaj gönder (veya `/mirror`, `/send <yol>`, `/pull <dosya>`) |
| **`Esc`** | Açık menüyü kapat veya NexusTUI'den temiz çıkış yap |

---

## 🚀 Hızlı Başlangıç

### 1. Depoyu Klonlayın
```bash
git clone https://github.com/SenatorHerrscher/NexusTUI.git
cd NexusTUI
```

### 2. Backend Katmanını Başlatın (Docker Compose)
```bash
docker compose up -d
```

### 3. Rust TUI İstemcisini Çalıştırın
```bash
cd tui
cargo run --release
```

---

## 📄 Lisans
MIT License © 2026 Arda Serbest (SenatorHerrscher)
