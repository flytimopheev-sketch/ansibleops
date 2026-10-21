use crate::models::{SshConfig, TaskEvent, TaskStatus};
use anyhow::Result;
use chrono::Local;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;

pub struct AnsibleRunner;

impl AnsibleRunner {
    /// Запуск playbook без возможности отмены (обёртка над [`Self::run_cancellable`]).
    #[allow(dead_code)]
    pub async fn run(
        playbook: &str,
        inventory: &str,
        extra_vars: Option<&str>,
        ssh: &SshConfig,
        tx: mpsc::Sender<TaskEvent>,
    ) -> Result<bool> {
        Self::run_cancellable(
            playbook,
            inventory,
            extra_vars,
            ssh,
            tx,
            std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            Vec::new(),
        )
        .await
    }

    /// Запуск с поддержкой отмены: установка cancel=true прерывает процесс.
    /// `redact` — список секретных значений, которые маскируются в выводе.
    pub async fn run_cancellable(
        playbook: &str,
        inventory: &str,
        extra_vars: Option<&str>,
        ssh: &SshConfig,
        tx: mpsc::Sender<TaskEvent>,
        cancel: std::sync::Arc<std::sync::atomic::AtomicBool>,
        redact: Vec<String>,
    ) -> Result<bool> {
        use std::sync::atomic::Ordering;
        let cmd_name = if cfg!(windows) {
            "ansible-playbook.cmd"
        } else {
            "ansible-playbook"
        };

        let mut cmd = Command::new(cmd_name);
        cmd.arg(playbook)
            .arg("-i")
            .arg(inventory)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        if let Some(v) = extra_vars {
            if !v.is_empty() {
                cmd.arg("-e").arg(v);
            }
        }
        if let Some(k) = &ssh.private_key {
            cmd.arg("--private-key").arg(k);
        }
        if ssh.use_become {
            cmd.arg("--become");
            if !ssh.become_user.is_empty() {
                cmd.arg("--become-user").arg(&ssh.become_user);
            }
        }
        if ssh.forks > 0 {
            cmd.arg("-f").arg(ssh.forks.to_string());
        }

        let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => {
                // Мок-режим включается только явно: ANSIBLEOPS_MOCK=1.
                if std::env::var("ANSIBLEOPS_MOCK").as_deref() == Ok("1") {
                    eprintln!("⚠️  Включён мок-режим (ANSIBLEOPS_MOCK=1): {cmd_name} не найден ({e}). Результаты НЕ настоящие.");
                    return Self::run_mock(playbook, inventory, tx).await;
                }
                let _ = tx.send(TaskEvent { host: "all".into(), task: "❌ Ошибка запуска".into(), status: TaskStatus::Failed, duration_ms: 0, output: format!("Не удалось запустить {cmd_name}: {e}. Установите Ansible (ansible-core) и убедитесь, что он доступен в PATH."), timestamp: Local::now() }).await;
                return Ok(false);
            }
        };

        let stdout = child.stdout.take().expect("stdout piped above");
        let mut reader = BufReader::new(stdout).lines();
        let mut success = true;
        let mut cancelled = false;
        while let Ok(Some(line)) = reader.next_line().await {
            if cancel.load(Ordering::Relaxed) {
                cancelled = true;
                break;
            }
            if let Some(mut ev) = Self::parse(&line) {
                ev.output = Self::redact(&ev.output, &redact);
                ev.task = Self::redact(&ev.task, &redact);
                let _ = tx.send(ev).await;
            }
        }
        if cancelled {
            let _ = child.kill().await;
            let _ = child.wait().await;
            let _ = tx
                .send(TaskEvent {
                    host: "all".into(),
                    task: "⏹ Остановлено пользователем".into(),
                    status: TaskStatus::Failed,
                    duration_ms: 0,
                    output: "Прервано пользователем (SIGKILL процесса ansible-playbook)".into(),
                    timestamp: Local::now(),
                })
                .await;
            return Ok(false);
        }
        if let Ok(st) = child.wait().await {
            if !st.success() {
                success = false;
            }
        }
        Ok(success)
    }

    async fn run_mock(
        playbook: &str,
        _inventory: &str,
        tx: mpsc::Sender<TaskEvent>,
    ) -> Result<bool> {
        let hosts = vec!["web1", "web2", "db1"];
        let tasks = vec![
            ("Gathering Facts", TaskStatus::Ok),
            ("Ping hosts", TaskStatus::Ok),
            ("Install packages", TaskStatus::Changed),
            ("Start service", TaskStatus::Ok),
        ];

        let _ = tx.send(TaskEvent { host: "all".into(), task: "⚠️ MOCK-РЕЖИМ".into(), status: TaskStatus::Pending, duration_ms: 0, output: "⚠️ ansible-playbook не найден — работает MOCK-режим с фиктивными хостами web1/web2/db1. Результаты НЕ настоящие!".into(), timestamp: Local::now() }).await;
        let _ = tx
            .send(TaskEvent {
                host: "all".into(),
                task: format!("[MOCK] PLAY [{playbook}]"),
                status: TaskStatus::Pending,
                duration_ms: 0,
                output: format!("\n[MOCK] PLAY [{playbook}] ****"),
                timestamp: Local::now(),
            })
            .await;
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;

        for (task_name, default_status) in &tasks {
            let _ = tx
                .send(TaskEvent {
                    host: "all".into(),
                    task: task_name.to_string(),
                    status: TaskStatus::Running,
                    duration_ms: 0,
                    output: format!("\nTASK [{task_name}] ****"),
                    timestamp: Local::now(),
                })
                .await;
            tokio::time::sleep(std::time::Duration::from_millis(300)).await;

            for host in &hosts {
                let status = if *host == "db1" && *task_name == "Install packages" {
                    TaskStatus::Failed
                } else {
                    *default_status
                };
                let icon = match status {
                    TaskStatus::Ok => "ok",
                    TaskStatus::Changed => "changed",
                    TaskStatus::Failed => "fatal",
                    TaskStatus::Skipped => "skipping",
                    _ => "ok",
                };
                let _ = tx
                    .send(TaskEvent {
                        host: host.to_string(),
                        task: task_name.to_string(),
                        status,
                        duration_ms: 150,
                        output: format!("[MOCK] {icon}: [{host}]"),
                        timestamp: Local::now(),
                    })
                    .await;
                tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            }
        }
        let _ = tx.send(TaskEvent { host: "all".into(), task: "PLAY RECAP".into(), status: TaskStatus::Pending, duration_ms: 0, output: "\nPLAY RECAP ****\nweb1: ok=3 changed=1 failed=0\nweb2: ok=3 changed=1 failed=0\ndb1: ok=2 changed=0 failed=1".into(), timestamp: Local::now() }).await;
        Ok(false)
    }

    fn parse(line: &str) -> Option<TaskEvent> {
        let (status, rest) = if let Some(r) = line.strip_prefix("ok: ") {
            (TaskStatus::Ok, r)
        } else if let Some(r) = line.strip_prefix("changed: ") {
            (TaskStatus::Changed, r)
        } else if let Some(r) = line.strip_prefix("fatal: ") {
            (TaskStatus::Failed, r)
        } else {
            (TaskStatus::Skipped, line.strip_prefix("skipping: ")?)
        };
        let host = rest
            .trim()
            .trim_start_matches('[')
            .split(']')
            .next()
            .unwrap_or("?")
            .into();
        Some(TaskEvent {
            host,
            task: line.chars().take(120).collect(),
            status,
            duration_ms: 0,
            output: line.into(),
            timestamp: Local::now(),
        })
    }

    /// Парсинг строки PLAY RECAP: "web1 : ok=3 changed=1 failed=0 skipped=0"
    #[allow(dead_code)]
    pub fn parse_recap_line(line: &str) -> Option<(String, usize, usize, usize, usize)> {
        let (host, stats) = line.split_once(':')?;
        let get = |key: &str| {
            stats
                .split_whitespace()
                .find_map(|t| t.strip_prefix(key).and_then(|v| v.parse::<usize>().ok()))
                .unwrap_or(0)
        };
        let (ok, changed, failed, skipped) =
            (get("ok="), get("changed="), get("failed="), get("skipped="));
        if ok + changed + failed + skipped == 0 {
            return None;
        }
        Some((host.trim().to_string(), ok, changed, failed, skipped))
    }

    /// Маскирование секретов в строке вывода.
    pub fn redact(line: &str, secrets: &[String]) -> String {
        let mut out = line.to_string();
        for s in secrets {
            if !s.is_empty() {
                out = out.replace(s, "****");
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redact_masks_secrets() {
        let secrets = vec!["SuperSecret123".to_string()];
        let out = AnsibleRunner::redact("fatal: [h1]: password=SuperSecret123", &secrets);
        assert_eq!(out, "fatal: [h1]: password=****");
        assert!(!out.contains("SuperSecret123"));
    }

    #[test]
    fn test_redact_empty_secret_ignored() {
        assert_eq!(
            AnsibleRunner::redact("ok: [h1]", &["".to_string()]),
            "ok: [h1]"
        );
    }

    #[test]
    fn test_parse_recap_line() {
        let r =
            AnsibleRunner::parse_recap_line("web1 : ok=3 changed=1 failed=0 skipped=2").unwrap();
        assert_eq!(r, ("web1".to_string(), 3, 1, 0, 2));
        assert!(AnsibleRunner::parse_recap_line("PLAY RECAP ****").is_none());
    }
}
