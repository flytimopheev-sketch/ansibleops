use std::io::{Read, Write};
use std::net::TcpListener;
use std::time::{SystemTime, UNIX_EPOCH};

/// Генерирует криптографически стойкий bearer-токен (24 случайных байта, hex-кодировка).
pub fn generate_token() -> String {
    use rand::RngCore;
    const HEX_CHARS: &[u8; 16] = b"0123456789abcdef";
    let mut rng = rand::thread_rng();
    let mut buf = [0u8; 24];
    rng.fill_bytes(&mut buf);
    let mut hex = Vec::with_capacity(48);
    for &b in &buf {
        hex.push(HEX_CHARS[(b >> 4) as usize]);
        hex.push(HEX_CHARS[(b & 0x0F) as usize]);
    }
    String::from_utf8(hex).expect("hex is valid utf-8")
}

/// Извлекает bearer-токен из заголовка `Authorization: Bearer <token>` HTTP-запроса.
fn extract_bearer_token(req: &str) -> Option<&str> {
    req.lines()
        .find(|line| line.to_ascii_lowercase().starts_with("authorization:"))
        .and_then(|line| line.split_once(':'))
        .and_then(|(_, value)| value.trim().strip_prefix("Bearer "))
}

/// Безопасный мини-веб-сервер: только GET, bearer-токен, лог запросов.
/// По умолчанию слушает 127.0.0.1. НЕ запускайте на 0.0.0.0 без понимания рисков.
pub fn start(port: u16, bind_addr: &str, token: &str, history_dir: String) -> Result<(), String> {
    let listener = TcpListener::bind((bind_addr, port)).map_err(|e| e.to_string())?;
    let token = token.to_string();
    let log_path = if history_dir.is_empty() {
        None
    } else {
        let p = std::path::Path::new(&history_dir).join("api_access.log");
        let _ = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&p);
        Some(p)
    };
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let mut s = match stream {
                Ok(s) => s,
                Err(_) => continue,
            };
            let mut buf = Vec::new();
            let mut chunk = [0u8; 1024];
            // Читаем до конца заголовков (ограничение 64 КБ)
            loop {
                match s.read(&mut chunk) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        buf.extend_from_slice(&chunk[..n]);
                        if buf.len() > 65536 || buf.windows(4).any(|w| w == b"\r\n\r\n") {
                            break;
                        }
                    }
                }
            }
            let req = String::from_utf8_lossy(&buf).to_string();
            let first_line = req.lines().next().unwrap_or("").to_string();
            let (method, path) = {
                let mut it = first_line.split_whitespace();
                (
                    it.next().unwrap_or("").to_string(),
                    it.next().unwrap_or("").to_string(),
                )
            };
            let authorized = !token.is_empty()
                && extract_bearer_token(&req) == Some(token);
            let (status, body, ctype) = match (method.as_str(), authorized) {
                ("GET", true) if path.starts_with("/api/history") => {
                    let mut items = Vec::new();
                    if let Ok(rd) = std::fs::read_dir(&history_dir) {
                        for e in rd.flatten() { items.push(format!("\"{}\"", e.file_name().to_string_lossy())); }
                    }
                    (200u16, format!("[{}]", items.join(",")), "application/json")
                }
                ("GET", true) => (
                    200,
                    "<html><body style='background:#1a1b26;color:#c0caf5;font-family:sans-serif'><h1>⚡ AnsibleOps Web API</h1><p>Status: <b style='color:#9ece6a'>online</b></p><p><a style='color:#7aa2f7' href='/api/history'>/api/history</a></p></body></html>".to_string(),
                    "text/html",
                ),
                (_, false) => (401, "{\"error\":\"unauthorized\"}".into(), "application/json"),
                _ => (405, "{\"error\":\"method not allowed — только GET (read-only)\"}".into(), "application/json"),
            };
            if let Some(lp) = &log_path {
                if let Ok(mut lg) = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(lp)
                {
                    let ts = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .map(|d| d.as_secs())
                        .unwrap_or(0);
                    let _ = writeln!(
                        lg,
                        "{} {} {} {} -> {}",
                        ts,
                        s.peer_addr().map(|a| a.to_string()).unwrap_or_default(),
                        method,
                        path,
                        status
                    );
                }
            }
            let reason = match status {
                200 => "OK",
                401 => "Unauthorized",
                405 => "Method Not Allowed",
                _ => "Not Found",
            };
            let resp = format!("HTTP/1.1 {} {}\r\nContent-Type: {}; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", status, reason, ctype, body.len(), body);
            let _ = s.write_all(resp.as_bytes());
        }
    });
    Ok(())
}
