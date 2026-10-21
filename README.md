# AnsibleOps

**AnsibleOps** — десктопный центр управления Ansible и Terraform: запуск playbook,
ad-hoc команды, визуальный конструктор, граф задач, инвентарь, роли, GitOps,
compliance, планировщик, отчёты, локальный RPM-репозиторий, Vault, Zabbix и
read-only Web API.

Написано на Rust (egui/eframe), работает на РЕДОС 7/8, CentOS/RHEL 7+, Fedora,
Debian/Ubuntu и Windows.

![AnsibleOps](logo.svg)

## Возможности

| Раздел | Что делает |
|---|---|
| **Run** | Запуск playbook с живым логом, отменой, маскированием секретов, PLAY RECAP |
| **Editor** | YAML-редактор с подсветкой синтаксиса, сохранение, версии файлов |
| **Constructor** | Визуальный конструктор playbook → YAML |
| **Admin** | Диагностика хостов, parallel ping, отчёты (html/json/csv) |
| **Ad-Hoc** | Произвольные ad-hoc команды Ansible |
| **Library** | Каталог модулей и их аргументов |
| **Roles** | Карточки ролей: версия, совместимость, риск, rollback |
| **Diff** | Сканирование `--check --diff` по хостам, сравнение файлов до/после |
| **Graph** | Граф зависимостей задач playbook, топологическая сортировка |
| **Terraform** | Запуск terraform plan/apply с потоком лога |
| **Tools** | Локальный RPM-репозиторий (createrepo), Git-операции, Vault, Zabbix sender |
| **History** | Журнал запусков, аудит, экспорт |
| **Domain** | Управление доменными ролями и политиками |
| **Profiles** | Профили конфигураций (шаблоны extra-vars) |
| **Workflow** | Сценарии цепочек задач |
| **OpsCenter** | Каталог типовых операций с уровнями безопасности Safe/Confirm/Danger |
| **Compliance** | Проверки Audit/Enforce по правилам безопасности |

Дополнительно: 4 темы оформления, RU/EN, палитра команд (Ctrl+P), планировщик
задач, маскирование секретов в логах, read-only Web API с bearer-токеном.

## Установка

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

Скомпилируйте `cargo build --release` и запустите `target\release\ansibleops.exe`.
Для Ansible нужен WSL либо прокси-скрипты `ansible-playbook.cmd` / `ansible.cmd`
в `PATH`.

## Требования

* Ansible Core 2.9+ (для запуска playbook) — опционально, GUI работает и без него;
* `git`, `terraform`, `createrepo_c` — нужны только для соответствующих вкладок;
* Графическое окружение: X11 или Wayland (XDG portal для диалогов выбора файлов).

## Сборка из исходников

```bash
git clone https://github.com/flytimopheev-sketch/ansibleops.git
cd ansibleops
cargo build --release
```

## Переменные окружения

| Переменная | Назначение |
|---|---|
| `ANSIBLEOPS_MOCK=1` | Демо-режим: если `ansible-playbook` не найден, показывать фиктивный вывод (явный opt-in) |
| `ANSIBLEOPS_CONFIG` | Путь к конфигурации (по умолчанию `~/.local/share/ansibleops/config.json`) |

## Документация

Полное руководство администратора: [`docs/manual.html`](docs/manual.html)
(версия 1.2.0), PDF — [`docs/AnsibleOps_Руководство_v1.2.0.pdf`](docs/).

## Лицензия

MIT — см. [LICENSE](LICENSE).

## Сообщество

Issues и Pull Requests: <https://github.com/flytimopheev-sketch/ansibleops/issues>
