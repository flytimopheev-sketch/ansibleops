use crate::models::{CompCheck, WfStep};

#[derive(Clone, Copy, PartialEq)]
pub enum Safety {
    Safe,
    Confirm,
    Danger,
}

pub struct OpsItem {
    pub name: &'static str,
    pub module: &'static str,
    pub args: &'static str,
    pub safety: Safety,
    #[allow(dead_code)]
    pub need_param: bool,
}

pub fn ops_catalog() -> Vec<OpsItem> {
    vec![
        OpsItem { name: "Проверка места на диске", module: "shell", args: "df -h", safety: Safety::Safe, need_param: false },
        OpsItem { name: "IP / hostname / версия ОС", module: "shell", args: "hostname; hostname -I; cat /etc/os-release | head -3", safety: Safety::Safe, need_param: false },
        OpsItem { name: "Память и нагрузка", module: "shell", args: "free -h; uptime", safety: Safety::Safe, need_param: false },
        OpsItem { name: "Журналы journalctl (50 строк)", module: "shell", args: "journalctl -n 50 --no-pager", safety: Safety::Safe, need_param: false },
        OpsItem { name: "Проверка сертификатов", module: "shell", args: "for c in /etc/ssl/certs/*.pem; do openssl x509 -enddate -noout -in $c 2>/dev/null; done", safety: Safety::Safe, need_param: false },
        OpsItem { name: "Очистка временных файлов", module: "shell", args: "rm -rf /tmp/* /var/tmp/*", safety: Safety::Confirm, need_param: false },
        OpsItem { name: "Обновление пакетов", module: "shell", args: "dnf update -y", safety: Safety::Confirm, need_param: false },
        OpsItem { name: "Перезапуск службы", module: "shell", args: "systemctl restart {p}", safety: Safety::Confirm, need_param: true },
        OpsItem { name: "Остановка службы", module: "shell", args: "systemctl stop {p}", safety: Safety::Confirm, need_param: true },
        OpsItem { name: "Добавить пользователя", module: "shell", args: "useradd -m {p}", safety: Safety::Confirm, need_param: true },
        OpsItem { name: "Изменить SSH-ключ пользователя", module: "shell", args: "echo '{p}' >> /root/.ssh/authorized_keys", safety: Safety::Confirm, need_param: true },
        OpsItem { name: "Перезагрузка хоста", module: "shell", args: "reboot", safety: Safety::Danger, need_param: false },
        OpsItem { name: "Произвольная команда", module: "shell", args: "{p}", safety: Safety::Danger, need_param: true },
    ]
}

pub fn domain_catalog() -> Vec<(&'static str, &'static str, bool, bool)> {
    vec![
        (
            "Просмотр пользователей домена",
            "ipa user-find 2>/dev/null || wbinfo -u",
            false,
            false,
        ),
        (
            "Просмотр групп домена",
            "ipa group-find 2>/dev/null || wbinfo -g",
            false,
            false,
        ),
        (
            "Просмотр компьютеров домена",
            "ipa host-find 2>/dev/null || echo standalone",
            false,
            false,
        ),
        (
            "Создать пользователя",
            "ipa user-add {n} --first=Ops --last=User 2>/dev/null || useradd {n}",
            true,
            false,
        ),
        (
            "Массово отключить учётки",
            "for u in $(echo {n}); do usermod -L $u; done",
            true,
            false,
        ),
        (
            "Массово включить учётки",
            "for u in $(echo {n}); do usermod -U $u; done",
            true,
            false,
        ),
        ("Сброс пароля пользователя", "passwd {n}", true, false),
        (
            "Снять блокировку учётки",
            "faillock --user {n} --reset 2>/dev/null || pam_tally2 --user={n} --reset",
            true,
            false,
        ),
        (
            "Ввести машину в домен",
            "realm join {e} 2>/dev/null || net ads join -S {e}",
            false,
            true,
        ),
        (
            "Вывести машину из домена",
            "realm leave 2>/dev/null || net ads leave",
            false,
            false,
        ),
        (
            "Последнее подключение компьютера",
            "lastlog -u {n} 2>/dev/null || last -n 5",
            true,
            false,
        ),
        (
            "Назначить группу компьютерам",
            "ipa group-add-member {e} --hosts={n} 2>/dev/null || echo group-assign",
            true,
            true,
        ),
        (
            "Действие для подразделения (OU)",
            "ipa host-find --hostname={n} 2>/dev/null || echo ou-action",
            true,
            true,
        ),
    ]
}

pub fn comp_audit_cmd(c: &CompCheck) -> String {
    match c.kind.as_str() {
        "service" => format!("systemctl is-active {}", c.name),
        "package" => format!("rpm -q {}", c.name),
        "port" => format!("ss -ltn | grep -q :{} && echo OPEN || echo CLOSED", c.name),
        _ => format!("test -e {} && echo EXISTS || echo MISSING", c.name),
    }
}

pub fn comp_expected(c: &CompCheck) -> String {
    match c.kind.as_str() {
        "service" => "active".into(),
        "package" => "установлен".into(),
        "port" => "OPEN".into(),
        _ => "EXISTS".into(),
    }
}

pub fn comp_fix(c: &CompCheck) -> String {
    match c.kind.as_str() {
        "service" => format!("systemctl enable --now {}", c.name),
        "package" => format!("dnf install -y {}", c.name),
        "port" => format!(
            "firewall-cmd --add-port={}/tcp --permanent && firewall-cmd --reload",
            c.name
        ),
        _ => format!("touch {}", c.name),
    }
}

pub fn comp_ok(c: &CompCheck, success: bool, out: &str) -> bool {
    match c.kind.as_str() {
        "service" => out.contains("active"),
        "package" => success,
        "port" => out.contains("OPEN"),
        _ => success,
    }
}

pub struct ProfileDef {
    pub name: &'static str,
    pub desc: &'static str,
    pub packages: &'static [&'static str],
    pub services: &'static [&'static str],
    pub hardening: bool,
}

pub fn profiles() -> Vec<ProfileDef> {
    vec![
        ProfileDef {
            name: "Базовая рабочая станция",
            desc: "Минимум для работы: офис, браузер, утилиты",
            packages: &["firefox", "libreoffice", "unzip", "htop"],
            services: &["sshd", "NetworkManager"],
            hardening: false,
        },
        ProfileDef {
            name: "Сервер приложений",
            desc: "Java/Python-стеки, nginx",
            packages: &["nginx", "java-17-openjdk", "python3"],
            services: &["nginx", "sshd"],
            hardening: true,
        },
        ProfileDef {
            name: "Файловый сервер",
            desc: "Samba/NFS + резервное копирование",
            packages: &["samba", "nfs-utils", "rsync"],
            services: &["smb", "nfs-server", "sshd"],
            hardening: true,
        },
        ProfileDef {
            name: "АРМ администратора",
            desc: "Инструменты администрирования",
            packages: &["ansible-core", "git", "tmux", "nmap", "wireshark-cli"],
            services: &["sshd"],
            hardening: true,
        },
        ProfileDef {
            name: "Безопасный профиль",
            desc: "Hardening: firewall, audit, минимум пакетов",
            packages: &["audit", "firewalld", "selinux-policy-targeted"],
            services: &["firewalld", "auditd", "sshd"],
            hardening: true,
        },
        ProfileDef {
            name: "Профиль разработчика",
            desc: "GCC, Rust, Docker",
            packages: &["gcc", "cargo", "docker", "git", "code"],
            services: &["docker", "sshd"],
            hardening: false,
        },
        ProfileDef {
            name: "Профиль терминального сервера",
            desc: "RDP/VNC для удалённых сессий",
            packages: &["xrdp", "tigervnc-server"],
            services: &["xrdp", "sshd"],
            hardening: true,
        },
    ]
}

pub fn profile_yaml(p: &ProfileDef) -> String {
    let mut y = String::from("---\n- name: Профиль: ");
    y.push_str(&format!(
        "{}\n  hosts: all\n  become: yes\n  tasks:\n",
        p.name
    ));
    for pkg in p.packages {
        y.push_str(&format!(
            "    - name: Пакет {}\n      dnf:\n        name: {}\n        state: present\n",
            pkg, pkg
        ));
    }
    for svc in p.services {
        y.push_str(&format!("    - name: Служба {}\n      service:\n        name: {}\n        state: started\n        enabled: yes\n", svc, svc));
    }
    if p.hardening {
        y.push_str("    - name: Firewall включён\n      service:\n        name: firewalld\n        state: started\n        enabled: yes\n");
        y.push_str("    - name: Запрет root-логина\n      lineinfile:\n        path: /etc/ssh/sshd_config\n        regexp: '^PermitRootLogin'\n        line: 'PermitRootLogin no'\n");
    }
    y
}

pub fn step_command(kind: &str, param: &str) -> (&'static str, String) {
    match kind {
        "ping" => ("ping", String::new()),
        "backup" => (
            "shell",
            "tar czf /root/cfg-backup-$(date +%F-%H%M).tar.gz /etc".into(),
        ),
        "update" => ("shell", "dnf update -y".into()),
        "restart" => ("shell", format!("systemctl restart {}", param)),
        "check" => ("shell", format!("systemctl is-active {}", param)),
        "report" => ("shell", "hostname; uptime; df -h /; free -h".into()),
        _ => ("shell", param.to_string()),
    }
}

pub fn step_label(kind: &str) -> &'static str {
    match kind {
        "ping" => "Проверить доступность",
        "backup" => "Резервная копия /etc",
        "update" => "Обновить пакеты",
        "restart" => "Перезапустить службу",
        "check" => "Проверить службу",
        "report" => "Собрать отчёт",
        _ => "Произвольная команда",
    }
}

pub fn wf_default() -> Vec<WfStep> {
    vec![
        WfStep {
            kind: "ping".into(),
            param: "".into(),
            retries: 1,
            stop_on_fail: true,
            confirm: false,
        },
        WfStep {
            kind: "backup".into(),
            param: "".into(),
            retries: 1,
            stop_on_fail: true,
            confirm: false,
        },
        WfStep {
            kind: "update".into(),
            param: "".into(),
            retries: 2,
            stop_on_fail: true,
            confirm: true,
        },
        WfStep {
            kind: "restart".into(),
            param: "sshd".into(),
            retries: 1,
            stop_on_fail: false,
            confirm: false,
        },
        WfStep {
            kind: "check".into(),
            param: "sshd".into(),
            retries: 2,
            stop_on_fail: true,
            confirm: false,
        },
        WfStep {
            kind: "report".into(),
            param: "".into(),
            retries: 1,
            stop_on_fail: false,
            confirm: false,
        },
    ]
}

// === EVENT-ПРАВИЛА ===
pub fn check_rule(kind: &str, param: &str, hosts: &str) -> Option<String> {
    let run = |cmd: &str| -> Option<String> {
        let o = std::process::Command::new("ansible")
            .arg(hosts)
            .arg("-m")
            .arg("shell")
            .arg("-a")
            .arg(cmd)
            .output()
            .ok()?;
        Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
    };
    match kind {
        "disk" => {
            let v = run("df / --output=pcent | tail -1 | tr -dc 0-9")?;
            let n: u32 = v.parse().unwrap_or(0);
            let th: u32 = param.parse().unwrap_or(90);
            if n >= th {
                Some(format!("Диск заполнен на {}% (порог {}%)", n, th))
            } else {
                None
            }
        }
        "service" => {
            let v = run(&format!("systemctl is-active {}", param)).unwrap_or_default();
            if !v.contains("active") {
                Some(format!("Служба '{}' не активна", param))
            } else {
                None
            }
        }
        "host" => {
            let r = crate::ansible::diagnostics::check_host(hosts, 22);
            if !r.ping_ok {
                Some(format!("Хост '{}' недоступен", hosts))
            } else {
                None
            }
        }
        _ => None,
    }
}
