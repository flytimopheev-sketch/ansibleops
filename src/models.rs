use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Lang {
    #[default]
    Ru,
    En,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Tab {
    #[default]
    Run,
    Editor,
    Constructor,
    Admin,
    AdHoc,
    Library,
    Roles,
    Graph,
    Terraform,
    Tools,
    History,
    Domain,
    Profiles,
    Workflow,
    OpsCenter,
    Compliance,
    DiffView,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    pub inventory_path: String,
    pub playbook_path: String,
    pub extra_vars: String,
    pub ssh: SshConfig,
    pub last_tab: Tab,
    #[serde(default)]
    pub theme: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SshConfig {
    pub private_key: Option<PathBuf>,
    pub vault_password_file: Option<PathBuf>,
    pub ask_vault_pass: bool,
    pub ask_become_pass: bool,
    pub forks: usize,
    pub use_become: bool,
    pub become_user: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Host {
    pub name: String,
    pub group: String,
    pub ansible_host: Option<String>,
    pub ansible_user: Option<String>,
    pub vars: HashMap<String, String>,
}

#[derive(Debug, Clone, Default)]
pub struct Playbook {
    #[allow(dead_code)]
    pub name: String,
    #[allow(dead_code)]
    pub path: PathBuf,
    pub raw_data: Vec<serde_yaml::Value>,
    pub tasks: Vec<TaskDef>,
}

#[derive(Debug, Clone, Default)]
pub struct TaskDef {
    pub name: String,
    #[allow(dead_code)]
    pub module: String,
    pub depends_on: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RunSession {
    pub id: String,
    pub playbook: String,
    pub inventory: String,
    pub state: RunState,
    pub started_at: Option<DateTime<Local>>,
    pub finished_at: Option<DateTime<Local>>,
    pub events: Vec<TaskEvent>,
    pub summary: RunSummary,
}

impl RunSession {
    pub fn new(playbook: String, inventory: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            playbook,
            inventory,
            ..Default::default()
        }
    }
    pub fn recompute(&mut self) {
        let mut s = RunSummary::default();
        for ev in &self.events {
            match ev.status {
                TaskStatus::Ok => s.ok += 1,
                TaskStatus::Changed => s.changed += 1,
                TaskStatus::Failed => s.failed += 1,
                TaskStatus::Skipped => s.skipped += 1,
                _ => {}
            }
        }
        self.summary = s;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RunSummary {
    pub ok: usize,
    pub changed: usize,
    pub failed: usize,
    pub skipped: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum RunState {
    #[default]
    Idle,
    Running,
    Finished {
        success: bool,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskEvent {
    pub task: String,
    pub host: String,
    pub status: TaskStatus,
    pub duration_ms: u64,
    pub output: String,
    pub timestamp: DateTime<Local>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum TaskStatus {
    Pending,
    Running,
    Ok,
    Changed,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AdminAction {
    pub target_hosts: String,
    pub module_type: AdminModuleType,
    pub action: String,
    pub name: String,
    pub extra_args: String,
    pub use_become: bool,
    pub check_mode: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AdminModuleType {
    #[default]
    Package,
    Service,
    User,
    File,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackgroundJob {
    pub id: String,
    pub name: String,
    pub status: String,
    pub progress: f32,
    pub output: Vec<String>,
    pub started_at: DateTime<Local>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TerraformConfig {
    pub workdir: PathBuf,
    pub auto_approve: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuiltInPlaybook {
    pub id: String,
    pub category: String,
    pub name: String,
    pub description: String,
    pub yaml_content: String,
}

#[derive(Debug, Clone, Default)]
pub struct ConstructorTask {
    pub name: String,
    pub module: String,
    pub params: HashMap<String, String>,
    pub use_become: bool,
    pub ignore_errors: bool,
    pub when_condition: String,
    pub register_var: String,
    pub notify_handler: String,
}

#[derive(Debug, Clone, Default)]
pub struct ConstructorPlay {
    pub name: String,
    pub hosts: String,
    pub use_become: bool,
    pub gather_facts: bool,
    pub tasks: Vec<ConstructorTask>,
}

#[derive(Debug, Clone)]
pub struct SchedTask {
    pub playbook: String,
    pub inventory: String,
    pub interval_min: u64,
    pub last_run: Option<std::time::Instant>,
}

#[derive(Debug, Clone)]
pub struct DiagResult {
    pub host: String,
    pub ping_ok: bool,
    pub ssh_ok: bool,
    pub port_ok: bool,
    pub port: u16,
}

#[derive(Debug, Clone)]
pub struct CompCheck {
    pub kind: String,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct ComplianceResult {
    pub check: String,
    pub expected: String,
    pub actual: String,
    pub ok: bool,
    pub fixed: bool,
}

#[derive(Debug, Clone)]
pub struct WfStep {
    pub kind: String,
    pub param: String,
    pub retries: u32,
    pub stop_on_fail: bool,
    pub confirm: bool,
}

#[derive(Debug, Clone)]
pub struct PendingOp {
    pub name: String,
    pub hosts: String,
    pub module: String,
    pub args: String,
    pub safety: String,
    pub reason: String,
}

#[derive(Debug, Clone)]
pub struct EventRule {
    pub kind: String,
    pub param: String,
    pub action: String,
    pub enabled: bool,
    /// Cooldown между срабатываниями (сек). 0 = без автозапуска действия.
    pub cooldown_secs: u64,
    /// Максимум запусков действия за сессию.
    pub max_runs: u32,
    pub runs: u32,
    pub last_run: Option<std::time::Instant>,
    /// Требовать ручного подтверждения перед действием (рекомендуется true).
    pub require_confirm: bool,
    /// Хосты, на которых разрешено действие (пусто = hosts из OpsCenter).
    pub allowed_hosts: String,
    /// Действие отклонено пользователем (для очереди подтверждения).
    pub rejected: bool,
}

impl Default for EventRule {
    fn default() -> Self {
        Self {
            kind: "disk".into(),
            param: "90".into(),
            action: String::new(),
            enabled: false,
            cooldown_secs: 3600,
            max_runs: 3,
            runs: 0,
            last_run: None,
            require_confirm: true,
            allowed_hosts: String::new(),
            rejected: false,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct OpsState {
    pub hosts: String,
    pub ops_log: Vec<String>,
    pub op_param: String,
    pub domain_op: String,
    pub domain_name: String,
    pub domain_extra: String,
    pub profile_sel: String,
    pub wf_steps: Vec<WfStep>,
    pub wf_running: bool,
    pub wf_confirm: bool,
    pub comp_checks: Vec<CompCheck>,
    pub comp_results: Vec<ComplianceResult>,
    pub comp_enforce: bool,
    pub pending: Option<PendingOp>,
    pub rules: Vec<EventRule>,
    pub rule_log: Vec<String>,
    pub pending_rules: Vec<EventRule>,
}

#[derive(Debug, Clone, Default)]
pub struct DiffState {
    pub changes: Vec<crate::ansible::diff::FileChange>,
    pub versions: Vec<crate::ansible::diff::FileVersion>,
    pub selected_file: String,
    pub selected_host: String,
    pub scan_running: bool,
}

#[derive(Debug, Clone, Default)]
pub struct RolesState {
    pub filter_tag: String,
    pub selected: String,
    pub target_hosts: String,
}

pub struct ModuleCatalog;
impl ModuleCatalog {
    pub fn categories() -> Vec<(&'static str, Vec<&'static str>)> {
        vec![
            ("Пакеты", vec!["dnf", "yum", "apt", "package"]),
            ("Службы", vec!["service", "systemd"]),
            ("Файлы", vec!["copy", "template", "file", "lineinfile"]),
            ("Пользователи", vec!["user", "group", "authorized_key"]),
            ("Сеть", vec!["firewalld", "hostname", "nmcli"]),
            ("Система", vec!["command", "shell", "script"]),
        ]
    }
    pub fn common_params(module: &str) -> Vec<&'static str> {
        match module {
            "dnf" | "yum" | "apt" | "package" => vec!["name", "state"],
            "service" | "systemd" => vec!["name", "state", "enabled"],
            "copy" => vec!["src", "dest", "mode"],
            "template" => vec!["src", "dest", "mode"],
            "file" => vec!["path", "state", "mode", "owner"],
            "lineinfile" => vec!["path", "regexp", "line"],
            "user" => vec!["name", "state", "shell"],
            "group" => vec!["name", "state"],
            "command" | "shell" => vec!["cmd"],
            _ => vec![],
        }
    }
}

pub struct ModuleReference;
impl ModuleReference {
    pub fn get_module_info(module: &str) -> (&'static str, &'static str, Vec<&'static str>) {
        match module {
            "dnf" => (
                "DNF пакетный менеджер",
                "Управление пакетами через dnf",
                vec!["name", "state", "update_cache"],
            ),
            "yum" => (
                "YUM пакетный менеджер",
                "Управление пакетами через yum",
                vec!["name", "state"],
            ),
            "apt" => (
                "APT пакетный менеджер",
                "Управление пакетами через apt",
                vec!["name", "state", "update_cache"],
            ),
            "service" => (
                "Управление службами",
                "Start/stop/restart/enable служб",
                vec!["name", "state", "enabled"],
            ),
            "user" => (
                "Управление пользователями",
                "Создание/удаление пользователей",
                vec!["name", "state", "shell", "groups"],
            ),
            "copy" => (
                "Копирование файлов",
                "Копирует файлы на хосты",
                vec!["src", "dest", "mode", "owner"],
            ),
            "template" => (
                "Шаблоны Jinja2",
                "Генерация файлов из шаблонов",
                vec!["src", "dest", "mode"],
            ),
            "file" => (
                "Управление файлами",
                "Создание файлов/каталогов/ссылок",
                vec!["path", "state", "mode"],
            ),
            "command" => ("Выполнение команд", "Запуск команд без shell", vec!["cmd"]),
            "shell" => ("Shell-команды", "Запуск команд через shell", vec!["cmd"]),
            "ping" => ("Проверка связи", "Проверяет доступность хоста", vec![]),
            "setup" => (
                "Сбор фактов",
                "Собирает системную информацию",
                vec!["filter"],
            ),
            _ => ("Модуль Ansible", "Стандартный модуль Ansible", vec![]),
        }
    }
}
