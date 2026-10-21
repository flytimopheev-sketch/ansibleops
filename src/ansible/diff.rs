use chrono::{DateTime, Local};

#[derive(Debug, Clone)]
pub struct FileChange {
    pub host: String,
    pub path: String,
    pub before: String,
    pub after: String,
    pub added_lines: usize,
    pub removed_lines: usize,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FileVersion {
    pub file: String,
    pub host: String,
    pub content: String,
    pub saved_at: DateTime<Local>,
    pub source: String,
}

pub fn parse_check_output(output: &str) -> Vec<FileChange> {
    let mut changes = Vec::new();
    let mut current_host: Option<String> = None;
    let mut current_path: Option<String> = None;
    let mut before_lines: Vec<String> = Vec::new();
    let mut after_lines: Vec<String> = Vec::new();

    for line in output.lines() {
        if line.starts_with("TASK [") || line.starts_with("PLAY [") {
            if let (Some(h), Some(p)) = (current_host.take(), current_path.take()) {
                if !before_lines.is_empty() || !after_lines.is_empty() {
                    let added = after_lines.iter().filter(|l| l.starts_with('+')).count();
                    let removed = before_lines.iter().filter(|l| l.starts_with('-')).count();
                    changes.push(FileChange {
                        host: h,
                        path: p,
                        before: before_lines.join("\n"),
                        after: after_lines.join("\n"),
                        added_lines: added,
                        removed_lines: removed,
                    });
                }
                before_lines.clear();
                after_lines.clear();
            }
            continue;
        }
        if line.contains(" => ") && line.starts_with("changed:") {
            if let Some(h) = line.split('[').nth(1).and_then(|s| s.split(']').next()) {
                current_host = Some(h.to_string());
            }
        }
        if line.trim().starts_with("path:") || line.trim().starts_with("dest:") {
            if let Some(p) = line.split(':').nth(1) {
                current_path = Some(p.trim().trim_matches('"').to_string());
            }
        }
        if line.starts_with("---") && line.contains('/') {
            current_path = Some(line.trim_start_matches("--- ").trim().to_string());
        }
        if line.starts_with('-') && !line.starts_with("---") {
            before_lines.push(line.to_string());
        }
        if line.starts_with('+') && !line.starts_with("+++") {
            after_lines.push(line.to_string());
        }
    }
    if let (Some(h), Some(p)) = (current_host, current_path) {
        if !before_lines.is_empty() || !after_lines.is_empty() {
            let added = after_lines.iter().filter(|l| l.starts_with('+')).count();
            let removed = before_lines.iter().filter(|l| l.starts_with('-')).count();
            changes.push(FileChange {
                host: h,
                path: p,
                before: before_lines.join("\n"),
                after: after_lines.join("\n"),
                added_lines: added,
                removed_lines: removed,
            });
        }
    }
    changes
}

pub struct VersionStore {
    pub dir: std::path::PathBuf,
}

impl VersionStore {
    pub fn new() -> Result<Self, String> {
        let dir = dirs::data_local_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("ansibleops")
            .join("versions");
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        Ok(Self { dir })
    }
    pub fn save(&self, host: &str, file: &str, content: &str, source: &str) -> Result<(), String> {
        let ts = Local::now().format("%Y%m%d_%H%M%S");
        let safe_host = host.replace(|c: char| !c.is_alphanumeric(), "_");
        let safe_file = file.replace('/', "_").trim_start_matches('_').to_string();
        let fname = format!("{}_{}_{}.json", safe_host, safe_file, ts);
        let v = FileVersion {
            file: file.to_string(),
            host: host.to_string(),
            content: content.to_string(),
            saved_at: Local::now(),
            source: source.to_string(),
        };
        let json = serde_json::to_string_pretty(&v).map_err(|e| e.to_string())?;
        std::fs::write(self.dir.join(fname), json).map_err(|e| e.to_string())
    }
    pub fn list(&self, host: &str, file: &str) -> Vec<FileVersion> {
        let safe_host = host.replace(|c: char| !c.is_alphanumeric(), "_");
        let safe_file = file.replace('/', "_").trim_start_matches('_').to_string();
        let prefix = format!("{}_{}", safe_host, safe_file);
        let mut v = Vec::new();
        if let Ok(rd) = std::fs::read_dir(&self.dir) {
            for e in rd.flatten() {
                if let Some(n) = e.file_name().to_str() {
                    if n.starts_with(&prefix) && n.ends_with(".json") {
                        if let Ok(c) = std::fs::read_to_string(e.path()) {
                            if let Ok(fv) = serde_json::from_str::<FileVersion>(&c) {
                                v.push(fv);
                            }
                        }
                    }
                }
            }
        }
        v.sort_by_key(|b| std::cmp::Reverse(b.saved_at));
        v
    }
    pub fn fetch_remote(host: &str, file: &str, inv: &str) -> Result<String, String> {
        let out = std::process::Command::new("ansible")
            .arg(host)
            .arg("-i")
            .arg(inv)
            .arg("-m")
            .arg("slurp")
            .arg("-a")
            .arg(format!("src={}", file))
            .output()
            .map_err(|e| e.to_string())?;
        let stdout = String::from_utf8_lossy(&out.stdout);
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&stdout) {
            if let Some(b64) = v.get("content").and_then(|c| c.as_str()) {
                use base64::{engine::general_purpose, Engine};
                if let Ok(bytes) = general_purpose::STANDARD.decode(b64) {
                    return Ok(String::from_utf8_lossy(&bytes).into_owned());
                }
            }
        }
        Err(format!(
            "Не удалось получить файл: {}",
            String::from_utf8_lossy(&out.stderr)
        ))
    }
}
