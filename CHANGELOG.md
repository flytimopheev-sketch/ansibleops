# CHANGELOG

Все заметные изменения в проекте AnsibleOps задокументированы в этом файле.

Формат следует принципам [Keep a Changelog](https://keepachangelog.com/ru/1.0.0/), и проект придерживается [Semantic Versioning](https://semver.org/lang/ru/).

## [1.2.0] — 2026-10-21

### Добавлено
- Ребрендинг в AnsibleOps, версия 1.2.0
- RPM-упаковка для РЕДОС 7 / EL7 (spec, скрипт сборки, desktop-файл, systemd-unit)
- GitHub Actions: CI (fmt/clippy/check/test/build) и release (RPM + Release)
- Лицензия MIT, обновлённый README
- Новые модули: `diff.rs` (diff-сканирование), `ops.rs` (каталог операций Safety), `roles.rs` (карточки ролей)
- Playbook `docker-ce-install.yml`
- 4 темы оформления, RU/EN, палитра команд (Ctrl+P), планировщик задач
- Маскирование секретов в логах, read-only Web API с bearer-токеном

### Исправлено
- Все предупреждения `cargo check` (dead_code) устранены
- 16 замечаний `clippy -D warnings` устранены
- Мок-режим теперь только через явный флаг `ANSIBLEOPS_MOCK=1`

### Безопасность
- Токен веб-API: замена нестойкого `DefaultHasher` на криптографически стойкий `rand::thread_rng()`
- Авторизация веб-API: замена `substring contains` на строгую проверку `Authorization: Bearer`
- Санитайзинг аргументов shell-команд в `check_rule` (`systemctl is-active`)

## [1.1.0] — 2026-09-15

### Добавлено
- Начальная версия AnsibleOps Studio (предыдущее название)
- Базовый запуск playbook, ad-hoc, инвентарь, редактор YAML

---

### Типы изменений
- `Добавлено` — новые функции
- `Исправлено` — исправления багов
- `Безопасность` — изменения, связанные с безопасностью
- `Удалено` — удалённые функции