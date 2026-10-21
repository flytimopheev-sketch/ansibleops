use crate::models::{AdminAction, BackgroundJob};
use anyhow::Result;
use chrono::Local;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;
use uuid::Uuid;

pub struct AdHocRunner;

impl AdHocRunner {
    pub async fn run_admin_action(
        action: AdminAction,
        inventory: &str,
        job_tx: mpsc::Sender<BackgroundJob>,
    ) -> Result<()> {
        let job_id = Uuid::new_v4().to_string();
        let job_name = format!("{}: {}", action.module_type_to_str(), action.name);
        let initial_job = BackgroundJob {
            id: job_id.clone(),
            name: job_name.clone(),
            status: "Running".into(),
            progress: 0.1,
            output: vec![format!(
                "Starting {} on {}...",
                action.module_type_to_str(),
                action.target_hosts
            )],
            started_at: Local::now(),
        };
        let _ = job_tx.send(initial_job.clone()).await;
        let mut cmd = Command::new("ansible");
        cmd.arg(&action.target_hosts)
            .arg("-i")
            .arg(inventory)
            .arg("-m")
            .arg(action.module_type_to_ansible_module())
            .arg("-a")
            .arg(format!("{} {}", action.name, action.extra_args));
        if action.use_become {
            cmd.arg("--become");
        }
        if action.check_mode {
            cmd.arg("--check");
        }
        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
        let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => {
                let mut f = initial_job;
                f.status = "Failed".into();
                f.progress = 1.0;
                f.output.push(format!("Error: {}", e));
                let _ = job_tx.send(f).await;
                return Ok(());
            }
        };
        let stdout = child.stdout.take().unwrap();
        let mut reader = BufReader::new(stdout).lines();
        let mut output_lines = initial_job.output.clone();
        let mut base_job = initial_job;
        while let Ok(Some(line)) = reader.next_line().await {
            output_lines.push(line.clone());
            let mut u = base_job.clone();
            u.output = output_lines.clone();
            u.progress = 0.5;
            let _ = job_tx.send(u).await;
        }
        let status = child.wait().await?;
        base_job.output = output_lines;
        base_job.progress = 1.0;
        base_job.status = if status.success() {
            "Success".into()
        } else {
            "Failed".into()
        };
        let _ = job_tx.send(base_job).await;
        Ok(())
    }
}

impl AdminAction {
    pub fn module_type_to_str(&self) -> &'static str {
        match self.module_type {
            crate::models::AdminModuleType::Package => "Package",
            crate::models::AdminModuleType::Service => "Service",
            crate::models::AdminModuleType::User => "User",
            crate::models::AdminModuleType::File => "File",
        }
    }
    pub fn module_type_to_ansible_module(&self) -> &'static str {
        match self.module_type {
            crate::models::AdminModuleType::Package => "package",
            crate::models::AdminModuleType::Service => "service",
            crate::models::AdminModuleType::User => "user",
            crate::models::AdminModuleType::File => "file",
        }
    }
}

// Универсальный потоковый запуск ad-hoc команды (для Ops/Domain/Workflow)
pub fn run_raw(
    hosts: &str,
    module: &str,
    args: &str,
    use_become: bool,
    tx: std::sync::mpsc::Sender<String>,
) {
    let mut cmd = std::process::Command::new("ansible");
    cmd.arg(hosts).arg("-m").arg(module);
    if !args.is_empty() {
        cmd.arg("-a").arg(args);
    }
    if use_become {
        cmd.arg("--become");
    }
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    let _ = tx.send(format!("$ ansible {} -m {} {}", hosts, module, args));
    match cmd.output() {
        Ok(o) => {
            for l in String::from_utf8_lossy(&o.stdout).lines() {
                let _ = tx.send(l.to_string());
            }
            for l in String::from_utf8_lossy(&o.stderr).lines() {
                let _ = tx.send(format!("ERR: {}", l));
            }
            let _ = tx.send(format!("exit: {}", o.status.code().unwrap_or(-1)));
        }
        Err(e) => {
            let _ = tx.send(format!("error: {}", e));
        }
    }
}
