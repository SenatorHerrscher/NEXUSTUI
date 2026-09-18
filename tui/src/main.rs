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

fn detect_phone_device(ip_with_port: &str) -> String {
    let clean_ip = ip_with_port.split(':').next().unwrap_or(ip_with_port);

    let market_out = Command::new("adb")
        .args(["-s", ip_with_port, "shell", "getprop", "ro.product.marketname"])
        .output();
    let model_out = Command::new("adb")
        .args(["-s", ip_with_port, "shell", "getprop", "ro.product.model"])
        .output();
    let mfg_out = Command::new("adb")
        .args(["-s", ip_with_port, "shell", "getprop", "ro.product.manufacturer"])
        .output();

    let market = market_out.ok().map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string()).unwrap_or_default();
    let model = model_out.ok().map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string()).unwrap_or_default();
    let mfg = mfg_out.ok().map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string()).unwrap_or_default();

    let display_name = if !market.is_empty() {
        if !mfg.is_empty() && !market.to_lowercase().contains(&mfg.to_lowercase()) {
            format!("{} {}", mfg, market)
        } else {
            market
        }
    } else if !model.is_empty() {
        if !mfg.is_empty() && !model.to_lowercase().contains(&mfg.to_lowercase()) {
            format!("{} {}", mfg, model)
        } else {
            model
        }
    } else {
        "Android Device".to_string()
    };

    format!("📱 {} ({})", display_name, clean_ip)
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
            eprintln!("❌ Failed to connect to Go server: {}", e);
            return Ok(());
        }
    };

    let username = std::env::var("USER").unwrap_or_else(|_| "User".to_string());
    let _ = net_client.send_line(&format!("/nick {}", username));

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut input = String::new();
    let mut messages: Vec<String> = Vec::new();
    let pc_device = format!("💻 {} (127.0.0.1)", username);
    let phone_device = detect_phone_device("192.168.1.50:5555");
    let mut devices: Vec<String> = vec![pc_device, phone_device];
    let mut errors: Vec<String> = vec![
        "ℹ️  System ready. Transferred files are stored in /sdcard/Download/NexusTUI/".to_string(),
    ];
    let mut selected_device_idx: usize = 1;
    let mut msg_scroll: usize = 0;
    let mut picker_state = FilePickerState::Closed;

    let (err_tx, err_rx) = mpsc::channel::<String>();

    // 1. SEND PHONE NOTIFICATION
    let send_phone_notification = |text: String| {
        thread::spawn(move || {
            let _ = Command::new("adb")
                .args([
                    "-s", "192.168.1.50:5555",
                    "shell", "cmd", "notification", "post",
                    "-t", "⚡ NexusTUI Message",
                    "msg_tag",
                    &text,
                ])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn();
        });
    };

    // 2. WIRELESS FILE TRANSFER (/sdcard/Download/NexusTUI/)
    let send_file_to_phone = |filepath: String, tx: mpsc::Sender<String>| {
        thread::spawn(move || {
            let path = Path::new(&filepath);
            if !path.exists() {
                let _ = tx.send(format!("❌ [FILE NOT FOUND]: {}", filepath));
                return;
            }

            let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("file");
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
            let is_media = matches!(ext.as_str(), "jpg" | "jpeg" | "png" | "webp" | "gif" | "mp4");

            let dest_folder = "/sdcard/Download/NexusTUI/";

            let _ = tx.send(format!("📤 [TRANSFER]: {} -> Download/NexusTUI/...", filename));

            let _ = Command::new("adb")
                .args(["-s", "192.168.1.50:5555", "shell", "mkdir", "-p", dest_folder])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();

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
                    let _ = tx.send(format!("✅ [COMPLETED]: {} -> Download/NexusTUI/", filename));

                    // Trigger Android Media Scanner so it appears instantly in Gallery album
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

                    // Send notification to phone
                    let notif_text = format!("{} saved to your Download/NexusTUI folder!", filename);
                    let _ = Command::new("adb")
                        .args([
                            "-s", "192.168.1.50:5555",
                            "shell", "cmd", "notification", "post",
                            "-t", "📥 NexusTUI File Received",
                            "file_tag",
                            &notif_text,
                        ])
                        .stdout(Stdio::null())
                        .stderr(Stdio::null())
                        .spawn();
                }
                _ => {
                    let _ = tx.send(format!("❌ [ERROR]: Failed to transfer {}.", filename));
                }
            }
        });
    };

    // 3. PULL FILE FROM PHONE (Phone -> PC ~/Downloads/)
    let pull_file_from_phone = |filename: String, tx: mpsc::Sender<String>| {
        thread::spawn(move || {
            let _ = tx.send(format!("📥 [PULLING]: Fetching {} from phone...", filename));
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
                    let _ = tx.send(format!("✅ [RECEIVED]: {} -> PC ~/Downloads/", filename));
                }
                _ => {
                    let _ = tx.send(format!("❌ [NOT FOUND]: {} not found on phone.", filename));
                }
            }
        });
    };

    // 4. SCREEN MIRRORING (scrcpy with screen-off and PC audio forwarding)
    let trigger_mirror = |target_str: &str, tx: mpsc::Sender<String>| {
        if target_str.contains("127.0.0.1") || target_str.contains("💻") {
            let _ = tx.send("❌ [ERROR]: Cannot mirror PC! Please select target phone using '↓'.".to_string());
            return;
        }

        let ip = if let (Some(start), Some(end)) = (target_str.find('('), target_str.find(')')) {
            let extracted = &target_str[start + 1..end];
            if extracted.contains(':') {
                extracted.to_string()
            } else {
                format!("{}:5555", extracted)
            }
        } else {
            "192.168.1.50:5555".to_string()
        };

        let target_title = target_str.replace("📱", "").trim().to_string();
        let tx_clone = tx.clone();
        let _ = tx.send(format!("🚀 [SCRCPY]: Starting {} (H.265, 60FPS, Screen-Off, PC Audio)...", ip));

        thread::spawn(move || {
            let child = Command::new("scrcpy")
                .arg("-s")
                .arg(&ip)
                .arg("--video-codec=h265")
                .arg("--video-bit-rate=12M")
                .arg("--max-size=1920")
                .arg("--max-fps=60")
                .arg("--video-buffer=30")
                .arg("--turn-screen-off")
                .arg("--stay-awake")
                .arg("--audio-source=output")
                .arg("--audio-buffer=50")
                .arg("--always-on-top")
                .arg(format!("--window-title=NexusTUI - {}", target_title))
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
            if (msg.contains("Client added") || msg.contains("joined") || msg.contains("katildi")) && msg.contains("192.168.") {
                let dev_name = format!("New Device ({})", msg);
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

    println!("👋 NexusTUI Closed.");
    Ok(())
}
