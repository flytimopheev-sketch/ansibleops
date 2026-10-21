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
        let out = child.stdout.take().unwrap();
        let err = child.stderr.take().unwrap();
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
        let _ = tokio::join!(h1, h2);
        Ok(child.wait().await?.success())
    }
}
