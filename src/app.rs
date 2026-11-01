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
    pub stop_confirm: bool,
    pub ui: UiState,
    pub web: WebConfig,
    pub zabbix: ZabbixConfig,
    pub rpm_git: RpmGitConfig,
    pub scheduler_compliance: SchedulerComplianceState,
    pub diff_versions: DiffVersionsState,
    pub vault_file: String,
    pub vault_pass: String,
    pub vault_out: String,
    pub ops: OpsState,
    pub ops_rx: Option<std::sync::mpsc::Receiver<String>>,
    pub report_format: String,
    pub diag_results: Vec<DiagResult>,
    pub diag_port: String,
    pub diag_rx: Option<std::sync::mpsc::Receiver<Vec<DiagResult>>>,
    pub roles: RolesState,
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
            stop_confirm: false,
            ui: UiState {
                auto_scroll: true,
                lang: Lang::Ru,
                ..Default::default()
            },
            web: WebConfig {
                enabled: false,
                bind: "127.0.0.1".into(),
                token: crate::ansible::webapi::generate_token(),
                api_port: "8080".into(),
                ..Default::default()
            },
            zabbix: ZabbixConfig {
                server: "127.0.0.1".into(),
                port: "10051".into(),
                ..Default::default()
            },
            rpm_git: RpmGitConfig {
                git_msg: "update playbooks".into(),
                ..Default::default()
            },
            scheduler_compliance: SchedulerComplianceState {
                last_rule_check: Instant::now(),
                ..Default::default()
            },
            diff_versions: DiffVersionsState {
                diff: DiffState::default(),
                versions: crate::ansible::diff::VersionStore::new().ok(),
                ..Default::default()
            },
            vault_file: String::new(),
            vault_pass: String::new(),
            vault_out: String::new(),
            ops: OpsState {
                hosts: "all".into(),
                ..Default::default()
            },
            ops_rx: None,
            report_format: "html".into(),
            diag_results: Vec::new(),
            diag_port: "80".into(),
            diag_rx: None,
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
        self.ui.toasts.push((msg, Instant::now()));
    }

    fn cycle_theme(&mut self, ctx: &egui::Context) {
        self.ui.theme_idx = (self.ui.theme_idx + 1) % 4;
        apply_theme(ctx, self.ui.theme_idx);
        self.toast(format!("рџЋЁ РўРµРјР°: {}", THEME_NAMES[self.ui.theme_idx]));
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
                self.ui.theme_idx = config.theme;
                self.ui.theme_dirty = true;
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
            theme: self.ui.theme_idx,
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
        self.ui.error_message = None;
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
        // РЎРѕР±РёСЂР°РµРј СЃРµРєСЂРµС‚РЅС‹Рµ Р·РЅР°С‡РµРЅРёСЏ РёР· extra vars РґР»СЏ РјР°СЃРєРёСЂРѕРІР°РЅРёСЏ РІ Р»РѕРіР°С…
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

    /// РћСЃС‚Р°РЅРѕРІРєР° С‚РµРєСѓС‰РµРіРѕ Р·Р°РїСѓСЃРєР°: РїСЂРµСЂС‹РІР°РЅРёРµ РїСЂРѕС†РµСЃСЃР° ansible-playbook.
    pub fn stop_run(&mut self) {
        if let Some(c) = &self.run_cancel {
            c.store(true, std::sync::atomic::Ordering::Relaxed);
        }
        let mut msg: Option<String> = None;
        if let Ok(mut s) = self.session.lock() {
            if matches!(s.state, RunState::Running) {
                let done = s.summary.ok + s.summary.changed + s.summary.failed + s.summary.skipped;
                msg = Some(format!(
                    "вЏ№ РћСЃС‚Р°РЅРѕРІРєР°... Р—Р°РІРµСЂС€РµРЅРѕ СЃРѕР±С‹С‚РёР№: {}. РЎРѕСЃС‚РѕСЏРЅРёРµ: РїСЂРµСЂРІР°РЅРѕ РїРѕР»СЊР·РѕРІР°С‚РµР»РµРј",
                    done
                ));
                s.events.push(crate::models::TaskEvent {
                    task: "РћСЃС‚Р°РЅРѕРІРєР°".into(),
                    host: "all".into(),
                    status: crate::models::TaskStatus::Failed,
                    duration_ms: 0,
                    output: "РџСЂРµСЂРІР°РЅРѕ РїРѕР»СЊР·РѕРІР°С‚РµР»РµРј".into(),
                    timestamp: chrono::Local::now(),
                });
            }
        }
        if let Some(m) = msg {
            self.toast(m);
        }
    }

    /// РР·РІР»РµС‡РµРЅРёРµ Р·РЅР°С‡РµРЅРёР№ СЃРµРєСЂРµС‚РЅС‹С… РїРµСЂРµРјРµРЅРЅС‹С… РёР· extra vars (k=v, "k": "v", k: v).
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
        self.ui.error_message = None;
        let path = std::path::Path::new(&self.playbook_path);
        if let Err(e) = self.editor.load(path) {
            self.ui.error_message = Some(format!("РќРµ СѓРґР°Р»РѕСЃСЊ РѕС‚РєСЂС‹С‚СЊ С„Р°Р№Р»: {}", e));
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
                        self.ui.error_message = Some(format!("РћС€РёР±РєР° СЃРѕС…СЂР°РЅРµРЅРёСЏ: {}", e));
                    } else {
                        self.ui.error_message = None;
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
                self.ui.error_message = Some(format!("РћС€РёР±РєР° СЃРѕС…СЂР°РЅРµРЅРёСЏ: {}", e));
            } else {
                self.ui.error_message = None;
                let _ = self.editor.load(&path);
                self.playbook_path = path.to_string_lossy().into_owned();
            }
        }
    }

    fn commands(&self) -> Vec<(String, String)> {
        vec![
            ("tab:Run".into(), "в–¶ РџРµСЂРµР№С‚Рё: Run".into()),
            ("tab:Editor".into(), "рџ“ќ РџРµСЂРµР№С‚Рё: Editor".into()),
            ("tab:Constructor".into(), "рџЏ— РџРµСЂРµР№С‚Рё: Constructor".into()),
            ("tab:Admin".into(), "рџ›  РџРµСЂРµР№С‚Рё: Admin".into()),
            ("tab:AdHoc".into(), "рџ’» РџРµСЂРµР№С‚Рё: Ad-Hoc".into()),
            ("tab:Library".into(), "рџ“љ РџРµСЂРµР№С‚Рё: Library".into()),
            ("tab:Roles".into(), "рџ“¦ РџРµСЂРµР№С‚Рё: Roles".into()),
            ("tab:Diff".into(), "рџ”Ќ РџРµСЂРµР№С‚Рё: Diff".into()),
            ("tab:Graph".into(), "рџ•ё РџРµСЂРµР№С‚Рё: Graph".into()),
            ("tab:Terraform".into(), "рџЊЌ РџРµСЂРµР№С‚Рё: Terraform".into()),
            ("tab:Tools".into(), "рџ§° РџРµСЂРµР№С‚Рё: Tools".into()),
            ("tab:History".into(), "рџ—‚ РџРµСЂРµР№С‚Рё: History".into()),
            ("tab:Domain".into(), "рџ–Ґ РџРµСЂРµР№С‚Рё: Domain".into()),
            ("tab:Profiles".into(), "рџ§ѕ РџРµСЂРµР№С‚Рё: Profiles".into()),
            ("tab:Workflow".into(), "рџ”Ђ РџРµСЂРµР№С‚Рё: Workflow".into()),
            ("tab:Ops".into(), "вљЎ РџРµСЂРµР№С‚Рё: OpsCenter".into()),
            ("tab:Compliance".into(), "рџ›Ў РџРµСЂРµР№С‚Рё: Compliance".into()),
            ("act:run".into(), "рџљЂ Р—Р°РїСѓСЃС‚РёС‚СЊ playbook".into()),
            ("act:theme".into(), "рџЋЁ РЎРјРµРЅРёС‚СЊ С‚РµРјСѓ".into()),
            ("act:clear".into(), "рџ§№ РћС‡РёСЃС‚РёС‚СЊ Р»РѕРі".into()),
            ("act:save".into(), "рџ’ѕ РЎРѕС…СЂР°РЅРёС‚СЊ editor".into()),
            ("act:about".into(), "в„№пёЏ Рћ РїСЂРѕРіСЂР°РјРјРµ".into()),
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
                self.toast("рџљЂ Р—Р°РїСѓСЃРє".into());
            }
            "act:theme" => {
                self.ui.theme_idx = (self.ui.theme_idx + 1) % 4;
                self.ui.theme_dirty = true;
            }
            "act:clear" => {
                if let Ok(mut s) = self.session.lock() {
                    s.events.clear();
                    s.recompute();
                }
            }
            "act:save" => {
                let _ = self.editor.save();
                self.toast("рџ’ѕ РЎРѕС…СЂР°РЅРµРЅРѕ".into());
            }
            "act:about" => self.ui.show_about = true,
            _ => {}
        }
    }
}

impl eframe::App for AnsibleApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Р“РѕСЂСЏС‡РёРµ РєР»Р°РІРёС€Рё: F5 вЂ” Р·Р°РїСѓСЃРє РїР»РµР№Р±СѓРєР°, Esc вЂ” РїРѕРґС‚РІРµСЂР¶РґРµРЅРёРµ РѕСЃС‚Р°РЅРѕРІРєРё
        if ctx.input(|i| i.key_pressed(egui::Key::F5)) {
            if self.playbook_path.is_empty() {
                self.ui.error_message = Some("РЈРєР°Р¶РёС‚Рµ РїСѓС‚СЊ Рє РїР»РµР№Р±СѓРєСѓ".into());
            } else if !std::path::Path::new(&self.playbook_path).exists() {
                self.ui.error_message = Some(format!(
                    "РџР»РµР№Р±СѓРє РЅРµ РЅР°Р№РґРµРЅ: {}",
                    self.playbook_path
                ));
            } else if self.inventory_path.is_empty() {
                self.ui.error_message = Some("РЈРєР°Р¶РёС‚Рµ РїСѓС‚СЊ Рє РёРЅРІРµРЅС‚Р°СЂСЋ".into());
            } else if !std::path::Path::new(&self.inventory_path).exists() {
                self.ui.error_message = Some(format!(
                    "РРЅРІРµРЅС‚Р°СЂСЊ РЅРµ РЅР°Р№РґРµРЅ: {}",
                    self.inventory_path
                ));
            } else {
                self.start_run();
            }
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            if let Ok(s) = self.session.lock() {
                if matches!(s.state, RunState::Running) {
                    self.stop_confirm = true;
                }
            }
        }

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
        if let Some(rx) = &mut self.scheduler_compliance.comp_rx {
            while let Ok(r) = rx.try_recv() {
                self.ops.comp_results.push(r);
            }
        }
        if let Some(rx) = &mut self.diff_versions.diff_rx {
            let mut buf = String::new();
            while let Ok(line) = rx.try_recv() {
                buf.push_str(&line);
                buf.push('\n');
            }
            if !buf.is_empty() {
                self.diff_versions.diff.scan_running = false;
                self.diff_versions.diff.changes = crate::ansible::diff::parse_check_output(&buf);
                self.toast(format!("рџ”Ќ РќР°Р№РґРµРЅРѕ РёР·РјРµРЅРµРЅРёР№: {}", self.diff_versions.diff.changes.len()));
            }
        }
        // Event-РїСЂР°РІРёР»Р°: РєР°Р¶РґС‹Рµ 60 СЃРµРє
        if self.scheduler_compliance.last_rule_check.elapsed() > Duration::from_secs(60) {
            self.scheduler_compliance.last_rule_check = Instant::now();
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
                self.scheduler_compliance.rule_rx = Some(rx);
                let pending: Vec<EventRule> = {
                    let mut out = Vec::new();
                    for r in &active {
                        // Cooldown Рё Р»РёРјРёС‚ Р·Р°РїСѓСЃРєРѕРІ
                        if let Some(t) = r.last_run {
                            if t.elapsed() < Duration::from_secs(r.cooldown_secs.max(1)) {
                                continue;
                            }
                        }
                        if r.runs >= r.max_runs {
                            continue;
                        }
                        // РћРіСЂР°РЅРёС‡РµРЅРёРµ РїРѕ С…РѕСЃС‚Р°Рј: РїСѓСЃС‚РѕРµ = hosts РёР· OpsCenter
                        let target = if r.allowed_hosts.is_empty() {
                            hosts.clone()
                        } else {
                            r.allowed_hosts.clone()
                        };
                        if let Some(msg) = ops::check_rule(&r.kind, &r.param, &target) {
                            let _ = tx.send(format!("вљ пёЏ [{}] {}", r.kind, msg));
                            // РџСЂРѕРёР·РІРѕР»СЊРЅС‹Р№ shell РІ Р°РІС‚РѕСЂРµР°РєС†РёРё Р·Р°РїСЂРµС‰С‘РЅ Р±РµР· РїРѕРґС‚РІРµСЂР¶РґРµРЅРёСЏ
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
                                "РћРџРђРЎРќРћ: "
                            } else {
                                ""
                            }
                        );
                        self.ops.rule_log.push(format!(
                            "вЏё РџСЂР°РІРёР»Рѕ {} С‚СЂРµР±СѓРµС‚ РїРѕРґС‚РІРµСЂР¶РґРµРЅРёСЏ РґРµР№СЃС‚РІРёСЏ: {}",
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
            self.toast("рџ©є Р”РёР°РіРЅРѕСЃС‚РёРєР° Р·Р°РІРµСЂС€РµРЅР°".into());
        }
        ctx.request_repaint_after(Duration::from_millis(100));

        if self.ui.theme_dirty {
            apply_theme(ctx, self.ui.theme_idx);
            self.ui.theme_dirty = false;
        }

        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::R))
            && self.tab == Tab::Run
        {
            self.start_run();
            self.toast("рџљЂ Р—Р°РїСѓСЃРє (Ctrl+R)".into());
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::S))
            && self.tab == Tab::Editor
        {
            let _ = self.editor.save();
            self.toast("рџ’ѕ РЎРѕС…СЂР°РЅРµРЅРѕ (Ctrl+S)".into());
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::P)) {
            self.ui.show_palette = !self.ui.show_palette;
            self.ui.palette_query.clear();
        }

        let mut due_tasks: Vec<(String, String)> = Vec::new();
        for t in self.scheduler_compliance.sched_tasks.iter_mut() {
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
            self.toast(format!("вЏ° РџР»Р°РЅРёСЂРѕРІС‰РёРє: {}", pb));
        }

        self.ui.toasts
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
                        "вљЎ",
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
                        if ui.small_button("в„№пёЏ Рћ РїСЂРѕРіСЂР°РјРјРµ").clicked() {
                            self.ui.show_about = true;
                        }
                        if ui.small_button("рџ”„").clicked() {
                            let url = format!("{}/releases/latest/download/VERSION", APP_REPO);
                            match ureq::get(&url).timeout(Duration::from_secs(5)).call() {
                                Ok(r) => {
                                    let latest =
                                        r.into_string().unwrap_or_default().trim().to_string();
                                    if latest.is_empty() {
                                        self.toast("рџ”„ РЎРµСЂРІРёСЃ РѕР±РЅРѕРІР»РµРЅРёР№: РїСѓСЃС‚РѕР№ РѕС‚РІРµС‚".into());
                                    } else if latest == APP_VERSION {
                                        self.toast(format!(
                                            "рџ”„ РЈСЃС‚Р°РЅРѕРІР»РµРЅР° Р°РєС‚СѓР°Р»СЊРЅР°СЏ РІРµСЂСЃРёСЏ ({})",
                                            latest
                                        ));
                                    } else {
                                        self.toast(format!("рџ”„ Р”РѕСЃС‚СѓРїРЅР° РЅРѕРІР°СЏ РІРµСЂСЃРёСЏ: {}", latest));
                                    }
                                }
                                Err(_) => self.toast("рџ”„ РЎРµСЂРІРёСЃ РѕР±РЅРѕРІР»РµРЅРёР№ РЅРµРґРѕСЃС‚СѓРїРµРЅ".into()),
                            }
                        }
                        if ui
                            .small_button(format!("рџЋЁ {}", THEME_NAMES[self.ui.theme_idx]))
                            .clicked()
                        {
                            self.cycle_theme(ctx);
                        }
                        if ui
                            .small_button(if self.ui.lang == Lang::Ru { "RU" } else { "EN" })
                            .clicked()
                        {
                            self.ui.lang = if self.ui.lang == Lang::Ru {
                                Lang::En
                            } else {
                                Lang::Ru
                            };
                        }
                        ui.add_space(8.0);
                        let state = self.session.lock().map(|s| s.state).unwrap_or(RunState::Idle);
                        match &state {
                            RunState::Idle => ui.colored_label(c_tm(), "в—Џ Idle"),
                            RunState::Running => ui.colored_label(c_orange(), "в—Џ Running"),
                            RunState::Finished { success } => ui.colored_label(
                                if *success { c_green() } else { c_red() },
                                if *success {
                                    "в—Џ Success"
                                } else {
                                    "в—Џ Failed"
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
                        (Tab::Run, "в–¶", "Run"),
                        (Tab::Editor, "рџ“ќ", "Editor"),
                        (Tab::Constructor, "рџЏ—", "Constructor"),
                        (Tab::Admin, "рџ› ", "Admin"),
                        (Tab::AdHoc, "рџ’»", "Ad-Hoc"),
                        (Tab::Library, "рџ“љ", "Library"),
                        (Tab::Roles, "рџ“¦", "Roles"),
                        (Tab::DiffView, "рџ”Ќ", "Diff"),
                        (Tab::Graph, "рџ•ё", "Graph"),
                        (Tab::Terraform, "рџЊЌ", "Terraform"),
                        (Tab::Tools, "рџ§°", "Tools"),
                        (Tab::History, "рџ—‚", "History"),
                        (Tab::Domain, "рџ–Ґ", "Domain"),
                        (Tab::Profiles, "рџ§ѕ", "Profiles"),
                        (Tab::Workflow, "рџ”Ђ", "Workflow"),
                        (Tab::OpsCenter, "вљЎ", "OpsCenter"),
                        (Tab::Compliance, "рџ›Ў", "Compliance"),
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
                    ui.colored_label(c_green(), "в—Џ");
                    ui.label(
                        RichText::new(&self.ui.ansible_version)
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
                    ui.label(RichText::new("Ctrl+P вЂ” РїР°Р»РёС‚СЂР°").color(c_tm()).size(12.0));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(
                            RichText::new(format!("{} v{}", APP_NAME, APP_VERSION))
                                .color(c_tm())
                                .size(12.0),
                        );
                    });
                });
            });

        if self.ui.show_about {
            let mut close_clicked = false;
            egui::Window::new(
                RichText::new(format!("в„№пёЏ Рћ РїСЂРѕРіСЂР°РјРјРµ {}", APP_NAME)).color(c_blue()),
            )
            .open(&mut self.ui.show_about)
            .default_size(Vec2::new(520.0, 480.0))
            .show(ctx, |ui| {
                ui.label(RichText::new(APP_NAME).size(22.0).strong().color(c_tx()));
                ui.label(RichText::new(format!("Р’РµСЂСЃРёСЏ {}", APP_VERSION)).color(c_tm()));
                ui.add_space(8.0);
                ui.separator();
                ui.add_space(6.0);
                ui.label(RichText::new("рџ¦– РќР°РїРёСЃР°РЅРѕ РЅР°:").strong().color(c_purple()));
                ui.label("вЂў Rust, egui/eframe 0.29, tokio, serde, syntect, petgraph, rfd, ureq");
                ui.add_space(6.0);
                ui.label(RichText::new("рџ”§ Р‘СЌРєРµРЅРґ:").strong().color(c_purple()));
                ui.label("вЂў Ansible Core, Terraform, Git, createrepo, Zabbix sender");
                ui.add_space(6.0);
                ui.label(RichText::new("рџ‘¤ РђРІС‚РѕСЂ:").strong().color(c_purple()));
                ui.label(format!("вЂў РџРѕС‡С‚Р°: {}", AUTHOR_EMAIL));
                ui.add_space(4.0);
                ui.label(RichText::new("рџ”— РџСЂРѕРµРєС‚:").strong().color(c_purple()));
                ui.hyperlink_to(APP_REPO, APP_REPO);
                ui.add_space(4.0);
                ui.label(RichText::new("рџ“њ Р›РёС†РµРЅР·РёСЏ: MIT").color(c_tm()));
                ui.add_space(10.0);
                if ui.button("Р—Р°РєСЂС‹С‚СЊ").clicked() {
                    close_clicked = true;
                }
            });
            if close_clicked {
                self.ui.show_about = false;
            }
        }

        if self.ui.show_palette {
            let mut action: Option<String> = None;
            let mut open = self.ui.show_palette;
            let mut query = self.ui.palette_query.clone();
            let cmds = self.commands();
            egui::Window::new("вЊ Command Palette")
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
            self.ui.palette_query = query;
            self.ui.show_palette = open;
            if let Some(id) = action {
                self.run_command(&id);
                self.ui.show_palette = false;
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
                for (msg, t) in &self.ui.toasts {
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

