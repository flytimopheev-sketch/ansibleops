# AnsibleOps

[![CI](https://github.com/flytimopheev-sketch/ansibleops/actions/workflows/ci.yml/badge.svg)](https://github.com/flytimopheev-sketch/ansibleops/actions/workflows/ci.yml)
[![Release](https://github.com/flytimopheev-sketch/ansibleops/actions/workflows/release.yml/badge.svg)](https://github.com/flytimopheev-sketch/ansibleops/actions/workflows/release.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://www.rust-lang.org/)
[![egui](https://img.shields.io/badge/egui-0.29-purple.svg)](https://github.com/emilk/egui)
[![PRs Welcome](https://img.shields.io/badge/PRs-welcome-brightgreen.svg)](CONTRIBUTING.md)

**AnsibleOps** — десктопный центр управления Ansible и Terraform: запуск playbook,
ad-hoc команды, визуальный конструктор, граф задач, инвентарь, роли, GitOps,
compliance, планировщик, отчёты, локальный RPM-репозиторий, Vault, Zabbix и
read-only Web API.

Написано на **Rust (egui/eframe 0.29)**, работает на **РЕДОС 7/8, CentOS/RHEL 7+,
Fedora, Debian/Ubuntu и Windows**.

<p align="center">
  <img src="logo.svg" width="200" alt="AnsibleOps logo">
</p>

## 📋 Возможности

| Раздел | Что делает |
|--------|------------|
| **Run** | Запуск playbook с живым логом, отменой, маскированием секретов, PLAY RECAP |
| **Editor** | YAML-редактор с подсветкой синтаксиса, сохранение, версии файлов |
| **Constructor** | Визуальный конструктор playbook → YAML |
| **Admin** | Диагностика хостов, parallel ping, отчёты (html/json/csv) |
| **Ad-Hoc** | Произвольные ad-hoc команды Ansible |
| **Library** | Каталог модулей и их аргументов |
| **Roles** | Карточки ролей: версия, совместимость, риск, rollback |
| **Diff** | Сканирование `--check --diff` по хостам, сравнение файлов до/после |
| **Graph** | Граф зависимостей задач playbook, топологическая сортировка |
| **Terraform** | Запуск `terraform plan/apply` с потоком лога |
| **Tools** | Локальный RPM-репозиторий (createrepo), Git-операции, Vault, Zabbix sender |
| **History** | Журнал запусков, аудит, экспорт |
| **Domain** | Управление доменными ролями и политиками |
| **Profiles** | Профили конфигураций (шаблоны extra-vars) |
| **Workflow** | Сценарии цепочек задач |
| **OpsCenter** | Каталог типовых операций с уровнями безопасности Safe/Confirm/Danger |
| **Compliance** | Проверки Audit/Enforce по правилам безопасности |

Дополнительно: **4 темы оформления**, **RU/EN**, **палитра команд (Ctrl+P)**,
**планировщик задач**, **маскирование секретов в логах**,
**read-only Web API с bearer-токеном**.

## 🚀 Установка

### РЕДОС / CentOS 7+ / RHEL (RPM)

```bash
sudo dnf install ./ansibleops-1.2.0-1.el7.x86_64.rpm
ansibleops
```

### Любой Linux (tarball)

```bash
tar xzf ansibleops-1.2.0-x86_64-linux.tar.gz
./ansibleops
```

### Windows

Скомпилируйте `cargo build --release` и запустите `target\\release\\ansibleops.exe`.

Для Ansible нужен **WSL** либо прокси-скрипты `ansible-playbook.cmd` /
`ansible.cmd` в `PATH`.

### Из исходников

```bash
git clone https://github.com/flytimopheev-sketch/ansibleops.git
cd ansibleops
cargo build --release
```

## 🔧 Требования

| Компонент | Версия | Примечание |
|-----------|--------|------------|
| Ansible Core | 2.9+ | Для запуска playbook — опционально, GUI работает и без него |
| git | — | Для GitOps-вкладки |
| terraform | — | Для Terraform-вкладки |
| createrepo_c | — | Для локального RPM-репозитория |
| Графическое окружение | X11 / Wayland | XDG portal для диалогов выбора файлов |

## 🌍 Переменные окружения

| Переменная | Назначение |
|------------|------------|
| `ANSIBLEOPS_MOCK=1` | Демо-режим: если `ansible-playbook` не найден, показывать фиктивный вывод (явный opt-in) |
| `ANSIBLEOPS_CONFIG` | Путь к конфигурации (по умолчанию `~/.local/share/ansibleops/config.json`) |

## 📖 Документация

Полное руководство администратора: [`docs/manual.html`](docs/manual.html)
(версия 1.2.0), PDF — `docs/AnsibleOps_Руководство_v1.2.0.pdf`.

## 🔒 Безопасность

Веб-API использует **криптографически стойкий bearer-токен** (24 случайных байта,
hex-кодировка). Авторизация проверяется строго по заголовку
`Authorization: Bearer <token>`.

> ⚠️ НеExpose веб-API на `0.0.0.0` без понимания рисков. По умолчанию сервер слушает
> `127.0.0.1`.

Если вы нашли уязвимость, пожалуйста, сообщите через [GitHub Issues](https://github.com/flytimopheev-sketch/ansibleops/issues)
или напишите на почту автора. См. также [SECURITY.md](.github/SECURITY.md).

## 🛠 Сборка и разработка

```bash
# Проверка форматирования
cargo fmt --all -- --check

# Линтинг
cargo clippy --all-targets -- -D warnings

# Тесты
cargo test --all

# Сборка release
cargo build --release
```

## 📦 Упаковка

- **RPM** для РЕДОС 7 / EL7: `packaging/ansibleops.spec`, `packaging/build-rpm.sh`
- **Desktop файл**: `packaging/ansibleops.desktop`
- **Systemd unit**: `packaging/ansibleops.service`

CI собирает RPM в контейнере РЕДОС 7 и публикует GitHub Release с `VERSION` и
`SHA256SUMS`.

## 🤝 Contributing

Мы приветствуем Pull Requests! Пожалуйста, ознакомьтесь с
[руководством по contribuции](CONTRIBUTING.md) и
[Кодексом поведения](CODE_OF_CONDUCT.md).

## 📄 Лицензия

Лицензировано под [MIT License](LICENSE).

---

**Автор:** flytimopheev-sketch <flytimopheev-sketch@users.noreply.github.com>  
**Репозиторий:** https://github.com/flytimopheev-sketch/ansibleops  
**Issues и Pull Requests:** https://github.com/flytimopheev-sketch/ansibleops/issues