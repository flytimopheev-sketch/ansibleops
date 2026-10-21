use crate::ansible::editor::YamlEditor;
use crate::ansible::graph::TaskGraph;
use crate::ansible::history::HistoryStore;
use crate::ansible::runner::AnsibleRunner;
use crate::ansible::terraform::{TerraformRunner, TfEvent};
use crate::ansible::{adhoc, ops};
use crate::models::*;
use crate::theme::*;
use egui::{Align, Color32, Frame, Layout, Margin, RichText, Rounding, Stroke, Vec2};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

pub struct AnsibleApp {
    pub inventory_path: String,
    pub playbook_path: String,
    pub extra_vars: String,
    pub ssh: SshConfig,
    pub hosts: Vec<Host>,
    pub session: Arc<Mutex<RunSession>>,
    pub tab: Tab,
    pub editor: YamlEditor,
    pub graph: Option<TaskGraph>,
    pub playbook_data: Option<Playbook>,
    pub tf: TerraformConfig,
    pub history: Vec<RunSession>,
    pub history_store: Option<HistoryStore>,
    pub rx: Option<mpsc::Receiver<TaskEvent>>,
    pub tf_rx: Option<mpsc::Receiver<TfEvent>>,
    pub tf_log: Vec<TfEvent>,
    pub auto_scroll: bool,
    pub error_message: Option<String>,
    pub config_path: PathBuf,
    pub constructor_play: ConstructorPlay,
    pub show_yaml_preview: bool,
    pub admin_action: AdminAction,
    pub adhoc_module: String,
    pub adhoc_args: String,
    #[allow(dead_code)]
    pub adhoc_output: Vec<String>,
    pub background_jobs: Vec<BackgroundJob>,
    pub job_rx: Option<mpsc::Receiver<BackgroundJob>>,
    pub run_cancel: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
    pub show_about: bool,
    pub theme_idx: usize,
    pub lang: Lang,
    pub toasts: Vec<(String, Instant)>,
    pub diag_results: Vec<DiagResult>,
    pub diag_port: String,
    pub diag_rx: Option<std::sync::mpsc::Receiver<Vec<DiagResult>>>,
    pub report_format: String,
    pub vault_file: String,
    pub vault_pass: String,
    pub vault_out: String,
    pub rpm_dir: String,
    pub rpm_out: String,
    pub git_dir: String,
    pub git_out: String,
    pub git_msg: String,
    pub sched_tasks: Vec<SchedTask>,
    pub zbx_server: String,
    pub zbx_port: String,
    pub zbx_out: String,
    pub web_on: bool,
    pub web_bind: String,
    pub web_token: String,
    pub api_port_str: String,
    pub ansible_version: String,
    pub show_palette: bool,
    pub palette_query: String,
    pub theme_dirty: bool,
    pub ops: OpsState,
    pub ops_rx: Option<std::sync::mpsc::Receiver<String>>,
    pub comp_rx: Option<std::sync::mpsc::Receiver<ComplianceResult>>,
    pub diff: DiffState,
    pub diff_rx: Option<std::sync::mpsc::Receiver<String>>,
    pub versions: Option<crate::ansible::diff::VersionStore>,
    pub roles: RolesState,
    pub rule_rx: Option<std::sync::mpsc::Receiver<String>>,
    pub last_rule_check: Instant,
}

impl Default for AnsibleApp {
    fn default() -> Self {
        let config_path = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("ansibleops")
            .join("config.json");
        Self {
            inventory_path: "inventory.ini".into(),
            playbook_path: "site.yml".into(),
            extra_vars: String::new(),
            ssh: SshConfig {
                forks: 10,
                become_user: "root".into(),
                ..Default::default()
            },
            hosts: Vec::new(),
            session: Arc::new(Mutex::new(RunSession::new(String::new(), String::new()))),
            tab: Tab::Run,
            editor: YamlEditor::default(),
            graph: None,
            playbook_data: None,
            tf: TerraformConfig::default(),
            history: Vec::new(),
            history_store: HistoryStore::new().ok(),
            rx: None,
            tf_rx: None,
            tf_log: Vec::new(),
            auto_scroll: true,
            error_message: None,
            config_path,
            constructor_play: ConstructorPlay::default(),
            show_yaml_preview: false,
            admin_action: AdminAction {
                target_hosts: "all".into(),
                use_become: true,
                ..Default::default()
            },
            adhoc_module: "ping".into(),
            adhoc_args: "".into(),
            adhoc_output: Vec::new(),
            background_jobs: Vec::new(),
            job_rx: None,
            run_cancel: None,
            show_about: false,
            theme_idx: 0,
            lang: Lang::Ru,
            toasts: Vec::new(),
            diag_results: Vec::new(),
            diag_port: "80".into(),
            diag_rx: None,
            report_format: "html".into(),
            vault_file: String::new(),
            vault_pass: String::new(),
            vault_out: String::new(),
            rpm_dir: String::new(),
            rpm_out: String::new(),
            git_dir: String::new(),
            git_out: String::new(),
            git_msg: "update playbooks".into(),
            sched_tasks: Vec::new(),
            zbx_server: "127.0.0.1".into(),
            zbx_port: "10051".into(),
            zbx_out: String::new(),
            web_on: false,
            web_bind: "127.0.0.1".into(),
            web_token: crate::ansible::webapi::generate_token(),
            api_port_str: "8080".into(),
            ansible_version: String::new(),
            show_palette: false,
            palette_query: String::new(),
            theme_dirty: false,
            ops: OpsState {
                hosts: "all".into(),
                ..Default::default()
            },
            ops_rx: None,
            comp_rx: None,
            rule_rx: None,
            last_rule_check: Instant::now(),
            diff: DiffState::default(),
            diff_rx: None,
            versions: crate::ansible::diff::VersionStore::new().ok(),
            roles: RolesState::default(),
        }
    }
}

impl AnsibleApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let mut a = Self::default();
        a.load_config();
        a.refresh_history();
        a.ansible_version = std::process::Command::new("ansible")
            .arg("--version")
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .and_then(|s| s.lines().next().map(|l| l.trim().to_string()))
            .unwrap_or_else(|| "ansible: not installed".into());
        a
    }

    pub fn toast(&mut self, msg: String) {
        self.toasts.push((msg, Instant::now()));
    }

    fn cycle_theme(&mut self, ctx: &egui::Context) {
        self.theme_idx = (self.theme_idx + 1) % 4;
        apply_theme(ctx, self.theme_idx);
        self.toast(format!("🎨 Тема: {}", THEME_NAMES[self.theme_idx]));
    }

    fn load_config(&mut self) {
        if let Some(parent) = self.config_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(content) = std::fs::read_to_string(&self.config_path) {
            if let Ok(config) = serde_json::from_str::<AppConfig>(&content) {
                self.inventory_path = config.inventory_path;
                self.playbook_path = config.playbook_path;
                self.extra_vars = config.extra_vars;
                self.ssh = config.ssh;
                self.tab = config.last_tab;
                self.theme_idx = config.theme;
                self.theme_dirty = true;
            }
        }
    }

    fn save_config(&self) {
        let config = AppConfig {
            inventory_path: self.inventory_path.clone(),
            playbook_path: self.playbook_path.clone(),
            extra_vars: self.extra_vars.clone(),
            ssh: self.ssh.clone(),
            last_tab: self.tab,
            theme: self.theme_idx,
        };
        if let Ok(json) = serde_json::to_string_pretty(&config) {
            let _ = std::fs::write(&self.config_path, json);
        }
    }

    pub fn refresh_history(&mut self) {
        if let Some(s) = &self.history_store {
            if let Ok(h) = s.list() {
                self.history = h;
            }
        }
    }

    pub fn start_run(&mut self) {
        self.error_message = None;
        if let Ok(mut s) = self.session.lock() {
            *s = RunSession::new(self.playbook_path.clone(), self.inventory_path.clone());
            s.state = RunState::Running;
            s.started_at = Some(chrono::Local::now());
        }
        let (tx, rx) = mpsc::channel(256);
        self.rx = Some(rx);
        let pb = self.playbook_path.clone();
        let inv = self.inventory_path.clone();
        let vars = if self.extra_vars.is_empty() {
            None
        } else {
            Some(self.extra_vars.clone())
        };
        let ssh = self.ssh.clone();
        let session = self.session.clone();
        let store = self.history_store.clone();
        let cancel = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        self.run_cancel = Some(cancel.clone());
        // Собираем секретные значения из extra vars для маскирования в логах
        let secrets = Self::extract_secrets(&vars.clone().unwrap_or_default());
        tokio::spawn(async move {
            let ok = AnsibleRunner::run_cancellable(
                &pb,
                &inv,
                vars.as_deref(),
                &ssh,
                tx,
                cancel,
                secrets,
            )
            .await
            .unwrap_or(false);
            if let Ok(mut s) = session.lock() {
                s.state = RunState::Finished { success: ok };
                s.finished_at = Some(chrono::Local::now());
                s.recompute();
                if let Some(st) = &store {
                    let _ = st.save(&s);
                    st.audit(&format!("run_finish id={} playbook={} inventory={} success={} ok/changed/failed={}/{}/{}", s.id, s.playbook, s.inventory, ok, s.summary.ok, s.summary.changed, s.summary.failed));
                }
            }
        });
    }

    /// Остановка текущего запуска: прерывание процесса ansible-playbook.
    pub fn stop_run(&mut self) {
        if let Some(c) = &self.run_cancel {
            c.store(true, std::sync::atomic::Ordering::Relaxed);
        }
        let mut msg: Option<String> = None;
        if let Ok(mut s) = self.session.lock() {
            if matches!(s.state, RunState::Running) {
                let done = s.summary.ok + s.summary.changed + s.summary.failed + s.summary.skipped;
                msg = Some(format!(
                    "⏹ Остановка... Завершено событий: {}. Состояние: прервано пользователем",
                    done
                ));
                s.events.push(crate::models::TaskEvent {
                    task: "Остановка".into(),
                    host: "all".into(),
                    status: crate::models::TaskStatus::Failed,
                    duration_ms: 0,
                    output: "Прервано пользователем".into(),
                    timestamp: chrono::Local::now(),
                });
            }
        }
        if let Some(m) = msg {
            self.toast(m);
        }
    }

    /// Извлечение значений секретных переменных из extra vars (k=v, "k": "v", k: v).
    fn extract_secrets(extra_vars: &str) -> Vec<String> {
        let mut out = Vec::new();
        for line in extra_vars.lines() {
            let (k, v) = if let Some((k, v)) = line.split_once('=') {
                (k, v)
            } else if let Some((k, v)) = line.split_once(':') {
                (k, v)
            } else {
                continue;
            };
            let k = k.trim().trim_matches('"').trim_matches('\'').to_lowercase();
            let v = v
                .trim()
                .trim_matches(',')
                .trim_matches('"')
                .trim_matches('\'');
            if (k.contains("pass")
                || k.contains("secret")
                || k.contains("token")
                || k.contains("apikey")
                || k.contains("api_key")
                || k.contains("private"))
                && !v.is_empty()
            {
                out.push(v.to_string());
            }
        }
        out
    }

    pub fn start_tf(&mut self, action: &str) {
        let (tx, rx) = mpsc::channel(256);
        self.tf_rx = Some(rx);
        self.tf_log.clear();
        let cfg = self.tf.clone();
        let action = action.to_string();
        tokio::spawn(async move {
            let _ = TerraformRunner::run_cmd(&cfg, &action, tx).await;
        });
    }

    pub fn load_playbook_graph(&mut self) {
        self.error_message = None;
        let path = std::path::Path::new(&self.playbook_path);
        if let Err(e) = self.editor.load(path) {
            self.error_message = Some(format!("Не удалось открыть файл: {}", e));
            return;
        }
        if let Ok(content) = std::fs::read_to_string(path) {
            if let Ok(raw_data) = serde_yaml::from_str::<Vec<serde_yaml::Value>>(&content) {
                let mut pb = Playbook {
                    name: path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into(),
                    path: path.to_path_buf(),
                    raw_data: raw_data.clone(),
                    tasks: Vec::new(),
                };
                for play in &raw_data {
                    if let Some(tasks) = play.get("tasks").and_then(|t| t.as_sequence()) {
                        for t in tasks {
                            let name = t
                                .get("name")
                                .and_then(|n| n.as_str())
                                .unwrap_or("unnamed")
                                .into();
                            let module = t
                                .as_mapping()
                                .and_then(|m| {
                                    m.keys()
                                        .find(|k| {
                                            let s = k.as_str().unwrap_or("");
                                            s != "name"
                                                && s != "when"
                                                && s != "register"
                                                && s != "notify"
                                        })
                                        .and_then(|k| k.as_str())
                                })
                                .unwrap_or("")
                                .into();
                            pb.tasks.push(TaskDef {
                                name,
                                module,
                                depends_on: Vec::new(),
                            });
                        }
                    }
                }
                self.playbook_data = Some(pb.clone());
                self.graph = Some(TaskGraph::from_playbook(&pb));
            }
        }
    }

    pub fn save_playbook_order(&mut self) {
        if let Some(pb) = &self.playbook_data {
            let mut new_raw_data = pb.raw_data.clone();
            let mut task_idx = 0;
            for play in &mut new_raw_data {
                if let Some(tasks) = play.get_mut("tasks") {
                    if let Some(tasks_seq) = tasks.as_sequence_mut() {
                        let mut new_tasks_seq = Vec::new();
                        for _ in 0..tasks_seq.len() {
                            if task_idx < pb.tasks.len() {
                                let task_name = &pb.tasks[task_idx].name;
                                if let Some(orig_task) = tasks_seq.iter().find(|t| {
                                    t.get("name").and_then(|n| n.as_str()) == Some(task_name)
                                }) {
                                    new_tasks_seq.push(orig_task.clone());
                                }
                                task_idx += 1;
                            }
                        }
                        *tasks_seq = new_tasks_seq;
                    }
                }
            }
            if let Ok(yaml) = serde_yaml::to_string(&new_raw_data) {
                if let Some(path) = &self.editor.path {
                    let path_clone = path.clone();
                    if let Err(e) = std::fs::write(&path_clone, yaml) {
                        self.error_message = Some(format!("Ошибка сохранения: {}", e));
                    } else {
                        self.error_message = None;
                        if let Some(g) = &mut self.graph {
                            g.order_changed = false;
                        }
                        let _ = self.editor.load(&path_clone);
                    }
                }
            }
        }
    }

    pub fn constructor_to_yaml(&self) -> String {
        let mut yaml = String::new();
        yaml.push_str("---\n");
        yaml.push_str(&format!("- name: {}\n", self.constructor_play.name));
        yaml.push_str(&format!("  hosts: {}\n", self.constructor_play.hosts));
        if self.constructor_play.use_become {
            yaml.push_str("  become: yes\n");
        }
        if !self.constructor_play.gather_facts {
            yaml.push_str("  gather_facts: no\n");
        }
        yaml.push_str("  tasks:\n");
        for task in &self.constructor_play.tasks {
            yaml.push_str(&format!("    - name: {}\n", task.name));
            yaml.push_str(&format!("      {}:\n", task.module));
            for (key, value) in &task.params {
                if !value.is_empty() {
                    yaml.push_str(&format!("        {}: {}\n", key, value));
                }
            }
            if task.use_become {
                yaml.push_str("      become: yes\n");
            }
            if task.ignore_errors {
                yaml.push_str("      ignore_errors: yes\n");
            }
            if !task.when_condition.is_empty() {
                yaml.push_str(&format!("      when: {}\n", task.when_condition));
            }
            if !task.register_var.is_empty() {
                yaml.push_str(&format!("      register: {}\n", task.register_var));
            }
            if !task.notify_handler.is_empty() {
                yaml.push_str(&format!("      notify: {}\n", task.notify_handler));
            }
        }
        yaml
    }

    pub fn save_constructor_to_file(&mut self) {
        let yaml = self.constructor_to_yaml();
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("YAML Files", &["yml", "yaml"])
            .save_file()
        {
            if let Err(e) = std::fs::write(&path, yaml) {
                self.error_message = Some(format!("Ошибка сохранения: {}", e));
            } else {
                self.error_message = None;
                let _ = self.editor.load(&path);
                self.playbook_path = path.to_string_lossy().into_owned();
            }
        }
    }

    fn commands(&self) -> Vec<(String, String)> {
        vec![
            ("tab:Run".into(), "▶ Перейти: Run".into()),
            ("tab:Editor".into(), "📝 Перейти: Editor".into()),
            ("tab:Constructor".into(), "🏗 Перейти: Constructor".into()),
            ("tab:Admin".into(), "🛠 Перейти: Admin".into()),
            ("tab:AdHoc".into(), "💻 Перейти: Ad-Hoc".into()),
            ("tab:Library".into(), "📚 Перейти: Library".into()),
            ("tab:Roles".into(), "📦 Перейти: Roles".into()),
            ("tab:Diff".into(), "🔍 Перейти: Diff".into()),
            ("tab:Graph".into(), "🕸 Перейти: Graph".into()),
            ("tab:Terraform".into(), "🌍 Перейти: Terraform".into()),
            ("tab:Tools".into(), "🧰 Перейти: Tools".into()),
            ("tab:History".into(), "🗂 Перейти: History".into()),
            ("tab:Domain".into(), "🖥 Перейти: Domain".into()),
            ("tab:Profiles".into(), "🧾 Перейти: Profiles".into()),
            ("tab:Workflow".into(), "🔀 Перейти: Workflow".into()),
            ("tab:Ops".into(), "⚡ Перейти: OpsCenter".into()),
            ("tab:Compliance".into(), "🛡 Перейти: Compliance".into()),
            ("act:run".into(), "🚀 Запустить playbook".into()),
            ("act:theme".into(), "🎨 Сменить тему".into()),
            ("act:clear".into(), "🧹 Очистить лог".into()),
            ("act:save".into(), "💾 Сохранить editor".into()),
            ("act:about".into(), "ℹ️ О программе".into()),
        ]
    }

    fn run_command(&mut self, id: &str) {
        match id {
            "tab:Run" => self.tab = Tab::Run,
            "tab:Editor" => self.tab = Tab::Editor,
            "tab:Constructor" => self.tab = Tab::Constructor,
            "tab:Admin" => self.tab = Tab::Admin,
            "tab:AdHoc" => self.tab = Tab::AdHoc,
            "tab:Library" => self.tab = Tab::Library,
            "tab:Roles" => self.tab = Tab::Roles,
            "tab:Diff" => self.tab = Tab::DiffView,
            "tab:Graph" => self.tab = Tab::Graph,
            "tab:Terraform" => self.tab = Tab::Terraform,
            "tab:Tools" => self.tab = Tab::Tools,
            "tab:History" => self.tab = Tab::History,
            "tab:Domain" => self.tab = Tab::Domain,
            "tab:Profiles" => self.tab = Tab::Profiles,
            "tab:Workflow" => self.tab = Tab::Workflow,
            "tab:Ops" => self.tab = Tab::OpsCenter,
            "tab:Compliance" => self.tab = Tab::Compliance,
            "act:run" => {
                self.start_run();
                self.toast("🚀 Запуск".into());
            }
            "act:theme" => {
                self.theme_idx = (self.theme_idx + 1) % 4;
                self.theme_dirty = true;
            }
            "act:clear" => {
                if let Ok(mut s) = self.session.lock() {
                    s.events.clear();
                    s.recompute();
                }
            }
            "act:save" => {
                let _ = self.editor.save();
                self.toast("💾 Сохранено".into());
            }
            "act:about" => self.show_about = true,
            _ => {}
        }
    }
}

impl eframe::App for AnsibleApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Some(rx) = &mut self.rx {
            while let Ok(ev) = rx.try_recv() {
                if let Ok(mut s) = self.session.lock() {
                    s.events.push(ev);
                }
            }
        }
        if let Some(rx) = &mut self.tf_rx {
            while let Ok(ev) = rx.try_recv() {
                self.tf_log.push(ev);
            }
        }
        if let Some(rx) = &mut self.job_rx {
            while let Ok(job) = rx.try_recv() {
                if let Some(existing) = self.background_jobs.iter_mut().find(|j| j.id == job.id) {
                    *existing = job;
                } else {
                    self.background_jobs.push(job);
                }
            }
        }
        if let Some(rx) = &mut self.comp_rx {
            while let Ok(r) = rx.try_recv() {
                self.ops.comp_results.push(r);
            }
        }
        if let Some(rx) = &mut self.diff_rx {
            let mut buf = String::new();
            while let Ok(line) = rx.try_recv() {
                buf.push_str(&line);
                buf.push('\n');
            }
            if !buf.is_empty() {
                self.diff.scan_running = false;
                self.diff.changes = crate::ansible::diff::parse_check_output(&buf);
                self.toast(format!("🔍 Найдено изменений: {}", self.diff.changes.len()));
            }
        }
        // Event-правила: каждые 60 сек
        if self.last_rule_check.elapsed() > Duration::from_secs(60) {
            self.last_rule_check = Instant::now();
            let active: Vec<EventRule> = self
                .ops
                .rules
                .iter()
                .filter(|r| r.enabled)
                .cloned()
                .collect();
            if !active.is_empty() {
                let hosts = self.ops.hosts.clone();
                let (tx, rx) = std::sync::mpsc::channel();
                self.rule_rx = Some(rx);
                let pending: Vec<EventRule> = {
                    let mut out = Vec::new();
                    for r in &active {
                        // Cooldown и лимит запусков
                        if let Some(t) = r.last_run {
                            if t.elapsed() < Duration::from_secs(r.cooldown_secs.max(1)) {
                                continue;
                            }
                        }
                        if r.runs >= r.max_runs {
                            continue;
                        }
                        // Ограничение по хостам: пустое = hosts из OpsCenter
                        let target = if r.allowed_hosts.is_empty() {
                            hosts.clone()
                        } else {
                            r.allowed_hosts.clone()
                        };
                        if let Some(msg) = ops::check_rule(&r.kind, &r.param, &target) {
                            let _ = tx.send(format!("⚠️ [{}] {}", r.kind, msg));
                            // Произвольный shell в автореакции запрещён без подтверждения
                            if r.action.is_empty() {
                                continue;
                            }
                            if r.require_confirm
                                || r.action.trim_start().starts_with("rm ")
                                || r.action.contains("reboot")
                                || r.action.contains("shutdown")
                                || r.action.contains("mkfs")
                                || r.action.contains("dd ")
                            {
                                out.push(r.clone());
                            } else {
                                if let Some(rule) = self
                                    .ops
                                    .rules
                                    .iter_mut()
                                    .find(|x| x.kind == r.kind && x.param == r.param)
                                {
                                    rule.runs += 1;
                                    rule.last_run = Some(Instant::now());
                                }
                                let act = r.action.clone();
                                let tx2 = tx.clone();
                                std::thread::spawn(move || {
                                    adhoc::run_raw(&target, "shell", &act, true, tx2);
                                });
                            }
                        }
                    }
                    out
                };
                if !pending.is_empty() {
                    for r in pending {
                        let desc = format!(
                            "[{}] {}",
                            r.kind,
                            if r.action.contains("reboot")
                                || r.action.contains("shutdown")
                                || r.action.contains("mkfs")
                            {
                                "ОПАСНО: "
                            } else {
                                ""
                            }
                        );
                        self.ops.rule_log.push(format!(
                            "⏸ Правило {} требует подтверждения действия: {}",
                            desc, r.action
                        ));
                        self.ops.pending_rules.push(r);
                    }
                }
            }
        }
        let mut diag_done = false;
        if let Some(rx) = &mut self.diag_rx {
            while let Ok(v) = rx.try_recv() {
                self.diag_results = v;
                diag_done = true;
            }
        }
        if diag_done {
            self.toast("🩺 Диагностика завершена".into());
        }
        ctx.request_repaint_after(Duration::from_millis(100));

        if self.theme_dirty {
            apply_theme(ctx, self.theme_idx);
            self.theme_dirty = false;
        }

        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::R))
            && self.tab == Tab::Run
        {
            self.start_run();
            self.toast("🚀 Запуск (Ctrl+R)".into());
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::S))
            && self.tab == Tab::Editor
        {
            let _ = self.editor.save();
            self.toast("💾 Сохранено (Ctrl+S)".into());
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::P)) {
            self.show_palette = !self.show_palette;
            self.palette_query.clear();
        }

        let mut due_tasks: Vec<(String, String)> = Vec::new();
        for t in self.sched_tasks.iter_mut() {
            let due = match t.last_run {
                Some(l) => l.elapsed() > Duration::from_secs(t.interval_min * 60),
                None => true,
            };
            if due {
                t.last_run = Some(Instant::now());
                due_tasks.push((t.playbook.clone(), t.inventory.clone()));
            }
        }
        for (pb, inv) in due_tasks {
            self.playbook_path = pb.clone();
            self.inventory_path = inv;
            self.start_run();
            self.toast(format!("⏰ Планировщик: {}", pb));
        }

        self.toasts
            .retain(|(_, t)| t.elapsed() < Duration::from_secs(4));
        if ctx.input(|i| i.viewport().close_requested()) {
            self.save_config();
        }

        egui::TopBottomPanel::top("topbar")
            .exact_height(56.0)
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    let (rect, _) =
                        ui.allocate_exact_size(Vec2::new(30.0, 30.0), egui::Sense::hover());
                    ui.painter().rect_filled(
                        rect,
                        Rounding::same(8.0),
                        c_blue().gamma_multiply(0.25),
                    );
                    ui.painter().rect_stroke(
                        rect,
                        Rounding::same(8.0),
                        Stroke::new(1.5_f32, c_blue()),
                    );
                    ui.painter().text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "⚡",
                        egui::FontId::proportional(16.0),
                        c_blue(),
                    );
                    ui.add_space(6.0);
                    ui.label(RichText::new(APP_NAME).size(20.0).color(c_tx()).strong());
                    ui.label(
                        RichText::new(format!("v{}", APP_VERSION))
                            .color(c_tm())
                            .size(12.0),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.small_button("ℹ️ О программе").clicked() {
                            self.show_about = true;
                        }
                        if ui.small_button("🔄").clicked() {
                            let url = format!("{}/releases/latest/download/VERSION", APP_REPO);
                            match ureq::get(&url).timeout(Duration::from_secs(5)).call() {
                                Ok(r) => {
                                    let latest =
                                        r.into_string().unwrap_or_default().trim().to_string();
                                    if latest.is_empty() {
                                        self.toast("🔄 Сервис обновлений: пустой ответ".into());
                                    } else if latest == APP_VERSION {
                                        self.toast(format!(
                                            "🔄 Установлена актуальная версия ({})",
                                            latest
                                        ));
                                    } else {
                                        self.toast(format!("🔄 Доступна новая версия: {}", latest));
                                    }
                                }
                                Err(_) => self.toast("🔄 Сервис обновлений недоступен".into()),
                            }
                        }
                        if ui
                            .small_button(format!("🎨 {}", THEME_NAMES[self.theme_idx]))
                            .clicked()
                        {
                            self.cycle_theme(ctx);
                        }
                        if ui
                            .small_button(if self.lang == Lang::Ru { "RU" } else { "EN" })
                            .clicked()
                        {
                            self.lang = if self.lang == Lang::Ru {
                                Lang::En
                            } else {
                                Lang::Ru
                            };
                        }
                        ui.add_space(8.0);
                        let s = self.session.lock().unwrap();
                        match &s.state {
                            RunState::Idle => ui.colored_label(c_tm(), "● Idle"),
                            RunState::Running => ui.colored_label(c_orange(), "● Running"),
                            RunState::Finished { success } => ui.colored_label(
                                if *success { c_green() } else { c_red() },
                                if *success {
                                    "● Success"
                                } else {
                                    "● Failed"
                                },
                            ),
                        };
                    });
                });
            });

        egui::SidePanel::left("nav_rail")
            .exact_width(56.0)
            .frame(Frame::none().fill(c_bgd()).inner_margin(Margin::same(6.0)))
            .show(ctx, |ui| {
                ui.add_space(6.0);
                ui.vertical_centered(|ui| {
                    let nav: Vec<(Tab, &str, &str)> = vec![
                        (Tab::Run, "▶", "Run"),
                        (Tab::Editor, "📝", "Editor"),
                        (Tab::Constructor, "🏗", "Constructor"),
                        (Tab::Admin, "🛠", "Admin"),
                        (Tab::AdHoc, "💻", "Ad-Hoc"),
                        (Tab::Library, "📚", "Library"),
                        (Tab::Roles, "📦", "Roles"),
                        (Tab::DiffView, "🔍", "Diff"),
                        (Tab::Graph, "🕸", "Graph"),
                        (Tab::Terraform, "🌍", "Terraform"),
                        (Tab::Tools, "🧰", "Tools"),
                        (Tab::History, "🗂", "History"),
                        (Tab::Domain, "🖥", "Domain"),
                        (Tab::Profiles, "🧾", "Profiles"),
                        (Tab::Workflow, "🔀", "Workflow"),
                        (Tab::OpsCenter, "⚡", "OpsCenter"),
                        (Tab::Compliance, "🛡", "Compliance"),
                    ];
                    for (t, icon, label) in nav {
                        let sel = self.tab == t;
                        let btn = ui.add_sized(
                            [44.0, 44.0],
                            egui::Button::new(RichText::new(icon).size(18.0))
                                .fill(if sel { c_surfh() } else { Color32::TRANSPARENT })
                                .rounding(Rounding::same(10.0)),
                        );
                        if btn.clicked() {
                            self.tab = t;
                        }
                        btn.on_hover_text(label);
                        ui.add_space(2.0);
                    }
                });
            });

        egui::TopBottomPanel::bottom("statusbar")
            .exact_height(26.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.colored_label(c_green(), "●");
                    ui.label(
                        RichText::new(&self.ansible_version)
                            .color(c_tm())
                            .size(12.0),
                    );
                    ui.separator();
                    ui.label(
                        RichText::new(format!("Hosts: {}", self.hosts.len()))
                            .color(c_tm())
                            .size(12.0),
                    );
                    ui.separator();
                    ui.label(
                        RichText::new(format!("Jobs: {}", self.background_jobs.len()))
                            .color(c_tm())
                            .size(12.0),
                    );
                    ui.separator();
                    ui.label(RichText::new("Ctrl+P — палитра").color(c_tm()).size(12.0));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(
                            RichText::new(format!("{} v{}", APP_NAME, APP_VERSION))
                                .color(c_tm())
                                .size(12.0),
                        );
                    });
                });
            });

        if self.show_about {
            let mut close_clicked = false;
            egui::Window::new(
                RichText::new(format!("ℹ️ О программе {}", APP_NAME)).color(c_blue()),
            )
            .open(&mut self.show_about)
            .default_size(Vec2::new(520.0, 480.0))
            .show(ctx, |ui| {
                ui.label(RichText::new(APP_NAME).size(22.0).strong().color(c_tx()));
                ui.label(RichText::new(format!("Версия {}", APP_VERSION)).color(c_tm()));
                ui.add_space(8.0);
                ui.separator();
                ui.add_space(6.0);
                ui.label(RichText::new("🦖 Написано на:").strong().color(c_purple()));
                ui.label("• Rust, egui/eframe 0.29, tokio, serde, syntect, petgraph, rfd, ureq");
                ui.add_space(6.0);
                ui.label(RichText::new("🔧 Бэкенд:").strong().color(c_purple()));
                ui.label("• Ansible Core, Terraform, Git, createrepo, Zabbix sender");
                ui.add_space(6.0);
                ui.label(RichText::new("👤 Автор:").strong().color(c_purple()));
                ui.label(format!("• Почта: {}", AUTHOR_EMAIL));
                ui.add_space(4.0);
                ui.label(RichText::new("🔗 Проект:").strong().color(c_purple()));
                ui.hyperlink_to(APP_REPO, APP_REPO);
                ui.add_space(4.0);
                ui.label(RichText::new("📜 Лицензия: MIT").color(c_tm()));
                ui.add_space(10.0);
                if ui.button("Закрыть").clicked() {
                    close_clicked = true;
                }
            });
            if close_clicked {
                self.show_about = false;
            }
        }

        if self.show_palette {
            let mut action: Option<String> = None;
            let mut open = self.show_palette;
            let mut query = self.palette_query.clone();
            let cmds = self.commands();
            egui::Window::new("⌘ Command Palette")
                .anchor(egui::Align2::CENTER_TOP, [0.0, 90.0])
                .default_size([440.0, 320.0])
                .open(&mut open)
                .show(ctx, |ui| {
                    ui.text_edit_singleline(&mut query);
                    ui.add_space(6.0);
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        for (id, label) in &cmds {
                            let q = query.to_lowercase();
                            if !q.is_empty() && !label.to_lowercase().contains(&q) {
                                continue;
                            }
                            if ui.selectable_label(false, label).clicked() {
                                action = Some(id.clone());
                            }
                        }
                    });
                });
            self.palette_query = query;
            self.show_palette = open;
            if let Some(id) = action {
                self.run_command(&id);
                self.show_palette = false;
            }
        }

        match self.tab {
            Tab::Run => self.show_run(ctx),
            Tab::Editor => self.show_editor(ctx),
            Tab::Constructor => self.show_constructor(ctx),
            Tab::Admin => self.show_admin(ctx),
            Tab::AdHoc => self.show_adhoc(ctx),
            Tab::Library => self.show_library(ctx),
            Tab::Roles => self.show_roles(ctx),
            Tab::DiffView => self.show_diff(ctx),
            Tab::Graph => self.show_graph(ctx),
            Tab::Terraform => self.show_terraform(ctx),
            Tab::Tools => self.show_tools(ctx),
            Tab::History => self.show_history(ctx),
            Tab::Domain => self.show_domain(ctx),
            Tab::Profiles => self.show_profiles(ctx),
            Tab::Workflow => self.show_workflow(ctx),
            Tab::OpsCenter => self.show_opscenter(ctx),
            Tab::Compliance => self.show_compliance(ctx),
        }

        egui::Area::new(egui::Id::new("toasts"))
            .fixed_pos(egui::pos2(60.0, 90.0))
            .show(ctx, |ui| {
                for (msg, t) in &self.toasts {
                    let alpha = 1.0 - (t.elapsed().as_secs_f32() / 4.0);
                    Frame::none()
                        .fill(c_blue().gamma_multiply(0.25 * alpha))
                        .rounding(Rounding::same(8.0))
                        .inner_margin(Margin::same(10.0))
                        .show(ui, |ui| {
                            ui.colored_label(c_blue().gamma_multiply(alpha), msg);
                        });
                    ui.add_space(4.0);
                }
            });
    }
}
