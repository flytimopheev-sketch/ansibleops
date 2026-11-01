use crate::models::TerraformConfig;
use anyhow::Result;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
pub struct TfEvent {
    pub line: String,
    pub is_error: bool,
}

pub struct TerraformRunner;

impl TerraformRunner {
    pub async fn run_cmd(
        cfg: &TerraformConfig,
        action: &str,
        tx: mpsc::Sender<TfEvent>,
    ) -> Result<bool> {
        let mut args = vec![action];
        if cfg.auto_approve && (action == "apply" || action == "destroy") {
            args.push("-auto-approve");
        }
        let mut cmd = Command::new("terraform");
        cmd.args(&args)
            .current_dir(&cfg.workdir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = cmd.spawn()?;
        let out = child.stdout.take().ok_or_else(|| {
            anyhow::anyhow!("не удалось получить stdout от процесса terraform")
        })?;
        let err = child.stderr.take().ok_or_else(|| {
            anyhow::anyhow!("не удалось получить stderr от процесса terraform")
        })?;
        let tx2 = tx.clone();
        let h1 = tokio::spawn(async move {
            let mut r = BufReader::new(out).lines();
            while let Ok(Some(l)) = r.next_line().await {
                let _ = tx2
                    .send(TfEvent {
                        line: l,
                        is_error: false,
                    })
                    .await;
            }
        });
        let h2 = tokio::spawn(async move {
            let mut r = BufReader::new(err).lines();
            while let Ok(Some(l)) = r.next_line().await {
                let _ = tx
                    .send(TfEvent {
                        line: l,
                        is_error: true,
                    })
                    .await;
            }
        });
        let read_timeout = std::time::Duration::from_secs(300);
        let _ = tokio::join!(
            tokio::time::timeout(read_timeout, h1),
            tokio::time::timeout(read_timeout, h2),
        );
        Ok(child.wait().await?.success())
    }
}
