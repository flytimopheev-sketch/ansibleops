use crate::models::BuiltInPlaybook;
use uuid::Uuid;

pub fn get_builtin_library() -> Vec<BuiltInPlaybook> {
    vec![
        BuiltInPlaybook {
            id: Uuid::new_v4().to_string(),
            category: "🖥️ Система".into(),
            name: "Обновление системы и перезагрузка".into(),
            description: "Безопасное обновление всех пакетов с перезагрузкой при необходимости. Использует serial: 1 для поочерёдного обновления серверов.".into(),
            yaml_content: r#"---
- name: Update system
  hosts: all
  become: yes
  serial: 1
  tasks:
    - name: Update all packages
      dnf:
        name: "*"
        state: latest
        update_only: yes
    - name: Check if reboot is required
      stat:
        path: /var/run/reboot-required
      register: reboot_required
    - name: Reboot if needed
      reboot:
        msg: "Rebooting after updates"
      when: reboot_required.stat.exists
"#.into(),
        },
        BuiltInPlaybook {
            id: Uuid::new_v4().to_string(),
            category: "🔒 Безопасность".into(),
            name: "Базовый SSH Hardening".into(),
            description: "Отключение root-логина и парольной аутентификации. Обязательно настройте SSH-ключи перед запуском!".into(),
            yaml_content: r#"---
- name: SSH Hardening
  hosts: all
  become: yes
  tasks:
    - name: Disable root login
      lineinfile:
        path: /etc/ssh/sshd_config
        regexp: '^PermitRootLogin'
        line: 'PermitRootLogin no'
      notify: restart sshd
    - name: Disable password auth
      lineinfile:
        path: /etc/ssh/sshd_config
        regexp: '^PasswordAuthentication'
        line: 'PasswordAuthentication no'
      notify: restart sshd
  handlers:
    - name: restart sshd
      service:
        name: sshd
        state: restarted
"#.into(),
        },
        BuiltInPlaybook {
            id: Uuid::new_v4().to_string(),
            category: "🔍 Диагностика".into(),
            name: "Сбор базовых метрик".into(),
            description: "Ping, проверка места на диске и версии ОС. Безопасный плейбук только для чтения.".into(),
            yaml_content: r#"---
- name: Diagnostics
  hosts: all
  gather_facts: yes
  tasks:
    - name: Ping hosts
      ping:
    - name: Check disk space
      shell: df -h /
      register: disk
      changed_when: false
    - name: Show disk space
      debug:
        var: disk.stdout_lines
"#.into(),
        },
        BuiltInPlaybook {
            id: Uuid::new_v4().to_string(),
            category: "👤 Пользователи".into(),
            name: "Создание пользователя с SSH-ключом".into(),
            description: "Создание пользователя deployer с sudo правами и SSH-ключом.".into(),
            yaml_content: r#"---
- name: Create user
  hosts: all
  become: yes
  vars:
    username: deployer
    ssh_key: "ssh-rsa AAAA... ваш_ключ ..."
  tasks:
    - name: Create user
      user:
        name: "{{ username }}"
        groups: wheel
        shell: /bin/bash
        create_home: yes
    - name: Add SSH key
      authorized_key:
        user: "{{ username }}"
        key: "{{ ssh_key }}"
    - name: Allow sudo without password
      copy:
        content: "{{ username }} ALL=(ALL) NOPASSWD: ALL\n"
        dest: "/etc/sudoers.d/{{ username }}"
        mode: '0440'
"#.into(),
        },
        BuiltInPlaybook {
            id: Uuid::new_v4().to_string(),
            category: "🐳 Docker".into(),
            name: "Установка Docker".into(),
            description: "Установка Docker CE и docker-compose на хосты.".into(),
            yaml_content: r#"---
- name: Install Docker
  hosts: all
  become: yes
  tasks:
    - name: Install dependencies
      dnf:
        name:
          - dnf-plugins-core
          - device-mapper-persistent-data
          - lvm2
        state: present
    - name: Add Docker repo
      command: dnf config-manager --add-repo https://download.docker.com/linux/centos/docker-ce.repo
      args:
        creates: /etc/yum.repos.d/docker-ce.repo
    - name: Install Docker
      dnf:
        name:
          - docker-ce
          - docker-ce-cli
          - containerd.io
        state: present
    - name: Start Docker
      service:
        name: docker
        state: started
        enabled: yes
"#.into(),
        },
    ]
}
