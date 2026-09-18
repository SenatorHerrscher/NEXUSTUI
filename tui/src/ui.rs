use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};

#[derive(Clone)]
pub enum FilePickerState {
    Closed,
    FolderMenu,
    FileMenu {
        folder_name: String,
        files: Vec<(String, String)>,
    },
    ReceiveMenu {
        files: Vec<String>,
    },
}

pub fn draw(
    frame: &mut Frame,
    messages: &[String],
    devices: &[String],
    errors: &[String],
    input: &str,
    selected_device_idx: usize,
    msg_scroll: usize,
    picker_state: &FilePickerState,
) {
    let size = frame.area();

    // Dikey Duzen
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Baslik
            Constraint::Min(10),   // Orta Govde
            Constraint::Length(3), // Komut / Mesaj Girisi
            Constraint::Length(1), // Kisayol bari
        ])
        .split(size);

    // 1. HEADER
    let header_text = vec![Line::from(vec![
        Span::styled(
            " ⚡ NEXUSTUI ",
            Style::default().fg(Color::Black).bg(Color::Green).add_modifier(Modifier::BOLD),
        ),
        Span::raw(" LAN HUB // Port: 9000 // Host: 192.168.1.11 // Status: "),
        Span::styled("● ACTIVE", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
    ])];
    let header = Paragraph::new(header_text).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Green)),
    );
    frame.render_widget(header, chunks[0]);

    // 2. MAIN BODY: Left 35% (Devices), Right 65% (Messages + Diagnostics)
    let body_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(35),
            Constraint::Percentage(65),
        ])
        .split(chunks[1]);

    // LEFT: CONNECTED DEVICES
    let device_items: Vec<ListItem> = if devices.is_empty() {
        vec![ListItem::new(Span::styled(
            " (Waiting for devices...)",
            Style::default().fg(Color::DarkGray),
        ))]
    } else {
        devices
            .iter()
            .enumerate()
            .map(|(i, dev)| {
                let is_selected = i == selected_device_idx;
                let prefix = if is_selected { " ▶ " } else { "   " };
                let style = if is_selected {
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::Cyan)
                };
                ListItem::new(Line::from(vec![
                    Span::styled(prefix, style),
                    Span::styled(dev.as_str(), style),
                ]))
            })
            .collect()
    };
    let devices_list = List::new(device_items).block(
        Block::default()
            .title(" 📱 Devices [↑/↓ | F2] ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan)),
    );
    frame.render_widget(devices_list, body_chunks[0]);

    // RIGHT: TOP MESSAGES (60%), BOTTOM SYSTEM & DIAGNOSTICS LOG (40%)
    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(60),
            Constraint::Percentage(40),
        ])
        .split(body_chunks[1]);

    // RIGHT TOP: MESSAGES & CLIPBOARD
    let visible_msgs: Vec<ListItem> = messages
        .iter()
        .skip(msg_scroll)
        .map(|msg| {
            let style = if msg.contains("[CLIPBOARD]") || msg.contains("[PANO]") || msg.contains("CLIPBOARD") {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else if msg.contains("[SYSTEM]") || msg.contains("[SISTEM]") {
                Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            ListItem::new(Span::styled(msg.as_str(), style))
        })
        .collect();
    let messages_list = List::new(visible_msgs).block(
        Block::default()
            .title(format!(" 💬 Live Stream & Clipboard [PgUp/PgDn: {}] ", msg_scroll))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Green)),
    );
    frame.render_widget(messages_list, right_chunks[0]);

    // RIGHT BOTTOM: SYSTEM & DIAGNOSTICS LOGS
    let error_items: Vec<ListItem> = if errors.is_empty() {
        vec![ListItem::new(Span::styled(
            " ✔️ System stable, no active errors.",
            Style::default().fg(Color::DarkGray),
        ))]
    } else {
        errors
            .iter()
            .rev()
            .take(6)
            .rev()
            .map(|err| {
                let style = if err.contains("❌") || err.contains("ERROR") {
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
                } else if err.contains("⚠️") || err.contains("warn") {
                    Style::default().fg(Color::Yellow)
                } else if err.contains("✅") {
                    Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::Cyan)
                };
                ListItem::new(Span::styled(err.as_str(), style))
            })
            .collect()
    };
    let error_list = List::new(error_items).block(
        Block::default()
            .title(" 🚨 System & Transfer Log ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Red)),
    );
    frame.render_widget(error_list, right_chunks[1]);

    // 3. BOTTOM INPUT LINE
    let input_text = format!(" > {}", input);
    let input_widget = Paragraph::new(input_text).block(
        Block::default()
            .title(" ⌨️  Message / Command (F3: Send, F4: Pull, Enter: Send) ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow)),
    );
    frame.render_widget(input_widget, chunks[2]);

    // 4. SHORTCUTS BAR
    let shortcuts = Line::from(vec![
        Span::styled(" [↑/↓] ", Style::default().fg(Color::Black).bg(Color::Cyan)),
        Span::raw(" Select  "),
        Span::styled(" [F2] ", Style::default().fg(Color::Black).bg(Color::Green)),
        Span::raw(" Mirror  "),
        Span::styled(" [F3] ", Style::default().fg(Color::Black).bg(Color::Magenta).add_modifier(Modifier::BOLD)),
        Span::raw(" Send File 📤  "),
        Span::styled(" [F4] ", Style::default().fg(Color::Black).bg(Color::LightBlue).add_modifier(Modifier::BOLD)),
        Span::raw(" Pull File 📥  "),
        Span::styled(" [Esc] ", Style::default().fg(Color::Black).bg(Color::Red)),
        Span::raw(" Quit "),
    ]);
    frame.render_widget(Paragraph::new(shortcuts), chunks[3]);

    // --- MODAL POPUP ---
    match picker_state {
        FilePickerState::Closed => {}
        FilePickerState::FolderMenu => {
            let popup_area = centered_rect(60, 45, size);
            frame.render_widget(Clear, popup_area);

            let items = vec![
                ListItem::new(Span::styled(" [1] 📂 Downloads  (/home/senatorherscher/Downloads)", Style::default().fg(Color::Cyan))),
                ListItem::new(Span::styled(" [2] 📂 Pictures   (/home/senatorherscher/Pictures)", Style::default().fg(Color::Cyan))),
                ListItem::new(Span::styled(" [3] 📂 Documents  (/home/senatorherscher/Documents)", Style::default().fg(Color::Cyan))),
                ListItem::new(Span::styled(" [4] 📂 Desktop    (/home/senatorherscher/Desktop)", Style::default().fg(Color::Cyan))),
                ListItem::new(Span::raw("")),
                ListItem::new(Span::styled(" 💡 Hint: Press [1-4] to select directory. Cancel: [Esc]", Style::default().fg(Color::DarkGray))),
            ];

            let modal = List::new(items).block(
                Block::default()
                    .title(" 📁 Send to Phone: Select Directory [1-4] ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)),
            );
            frame.render_widget(modal, popup_area);
        }
        FilePickerState::FileMenu { folder_name, files } => {
            let popup_area = centered_rect(65, 55, size);
            frame.render_widget(Clear, popup_area);

            let mut items: Vec<ListItem> = if files.is_empty() {
                vec![ListItem::new(Span::styled(" (No files found in this directory)", Style::default().fg(Color::DarkGray)))]
            } else {
                files
                    .iter()
                    .enumerate()
                    .map(|(idx, (name, _))| {
                        ListItem::new(Span::styled(
                            format!(" [{}] 📄 {}", idx + 1, name),
                            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                        ))
                    })
                    .collect()
            };

            items.push(ListItem::new(Span::raw("")));
            items.push(ListItem::new(Span::styled(
                " 🚀 Press [1-9] to send file to phone immediately! [Esc: Back]",
                Style::default().fg(Color::Green),
            )));

            let modal = List::new(items).block(
                Block::default()
                    .title(format!(" 📁 {} -> Send to Phone [1-9] ", folder_name))
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            );
            frame.render_widget(modal, popup_area);
        }
        FilePickerState::ReceiveMenu { files } => {
            let popup_area = centered_rect(65, 55, size);
            frame.render_widget(Clear, popup_area);

            let mut items: Vec<ListItem> = if files.is_empty() {
                vec![ListItem::new(Span::styled(" (No files found on phone)", Style::default().fg(Color::DarkGray)))]
            } else {
                files
                    .iter()
                    .enumerate()
                    .map(|(idx, name)| {
                        ListItem::new(Span::styled(
                            format!(" [{}] 📱 {}", idx + 1, name),
                            Style::default().fg(Color::LightBlue).add_modifier(Modifier::BOLD),
                        ))
                    })
                    .collect()
            };

            items.push(ListItem::new(Span::raw("")));
            items.push(ListItem::new(Span::styled(
                " 📥 Press [1-8] to pull file to PC ~/Downloads! [Esc: Cancel]",
                Style::default().fg(Color::Green),
            )));

            let modal = List::new(items).block(
                Block::default()
                    .title(" 📥 Pull File from Phone [1-8] ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::LightBlue).add_modifier(Modifier::BOLD)),
            );
            frame.render_widget(modal, popup_area);
        }
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
