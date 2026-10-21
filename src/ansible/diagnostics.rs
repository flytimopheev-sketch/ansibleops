use crate::models::DiagResult;
use std::net::TcpStream;
use std::time::Duration;

pub fn check_host(host: &str, port: u16) -> DiagResult {
    let ping_ok = std::process::Command::new("ping")
        .args(if cfg!(windows) {
            vec!["-n", "1", "-w", "1000", host]
        } else {
            vec!["-c", "1", "-W", "1", host]
        })
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    let ssh_ok = TcpStream::connect_timeout(
        &format!("{}:22", host).parse().unwrap(),
        Duration::from_secs(2),
    )
    .is_ok();
    let port_ok = if port == 22 {
        ssh_ok
    } else {
        TcpStream::connect_timeout(
            &format!("{}:{}", host, port).parse().unwrap(),
            Duration::from_secs(2),
        )
        .is_ok()
    };
    DiagResult {
        host: host.into(),
        ping_ok,
        ssh_ok,
        port_ok,
        port,
    }
}
