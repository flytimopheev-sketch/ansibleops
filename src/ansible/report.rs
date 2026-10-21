use crate::models::RunSession;

pub fn to_html(s: &RunSession) -> String {
    let mut rows = String::new();
    for ev in &s.events {
        let color = match ev.status {
            crate::models::TaskStatus::Ok => "#9ece6a",
            crate::models::TaskStatus::Changed => "#7aa2f7",
            crate::models::TaskStatus::Failed => "#f7768e",
            _ => "#7a84a8",
        };
        rows.push_str(&format!(
            "<tr><td style='color:{}'>{:?}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
            color,
            ev.status,
            ev.host,
            ev.task,
            ev.output.replace('<', "&lt;")
        ));
    }
    format!("<html><head><meta charset='utf-8'><title>Report {}</title><style>body{{background:#1a1b26;color:#c0caf5;font-family:monospace}}table{{border-collapse:collapse;width:100%}}td,th{{border:1px solid #24283b;padding:6px;text-align:left}}</style></head><body><h1>⚡ Отчёт: {}</h1><p>OK: {} | Changed: {} | Failed: {} | Skipped: {}</p><table><tr><th>Status</th><th>Host</th><th>Task</th><th>Output</th></tr>{}</table></body></html>",
        s.playbook, s.playbook, s.summary.ok, s.summary.changed, s.summary.failed, s.summary.skipped, rows)
}

pub fn to_json(s: &RunSession) -> String {
    serde_json::to_string_pretty(s).unwrap_or_default()
}

pub fn to_csv(s: &RunSession) -> String {
    let mut out = String::from("status;host;task;output\n");
    for ev in &s.events {
        out.push_str(&format!(
            "{:?};{};{};{}\n",
            ev.status,
            ev.host,
            ev.task,
            ev.output.replace(';', ",").replace('\n', " ")
        ));
    }
    out
}
