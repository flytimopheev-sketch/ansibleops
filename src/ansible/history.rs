use crate::models::RunSession;
use anyhow::Result;
use std::fs;
use std::path::PathBuf;

#[derive(Clone)]
pub struct HistoryStore {
    pub dir: PathBuf,
}

impl HistoryStore {
    pub fn new() -> Result<Self> {
        let dir = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("ansibleops")
            .join("history");
        fs::create_dir_all(&dir)?;
        Ok(Self { dir })
    }
    pub fn save(&self, s: &RunSession) -> Result<()> {
        let p = self.dir.join(format!("{}.json", s.id));
        fs::write(p, serde_json::to_string_pretty(s)?)?;
        Ok(())
    }
    pub fn list(&self) -> Result<Vec<RunSession>> {
        let mut v = Vec::new();
        if !self.dir.exists() {
            return Ok(v);
        }
        for e in fs::read_dir(&self.dir)? {
            let p = e?.path();
            if p.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Ok(s) = fs::read_to_string(&p)
                    .and_then(|c| serde_json::from_str(&c).map_err(std::io::Error::other))
                {
                    v.push(s);
                }
            }
        }
        v.sort_by_key(|b| std::cmp::Reverse(b.started_at));
        Ok(v)
    }
    /// Архивация вместо физического удаления: файл переименовывается в *.archived,
    /// в журнал аудита пишется запись. Данные не теряются (retention policy).
    pub fn delete(&self, id: &str) -> Result<()> {
        let path = self.dir.join(format!("{}.json", id));
        if path.exists() {
            let archived = self.dir.join(format!("{}.json.archived", id));
            fs::rename(&path, &archived)?;
            self.audit(&format!(
                "history_delete id={} -> archived as {}",
                id,
                archived
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default()
            ));
        }
        Ok(())
    }

    /// Запись в append-only журнал аудита (audit log).
    pub fn audit(&self, event: &str) {
        let p = self.dir.join("audit.log");
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&p)
        {
            use std::io::Write;
            let ts = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
            let _ = writeln!(f, "{} | {}", ts, event);
        }
    }

    /// Количество заархивированных записей.
    #[allow(dead_code)]
    pub fn archived_count(&self) -> usize {
        self.dir
            .read_dir()
            .map(|rd| {
                rd.flatten()
                    .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("archived"))
                    .count()
            })
            .unwrap_or(0)
    }
}
