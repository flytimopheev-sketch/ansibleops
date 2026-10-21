use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub fn send(server: &str, port: u16, host: &str, key: &str, value: &str) -> Result<String, String> {
    let clock = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let json = format!(
        r#"{{"request":"sender data","data":[{{"host":"{}","key":"{}","value":"{}"}}],"clock":{}}}"#,
        host, key, value, clock
    );
    let mut data = b"ZBXD\x01".to_vec();
    data.extend_from_slice(&(json.len() as u64).to_le_bytes());
    data.extend_from_slice(json.as_bytes());
    let mut stream = TcpStream::connect_timeout(
        &format!("{}:{}", server, port)
            .parse()
            .map_err(|e: std::net::AddrParseError| e.to_string())?,
        Duration::from_secs(3),
    )
    .map_err(|e| e.to_string())?;
    stream.write_all(&data).map_err(|e| e.to_string())?;
    let mut buf = Vec::new();
    stream.read_to_end(&mut buf).map_err(|e| e.to_string())?;
    Ok(if buf.len() > 13 {
        String::from_utf8_lossy(&buf[13..]).into_owned()
    } else {
        String::from_utf8_lossy(&buf).into_owned()
    })
}
