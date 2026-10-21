#[derive(Debug, Clone)]
pub struct RoleCard {
    pub name: &'static str,
    pub version: &'static str,
    pub compat: &'static [&'static str],
    pub risk: &'static str,
    pub author: &'static str,
    pub description: &'static str,
    pub tags: &'static [&'static str],
    pub requires_confirm: bool,
    pub has_rollback: bool,
    pub yaml: &'static str,
}

pub fn role_catalog() -> Vec<RoleCard> {
    vec![
        RoleCard {
            name: "hardening-ssh",
            version: "1.4.0",
            compat: &["Red OS 7", "Red OS 8", "CentOS 7"],
            risk: "высокий",
            author: "AnsibleOps",
            description: "SSH hardening: запрет root, отключение password-auth, cipher-наборы",
            tags: &["security", "ssh", "hardening"],
            requires_confirm: true,
            has_rollback: true,
            yaml: r#"---
- name: SSH Hardening Role
  hosts: all
  become: yes
  handlers:
    - name: restart sshd
      service: { name: sshd, state: restarted }
  tasks:
    - name: Отключить root login
      lineinfile: { path: /etc/ssh/sshd_config, regexp: '^PermitRootLogin', line: 'PermitRootLogin no' }
      notify: restart sshd
    - name: Отключить password auth
      lineinfile: { path: /etc/ssh/sshd_config, regexp: '^PasswordAuthentication', line: 'PasswordAuthentication no' }
      notify: restart sshd
"#,
        },
        RoleCard {
            name: "firewall-basic",
            version: "1.0.2",
            compat: &["Red OS 7", "Red OS 8"],
            risk: "средний",
            author: "AnsibleOps",
            description: "Firewalld: SSH + ICMP, остальное закрыто",
            tags: &["security", "firewall"],
            requires_confirm: true,
            has_rollback: true,
            yaml: r#"---
- name: Firewall Basic Role
  hosts: all
  become: yes
  tasks:
    - name: Установить firewalld
      dnf: { name: firewalld, state: present }
    - name: Запустить firewalld
      service: { name: firewalld, state: started, enabled: yes }
    - name: Разрешить SSH
      firewalld: { service: ssh, state: enabled, permanent: yes }
    - name: Применить правила
      command: firewall-cmd --reload
"#,
        },
        RoleCard {
            name: "ntp-setup",
            version: "2.1.0",
            compat: &["Red OS 7", "Red OS 8", "Debian 11"],
            risk: "низкий",
            author: "AnsibleOps",
            description: "Chrony с локальными NTP-серверами",
            tags: &["ntp", "time", "base"],
            requires_confirm: false,
            has_rollback: true,
            yaml: r#"---
- name: NTP Setup Role
  hosts: all
  become: yes
  tasks:
    - name: Установить chrony
      dnf: { name: chrony, state: present }
    - name: Настроить серверы
      copy:
        content: |
          server 0.pool.ntp.org iburst
          driftfile /var/lib/chrony/drift
        dest: /etc/chrony.conf
    - name: Запустить chronyd
      service: { name: chronyd, state: started, enabled: yes }
"#,
        },
        RoleCard {
            name: "user-deployer",
            version: "1.0.0",
            compat: &["Red OS 7", "Red OS 8"],
            risk: "низкий",
            author: "AnsibleOps",
            description: "Служебный пользователь deployer с sudo без пароля",
            tags: &["user", "deploy"],
            requires_confirm: false,
            has_rollback: false,
            yaml: r#"---
- name: Deployer User Role
  hosts: all
  become: yes
  vars: { deploy_user: deployer }
  tasks:
    - name: Создать пользователя
      user: { name: "{{ deploy_user }}", shell: /bin/bash, groups: wheel }
    - name: Настроить sudo
      copy:
        content: "{{ deploy_user }} ALL=(ALL) NOPASSWD: ALL\n"
        dest: "/etc/sudoers.d/{{ deploy_user }}"
        mode: '0440'
"#,
        },
        RoleCard {
            name: "logrotate-hardening",
            version: "1.2.1",
            compat: &["Red OS 7", "Red OS 8"],
            risk: "низкий",
            author: "AnsibleOps",
            description: "Logrotate: размер 100M, 4 копии",
            tags: &["logs", "maintenance"],
            requires_confirm: false,
            has_rollback: true,
            yaml: r#"---
- name: Logrotate Role
  hosts: all
  become: yes
  tasks:
    - name: Настроить logrotate для messages
      copy:
        content: |
          /var/log/messages {
              rotate 4
              weekly
              maxsize 100M
              compress
          }
        dest: /etc/logrotate.d/syslog-hardening
"#,
        },
        RoleCard {
            name: "audit-setup",
            version: "1.0.0",
            compat: &["Red OS 7", "Red OS 8"],
            risk: "средний",
            author: "AnsibleOps",
            description: "Auditd: отслеживание действий root",
            tags: &["security", "audit"],
            requires_confirm: true,
            has_rollback: true,
            yaml: r#"---
- name: Audit Setup Role
  hosts: all
  become: yes
  tasks:
    - name: Установить audit
      dnf: { name: audit, state: present }
    - name: Базовые правила
      copy:
        content: |
          -w /etc/passwd -p wa -k identity
          -w /etc/shadow -p wa -k identity
        dest: /etc/audit/rules.d/ansibleops.rules
    - name: Перезапустить auditd
      service: { name: auditd, state: restarted }
"#,
        },
        RoleCard {
            name: "docker-ce-install",
            version: "24.0.5",
            compat: &["Red OS 7", "Red OS 8"],
            risk: "средний",
            author: "AnsibleOps",
            description: "Docker CE из официального репозитория",
            tags: &["docker", "containers"],
            requires_confirm: true,
            has_rollback: false,
            yaml: r#"---
- name: Docker CE Install Role
  hosts: all
  become: yes
  tasks:
    - name: Установить docker
      dnf: { name: [docker-ce, docker-ce-cli, containerd.io], state: present }
    - name: Запустить docker
      service: { name: docker, state: started, enabled: yes }
"#,
        },
    ]
}

pub fn role_to_playbook(r: &RoleCard, hosts: &str) -> String {
    r.yaml.replace("hosts: all", &format!("hosts: {}", hosts))
}

pub fn role_tags_list() -> Vec<&'static str> {
    let mut v: Vec<&'static str> = Vec::new();
    for r in role_catalog() {
        for t in r.tags {
            if !v.contains(t) {
                v.push(*t);
            }
        }
    }
    v
}
