mod net;
mod ui;

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::fs;
use std::io::{self, BufRead, BufReader};
use std::net::TcpStream;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use ui::FilePickerState;

fn list_folder_files(folder_path: &str) -> Vec<(String, String)> {
    let mut results = Vec::new();
    if let Ok(entries) = fs::read_dir(folder_path) {
        for entry in entries.flatten() {
            if let Ok(ft) = entry.file_type() {
                if ft.is_file() {
                    let path = entry.path();
                    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
                    if !name.starts_with('.') {
                        let size_desc = if let Ok(meta) = entry.metadata() {
                            let len = meta.len();
                            if len > 1024 * 1024 {
                                format!("{:.1} MB", len as f64 / 1_048_576.0)
                            } else {
                                format!("{} KB", len / 1024)
                            }
                        } else {
                            "".to_string()
                        };
                        let display = if size_desc.is_empty() {
                            name.clone()
                        } else {
                            format!("{} ({})", name, size_desc)
                        };
                        results.push((display, path.to_string_lossy().to_string()));
                    }
                }
            }
            if results.len() >= 9 {
                break;
            }
        }
    }
    results
}

fn list_phone_files() -> Vec<String> {
    let output = Command::new("adb")
        .args([
            "-s", "192.168.1.50:5555",
            "shell",
            "ls -t /sdcard/Download/NexusTUI/ /sdcard/Download/ 2>/dev/null | grep -v ':$' | grep -v '^$' | head -n 8",
        ])
        .output();

    let mut files = Vec::new();
    if let Ok(out) = output {
        let text = String::from_utf8_lossy(&out.stdout);
        for line in text.lines() {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                files.push(trimmed.to_string());
            }
        }
    }
    files
}

fn ensure_go_server_running() {
    if TcpStream::connect("127.0.0.1:9000").is_ok() {
        return;
    }

    let candidates = [
        ("server/nexustui-server", "server"),
        ("../server/nexustui-server", "../server"),
    ];

    for (bin, dir) in &candidates {
        if Path::new(bin).exists() {
            let _ = Command::new(bin)
                .current_dir(dir)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn();
            thread::sleep(Duration::from_millis(600));
            if TcpStream::connect("127.0.0.1:9000").is_ok() {
                return;
            }
        }
    }

    let fallback_dirs = ["server", "../server"];
    for dir in &fallback_dirs {
        if Path::new(dir).exists() {
            let _ = Command::new("go")
                .args(["run", "."])
                .current_dir(dir)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn();
            thread::sleep(Duration::from_millis(1000));
            if TcpStream::connect("127.0.0.1:9000").is_ok() {
                return;
            }
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    ensure_go_server_running();

    let mut net_client = match net::NetworkClient::connect("127.0.0.1:9000") {
        Ok(client) => client,
        Err(e) => {
            eprintln!("❌ Go sunucusuna baglanilamadi: {}", e);
            return Ok(());
        }
    };

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut input = String::new();
    let mut messages: Vec<String> = Vec::new();
    let mut devices: Vec<String> = vec![
        "NexusTUI-PC (127.0.0.1)".to_string(),
        "Xiaomi Mi 11 Lite (192.168.1.50)".to_string(),
    ];
    let mut errors: Vec<String> = vec![
        "ℹ️  Sistem hazir. Dosyalar /Download/NexusTUI/ klasorune aktarilir.".to_string(),
    ];
    let mut selected_device_idx: usize = 1;
    let mut msg_scroll: usize = 0;
    let mut picker_state = FilePickerState::Closed;

    let (err_tx, err_rx) = mpsc::channel::<String>();

    // 1. TELEFONA BILDIRIM GONDER (Net okunabilir metin)
    let send_phone_notification = |text: String| {
        thread::spawn(move || {
            let _ = Command::new("adb")
                .args([
                    "-s", "192.168.1.50:5555",
                    "shell", "cmd", "notification", "post",
                    "-t", "⚡ NexusTUI Mesaj",
                    "msg_tag",
                    &text,
                ])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn();
        });
    };

    // 2. KABLOSUZ DOSYA GONDER (Tum dosyalar /sdcard/Download/NexusTUI/ altina gider)
    let send_file_to_phone = |filepath: String, tx: mpsc::Sender<String>| {
        thread::spawn(move || {
            let path = Path::new(&filepath);
            if !path.exists() {
                let _ = tx.send(format!("❌ [DOSYA BULUNAMADI]: {}", filepath));
                return;
            }

            let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("dosya");
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
            let is_media = matches!(ext.as_str(), "jpg" | "jpeg" | "png" | "webp" | "gif" | "mp4");

            // Standart tek klasor: Her sey /sdcard/Download/NexusTUI/ altina!
            let dest_folder = "/sdcard/Download/NexusTUI/";

            let _ = tx.send(format!("📤 [TRANSFER]: {} -> Download/NexusTUI/...", filename));

            // Klasor yoksa olustur
            let _ = Command::new("adb")
                .args(["-s", "192.168.1.50:5555", "shell", "mkdir", "-p", dest_folder])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();

            // Dosyayi gonder
            let status = Command::new("adb")
                .args([
                    "-s", "192.168.1.50:5555",
                    "push",
                    &filepath,
                    dest_folder,
                ])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();

            match status {
                Ok(s) if s.success() => {
                    let dest_file = format!("{}{}", dest_folder, filename);
                    let _ = tx.send(format!("✅ [TAMAMLANDI]: {} -> Download/NexusTUI/", filename));

                    // Resim ise galeride aninda gorunmesi icin media scan tetikle
                    if is_media {
                        let scan_uri = format!("file://{}", dest_file);
                        let _ = Command::new("adb")
                            .args([
                                "-s", "192.168.1.50:5555",
                                "shell", "am", "broadcast",
                                "-a", "android.intent.action.MEDIA_SCANNER_SCAN_FILE",
                                "-d", &scan_uri,
                            ])
                            .stdout(Stdio::null())
                            .stderr(Stdio::null())
                            .spawn();
                    }

                    // Bildirim firlat
                    let notif_text = format!("{} indirilenler klasörünüze kaydedildi!", filename);
                    let _ = Command::new("adb")
                        .args([
                            "-s", "192.168.1.50:5555",
                            "shell", "cmd", "notification", "post",
                            "-t", "📥 NexusTUI Dosya Geldi",
                            "file_tag",
                            &notif_text,
                        ])
                        .stdout(Stdio::null())
                        .stderr(Stdio::null())
                        .spawn();
                }
                _ => {
                    let _ = tx.send(format!("❌ [HATA]: {} gonderilemedi.", filename));
                }
            }
        });
    };

    // 3. TELEFONDAN DOSYA CEKME (Telefon -> PC ~/Downloads/)
    let pull_file_from_phone = |filename: String, tx: mpsc::Sender<String>| {
        thread::spawn(move || {
            let _ = tx.send(format!("📥 [ALINIYOR]: {} telefondan cekiliyor...", filename));
            let pc_dest = "/home/senatorherscher/Downloads/";

            let mut status = Command::new("adb")
                .args([
                    "-s", "192.168.1.50:5555",
                    "pull",
                    &format!("/sdcard/Download/NexusTUI/{}", filename),
                    pc_dest,
                ])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();

            if status.is_err() || !status.as_ref().map(|s| s.success()).unwrap_or(false) {
                status = Command::new("adb")
                    .args([
                        "-s", "192.168.1.50:5555",
                        "pull",
                        &format!("/sdcard/Download/{}", filename),
                        pc_dest,
                    ])
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status();
            }

            match status {
                Ok(s) if s.success() => {
                    let _ = tx.send(format!("✅ [ALINDI]: {} -> PC ~/Downloads/", filename));
                }
                _ => {
                    let _ = tx.send(format!("❌ [BULUNAMADI]: {} telefonda bulunamadi.", filename));
                }
            }
        });
    };

    // 4. EKRAN YANSITMA (scrcpy)
    let trigger_mirror = |target_str: &str, tx: mpsc::Sender<String>| {
        if target_str.contains("127.0.0.1") || target_str.contains("NexusTUI-PC") {
            let _ = tx.send("❌ [HATA]: NexusTUI-PC bir telefon degildir! Lutfen '↓' ile telefonu secin.".to_string());
            return;
        }

        let ip = "192.168.1.50:5555".to_string();
        let tx_clone = tx.clone();
        let _ = tx.send(format!("🚀 [SCRCPY 4.1]: {} (H.265, 12M, 60FPS) baslatiliyor...", ip));

        thread::spawn(move || {
            let child = Command::new("scrcpy")
                .arg("-s")
                .arg(&ip)
                .arg("--video-codec=h265")
                .arg("--video-bit-rate=12M")
                .arg("--max-size=1920")
                .arg("--max-fps=60")
                .arg("--video-buffer=30")
                .arg("--stay-awake")
                .arg("--always-on-top")
                .arg("--window-title=NexusTUI - Xiaomi Mi 11 Lite")
                .stdout(Stdio::null())
                .stderr(Stdio::piped())
                .spawn();

            if let Ok(mut proc) = child {
                if let Some(stderr) = proc.stderr.take() {
                    let reader = BufReader::new(stderr);
                    for line in reader.lines().flatten() {
                        if line.contains("ERROR") || line.contains("warn") || line.contains("Could not") {
                            let _ = tx_clone.send(format!("⚠️ [SCRCPY]: {}", line));
                        }
                    }
                }
            }
        });
    };

    loop {
        while let Some(msg) = net_client.try_recv() {
            if (msg.contains("katildi") || msg.contains("Client added")) && msg.contains("192.168.") {
                let dev_name = format!("Yeni Cihaz ({})", msg);
                if !devices.contains(&dev_name) {
                    devices.push(dev_name);
                }
            }
            messages.push(msg);
        }

        while let Ok(err) = err_rx.try_recv() {
            errors.push(err);
        }

        terminal.draw(|frame| {
            ui::draw(
                frame,
                &messages,
                &devices,
                &errors,
                &input,
                selected_device_idx,
                msg_scroll,
                &picker_state,
            );
        })?;

        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match &picker_state {
                        FilePickerState::FolderMenu => {
                            match key.code {
                                KeyCode::Esc => picker_state = FilePickerState::Closed,
                                KeyCode::Char('1') => {
                                    let files = list_folder_files("/home/senatorherscher/Downloads");
                                    picker_state = FilePickerState::FileMenu {
                                        folder_name: "Downloads".to_string(),
                                        files,
                                    };
                                }
                                KeyCode::Char('2') => {
                                    let files = list_folder_files("/home/senatorherscher/Pictures");
                                    picker_state = FilePickerState::FileMenu {
                                        folder_name: "Pictures".to_string(),
                                        files,
                                    };
                                }
                                KeyCode::Char('3') => {
                                    let files = list_folder_files("/home/senatorherscher/Documents");
                                    picker_state = FilePickerState::FileMenu {
                                        folder_name: "Documents".to_string(),
                                        files,
                                    };
                                }
                                KeyCode::Char('4') => {
                                    let files = list_folder_files("/home/senatorherscher/Desktop");
                                    picker_state = FilePickerState::FileMenu {
                                        folder_name: "Desktop".to_string(),
                                        files,
                                    };
                                }
                                _ => {}
                            }
                            continue;
                        }
                        FilePickerState::FileMenu { files, .. } => {
                            match key.code {
                                KeyCode::Esc => picker_state = FilePickerState::FolderMenu,
                                KeyCode::Char(c) if c.is_ascii_digit() && c != '0' => {
                                    let idx = (c as usize) - ('1' as usize);
                                    if let Some((_, full_path)) = files.get(idx) {
                                        send_file_to_phone(full_path.clone(), err_tx.clone());
                                        picker_state = FilePickerState::Closed;
                                    }
                                }
                                _ => {}
                            }
                            continue;
                        }
                        FilePickerState::ReceiveMenu { files } => {
                            match key.code {
                                KeyCode::Esc => picker_state = FilePickerState::Closed,
                                KeyCode::Char(c) if c.is_ascii_digit() && c != '0' => {
                                    let idx = (c as usize) - ('1' as usize);
                                    if let Some(target_file) = files.get(idx) {
                                        pull_file_from_phone(target_file.clone(), err_tx.clone());
                                        picker_state = FilePickerState::Closed;
                                    }
                                }
                                _ => {}
                            }
                            continue;
                        }
                        FilePickerState::Closed => {}
                    }

                    match key.code {
                        KeyCode::Esc => break,

                        KeyCode::F(3) => {
                            picker_state = FilePickerState::FolderMenu;
                        }

                        KeyCode::F(4) => {
                            let phone_files = list_phone_files();
                            picker_state = FilePickerState::ReceiveMenu { files: phone_files };
                        }

                        KeyCode::Up => {
                            if selected_device_idx > 0 {
                                selected_device_idx -= 1;
                            }
                        }
                        KeyCode::Down => {
                            if selected_device_idx + 1 < devices.len() {
                                selected_device_idx += 1;
                            }
                        }

                        KeyCode::PageUp => {
                            if msg_scroll > 0 {
                                msg_scroll -= 1;
                            }
                        }
                        KeyCode::PageDown => {
                            if msg_scroll + 1 < messages.len() {
                                msg_scroll += 1;
                            }
                        }

                        KeyCode::F(2) => {
                            if let Some(target_dev) = devices.get(selected_device_idx) {
                                trigger_mirror(target_dev, err_tx.clone());
                            }
                        }

                        KeyCode::Enter => {
                            let trimmed = input.trim().to_string();
                            if !trimmed.is_empty() {
                                if trimmed.starts_with("/mirror") {
                                    if let Some(target_dev) = devices.get(selected_device_idx) {
                                        trigger_mirror(target_dev, err_tx.clone());
                                    }
                                } else if trimmed.starts_with("/pull ") {
                                    let fname = trimmed.trim_start_matches("/pull ").trim().to_string();
                                    pull_file_from_phone(fname, err_tx.clone());
                                } else if trimmed.starts_with("/send ") || trimmed.starts_with("/file ") {
                                    let filepath = trimmed
                                        .trim_start_matches("/send ")
                                        .trim_start_matches("/file ")
                                        .trim()
                                        .to_string();
                                    send_file_to_phone(filepath, err_tx.clone());
                                } else {
                                    let _ = net_client.send_line(&trimmed);
                                    send_phone_notification(trimmed.clone());
                                }
                                input.clear();
                            }
                        }
                        KeyCode::Backspace => {
                            input.pop();
                        }
                        KeyCode::Char(c) => {
                            input.push(c);
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    println!("👋 NexusTUI Kapatildi.");
    Ok(())
}
