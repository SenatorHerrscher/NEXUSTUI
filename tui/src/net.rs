use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::sync::mpsc::{self, Receiver};
use std::thread;

pub struct NetworkClient {
    stream: TcpStream,
    rx: Receiver<String>,
}

impl NetworkClient {
    pub fn connect(addr: &str) -> std::io::Result<Self> {
        let stream = TcpStream::connect(addr)?;
        let read_stream = stream.try_clone()?;
        let (tx, rx) = mpsc::channel();

        thread::spawn(move || {
            let reader = BufReader::new(read_stream);
            for line in reader.lines() {
                if let Ok(l) = line {
                    if tx.send(l).is_err() {
                        break;
                    }
                } else {
                    break;
                }
            }
        });
        Ok(Self { stream, rx })
    }

    pub fn send_line(&mut self, text: &str) -> std::io::Result<()> {
        writeln!(self.stream, "{}", text)?;
        self.stream.flush()?;
        Ok(())
    }

    pub fn try_recv(&self) -> Option<String> {
        self.rx.try_recv().ok()
    }
}
