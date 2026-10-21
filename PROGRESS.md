# PROGRESS.md — журнал работы над AnsibleOps

> Файл для продолжения работы: в начале сессии читай этот файл, чтобы понять,
> на чём остановились. После каждой сессии дополняй раздел «Хронология» и
> обновляй «Текущее состояние» и «Следующие шаги».

## Проект
- **Название:** AnsibleOps (репозиторий `flytimopheev-sketch/ansibleops`), версия 1.2.0
- **Стек:** Rust + egui/eframe 0.29, tokio, serde, chrono, syntect, petgraph, rfd, ureq
- **Назначение:** GUI для Ansible/Terraform: запуск playbook, ad-hoc команды, граф задач,
  редактор YAML, diff-сканирование, история версий файлов, отчёты (html/json/csv), GitOps, роли.
- **Распространение:** нативный RPM для РЕДОС 7 / EL7 (`packaging/ansibleops.spec`),
  сборка через GitHub Actions (`.github/workflows/release.yml`).

## Текущее состояние (на 21.10.2026)
- Ребрендинг завершён: `Cargo.toml` → пакет `ansibleops`, версия 1.2.0, репозиторий
  `flytimopheev-sketch/ansibleops`; `theme.rs` → `APP_NAME = "AnsibleOps"`, версия из Cargo,
  `APP_REPO`, реальный email автора; `app.rs` → путь конфигурации `~/.local/share/ansibleops`,
  about-окно со ссылкой на проект и лицензией; `runner.rs` → mock только через
  `ANSIBLEOPS_MOCK=1`, иначе явная ошибка «Ansible не найден»; `webapi.rs` — убрано
  «AnsibleOps Studio».
- Сборка в РЕДОС 7 (контейнер `redosbuild` из `alrdockerhub/redos:latest`) проходит успешно:
  RPM `ansibleops-1.2.0-1.el7.x86_64.rpm` собран, `rpm -K` → digests OK, установка в чистый
  контейнер РЕДОС проходит (`dnf install` без ошибок зависимостей).
- Линтинг: `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings` и
  `cargo test --all` (5 тестов) проходят без замечаний.
- Упаковка: `packaging/ansibleops.spec`, `packaging/build-rpm.sh`, `packaging/ansibleops.desktop`,
  `packaging/ansibleops.service`, `packaging/env-check.sh`.
- CI: `.github/workflows/ci.yml` (fmt/clippy/check/test/build) и
  `.github/workflows/release.yml` (RPM в контейнере РЕДОС 7 + GitHub Release с `VERSION` и `SHA256SUMS`).
- Лицензия MIT добавлена (`LICENSE`), README переписан под AnsibleOps.
- `.gitattributes` фиксирует LF для скриптов/spec/yml — иначе `rpmbuild` падал на CRLF.

## Структура проекта
```
src/
  main.rs, app.rs (AnsibleApp), models.rs, theme.rs, ui_tabs.rs, ui_ops.rs (новый)
  ansible/
    mod.rs, runner.rs, adhoc.rs, editor.rs, graph.rs, history.rs,
    inventory.rs, library.rs, report.rs, webapi.rs, diagnostics.rs,
    gitops.rs, terraform.rs, vault.rs, zabbix.rs, rpmrepo.rs
    diff.rs     (новый: diff-сканирование, FileChange — до/после по файлам хостов)
    ops.rs      (новый: каталог операций OpsItem + уровни Safety: Safe/Confirm/Danger)
    roles.rs    (новый: карточки ролей RoleCard — версия, совместимость, риск, rollback)
playbooks/ (01-server-setup, 02-monitoring, 03-webserver, 08-docker-install,
            docker-ce-install.yml — новый)
docs/ (manual.html v1.2.0, PDF-руководство, cover.svg)
packaging/ (ansibleops.spec, build-rpm.sh, ansibleops.desktop, ansibleops.service, env-check.sh)
.github/workflows/ (ci.yml, release.yml)
site.yaml, inventory.ini, ansible-playbook.cmd
```

## Хронология изменений (последняя сессия)
- Ребрендинг ansible-gui → AnsibleOps (см. «Текущее состояние»).
- Добавлена RPM-упаковка: spec, скрипт сборки, desktop-файл, systemd-unit.
- Добавлены GitHub Actions: CI (fmt/clippy/check/test/build) и release (RPM для EL7 + Release).
- Добавлены `LICENSE` (MIT), обновлены `README.md`, `package.json`, `.gitignore`, `.gitattributes`.
- RPM успешно собран в контейнере РЕДОС 7, проверен `rpm -K` / `rpm -qpl` / `rpm -qp --requires`;
  пакет установлен в чистый контейнер РЕДОС (`dnf install`) — ошибок зависимостей нет.
- Устранены все предупреждения `cargo check` (dead_code) и 16 замечаний `clippy -D warnings`,
  чтобы CI-пайплайн проходил без откатов.

## Следующие шаги
1. `gh auth status` / git remote — создать публичный репозиторий
   `flytimopheev-sketch/ansibleops`, выполнить push.
2. Прогнать CI на GitHub и убедиться, что все jobs зелёные.
3. Выпустить тег `v1.2.0` → release-workflow соберёт RPM и опубликует Release с `VERSION`.
4. Проверить обновление версии из
   `https://github.com/flytimopheev-sketch/ansibleops/releases/latest/download/VERSION`.
5. Деплой RPM в репозиторий РЕДОС (createrepo) — при наличии инфраструктуры.

## Как продолжать
- Читать этот файл → выполнить «Следующие шаги» → при завершении сессии
  обновить «Хронология», «Текущее состояние», «Следующие шаги».
