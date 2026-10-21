use crate::ansible::{adhoc, ops};
use crate::app::AnsibleApp;
use crate::models::*;
use crate::theme::*;
use egui::{Frame, Margin, RichText, Rounding};

impl AnsibleApp {
    pub fn spawn_raw(&mut self, hosts: String, module: String, args: String) {
        let (tx, rx) = std::sync::mpsc::channel();
        self.ops_rx = Some(rx);
        std::thread::spawn(move || {
            adhoc::run_raw(&hosts, &module, &args, true, tx);
        });
    }

    fn run_shell_sync(hosts: &str, cmd: &str) -> (bool, String) {
        let out = std::process::Command::new("ansible")
            .arg(hosts)
            .arg("-m")
            .arg("shell")
            .arg("-a")
            .arg(cmd)
            .arg("--become")
            .output();
        match out {
            Ok(o) => (
                o.status.success(),
                String::from_utf8_lossy(&o.stdout)
                    .lines()
                    .next()
                    .unwrap_or("")
                    .trim()
                    .to_string(),
            ),
            Err(e) => (false, format!("error: {}", e)),
        }
    }

    // === 🖥 DOMAIN ===
    pub fn show_domain(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().frame(Frame::none().inner_margin(Margin::same(14.0))).show(ctx, |ui| {
            ui.heading(RichText::new("🖥 Управление доменом").color(c_purple()));
            ui.label(RichText::new("Операции описаны как адаптеры: GUI генерирует команды для FreeIPA/Samba/realm и запускает их через Ansible.").color(c_tm()));
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.label("Хосты-контроллеры домена:");
                ui.text_edit_singleline(&mut self.ops.hosts);
            });
            ui.add_space(4.0);
            let cat = ops::domain_catalog();
            ui.horizontal(|ui| {
                ui.label("Операция:");
                egui::ComboBox::from_id_salt("dom_op").selected_text(&self.ops.domain_op).show_ui(ui, |ui| {
                    for (label, _, _, _) in &cat { ui.selectable_value(&mut self.ops.domain_op, (*label).to_string(), *label); }
                });
            });
            let cur = cat.iter().find(|(l, _, _, _)| *l == self.ops.domain_op.as_str());
            let (need_n, need_e) = cur.map(|c| (c.2, c.3)).unwrap_or((false, false));
            if need_n { ui.horizontal(|ui| { ui.label("Имя (пользователь/машина, через пробел для массовых):"); ui.text_edit_singleline(&mut self.ops.domain_name); }); }
            if need_e { ui.horizontal(|ui| { ui.label("Доп. (домен/группа):"); ui.text_edit_singleline(&mut self.ops.domain_extra); }); }
            ui.add_space(8.0);
            if ui.button("▶ Выполнить").clicked() {
                if let Some((_, cmd, _, _)) = cur {
                    let cmd = cmd.replace("{n}", &self.ops.domain_name).replace("{e}", &self.ops.domain_extra);
                    self.ops.ops_log.push(format!("[domain] {}", self.ops.domain_op));
                    self.spawn_raw(self.ops.hosts.clone(), "shell".into(), cmd);
                }
            }
            ui.add_space(8.0);
            ui.label(RichText::new("📜 Вывод:").strong());
            Frame::none().fill(c_bgd()).rounding(Rounding::same(8.0)).inner_margin(Margin::same(8.0)).show(ui, |ui| {
                egui::ScrollArea::vertical().max_height(300.0).stick_to_bottom(true).show(ui, |ui| {
                    for l in &self.ops.ops_log { ui.label(RichText::new(l).monospace().color(c_tx())); }
                });
            });
        });
    }

    // === 🧾 PROFILES ===
    pub fn show_profiles(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().frame(Frame::none().inner_margin(Margin::same(14.0))).show(ctx, |ui| {
            ui.heading(RichText::new("🧾 Конфигурации рабочих станций").color(c_purple()));
            ui.label(RichText::new("Готовые профили: пакеты + службы + безопасность. Генерируют playbook для push-модели.").color(c_tm()));
            ui.add_space(8.0);
            let profs = ops::profiles();
            egui::ComboBox::from_id_salt("prof_sel").selected_text(&self.ops.profile_sel).show_ui(ui, |ui| {
                for p in &profs { ui.selectable_value(&mut self.ops.profile_sel, p.name.to_string(), p.name); }
            });
            if let Some(p) = profs.iter().find(|p| p.name == self.ops.profile_sel.as_str()) {
                ui.add_space(8.0);
                ui.label(RichText::new(p.desc).color(c_tx()));
                ui.label(RichText::new(format!("Пакеты: {}", p.packages.join(", "))).color(c_cyan()));
                ui.label(RichText::new(format!("Службы: {}", p.services.join(", "))).color(c_green()));
                ui.label(RichText::new(format!("Hardening: {}", if p.hardening { "да" } else { "нет" })).color(c_orange()));
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button("📋 В Editor").clicked() { self.editor.content = ops::profile_yaml(p); self.tab = Tab::Editor; }
                    if ui.button("💾 Сохранить YAML").clicked() {
                        let yaml = ops::profile_yaml(p);
                        if let Some(path) = rfd::FileDialog::new().set_file_name("profile.yml").add_filter("YAML", &["yml"]).save_file() {
                            let _ = std::fs::write(&path, yaml);
                            self.toast(format!("💾 {}", path.display()));
                        }
                    }
                    if ui.button("▶ Deploy (push)").clicked() {
                        let yaml = ops::profile_yaml(p);
                        let tmp = std::env::temp_dir().join("profile_deploy.yml");
                        let _ = std::fs::write(&tmp, yaml);
                        self.playbook_path = tmp.to_string_lossy().into_owned();
                        self.start_run();
                    }
                });
            } else {
                ui.label("Выберите профиль из списка.");
            }
        });
    }

    // === 🔀 WORKFLOW ===
    pub fn show_workflow(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().frame(Frame::none().inner_margin(Margin::same(14.0))).show(ctx, |ui| {
            ui.heading(RichText::new("🔀 Workflow-конструктор").color(c_purple()));
            ui.label(RichText::new("Последовательные шаги с повторами, остановкой при ошибке и подтверждением опасных шагов.").color(c_tm()));
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.label("Хосты:"); ui.text_edit_singleline(&mut self.ops.hosts);
                if ui.button("➕ Шаг").clicked() { self.ops.wf_steps.push(WfStep { kind: "ping".into(), param: "".into(), retries: 1, stop_on_fail: true, confirm: false }); }
                if ui.button("🧩 Пример сценария").clicked() { self.ops.wf_steps = ops::wf_default(); }
            });
            ui.add_space(4.0);
            let mut remove: Vec<usize> = Vec::new();
            for (i, s) in self.ops.wf_steps.iter_mut().enumerate() {
                Frame::none().fill(c_surf()).rounding(Rounding::same(8.0)).inner_margin(Margin::same(8.0)).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(format!("{}.", i + 1)).strong());
                        egui::ComboBox::from_id_salt(format!("wf{}", i)).selected_text(ops::step_label(&s.kind)).show_ui(ui, |ui| {
                            for k in ["ping", "backup", "update", "restart", "check", "report", "custom"] {
                                ui.selectable_value(&mut s.kind, k.to_string(), ops::step_label(k));
                            }
                        });
                        ui.text_edit_singleline(&mut s.param);
                        ui.add(egui::DragValue::new(&mut s.retries).range(1..=5).prefix("повторы: "));
                        ui.checkbox(&mut s.stop_on_fail, "стоп при ошибке");
                        ui.checkbox(&mut s.confirm, "подтверждение");
                        if ui.button("🗑").clicked() { remove.push(i); }
                    });
                });
                ui.add_space(3.0);
            }
            for i in remove.into_iter().rev() { self.ops.wf_steps.remove(i); }
            ui.add_space(8.0);
            let has_confirm = self.ops.wf_steps.iter().any(|s| s.confirm);
            if ui.button("▶ Запустить workflow").clicked() {
                if has_confirm && !self.ops.wf_confirm { self.ops.wf_confirm = true; }
                else { self.run_workflow(); }
            }
            if self.ops.wf_confirm {
                egui::Window::new("⚠️ Подтверждение шагов").anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0]).show(ctx, |ui| {
                    ui.label("Следующие шаги требуют подтверждения:");
                    for s in &self.ops.wf_steps { if s.confirm { ui.label(format!("  • {}", ops::step_label(&s.kind))); } }
                    ui.horizontal(|ui| {
                        if ui.button("✅ Подтвердить и запустить").clicked() { self.ops.wf_confirm = false; self.run_workflow(); }
                        if ui.button("❌ Отмена").clicked() { self.ops.wf_confirm = false; }
                    });
                });
            }
            ui.add_space(8.0);
            ui.label(RichText::new("📜 Журнал:").strong());
            Frame::none().fill(c_bgd()).rounding(Rounding::same(8.0)).inner_margin(Margin::same(8.0)).show(ui, |ui| {
                egui::ScrollArea::vertical().max_height(250.0).stick_to_bottom(true).show(ui, |ui| {
                    for l in &self.ops.ops_log { ui.label(RichText::new(l).monospace().color(c_tx())); }
                });
            });
        });
    }

    fn run_workflow(&mut self) {
        let steps = self.ops.wf_steps.clone();
        let hosts = self.ops.hosts.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        self.ops_rx = Some(rx);
        self.ops.wf_running = true;
        std::thread::spawn(move || {
            for (i, s) in steps.iter().enumerate() {
                let (m, a) = ops::step_command(&s.kind, &s.param);
                let _ = tx.send(format!(
                    "── шаг {}: {} {}",
                    i + 1,
                    ops::step_label(&s.kind),
                    a
                ));
                let mut attempt = 0;
                loop {
                    attempt += 1;
                    let mut cmd = std::process::Command::new("ansible");
                    cmd.arg(&hosts).arg("-m").arg(m).arg("--become");
                    if !a.is_empty() {
                        cmd.arg("-a").arg(&a);
                    }
                    match cmd.output() {
                        Ok(o) => {
                            for l in String::from_utf8_lossy(&o.stdout).lines() {
                                let _ = tx.send(l.to_string());
                            }
                            if o.status.success() {
                                break;
                            }
                            if attempt >= s.retries {
                                if s.stop_on_fail {
                                    let _ = tx.send("⛔ Остановка: критическая ошибка".into());
                                    return;
                                }
                                break;
                            }
                            let _ = tx.send(format!("↻ повтор {}...", attempt + 1));
                        }
                        Err(e) => {
                            let _ = tx.send(format!("error: {}", e));
                            break;
                        }
                    }
                }
            }
            let _ = tx.send("✅ Workflow завершён".into());
        });
    }

    // === ⚡ OPS CENTER ===
    pub fn show_opscenter(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().frame(Frame::none().inner_margin(Margin::same(14.0))).show(ctx, |ui| {
            ui.heading(RichText::new("⚡ Центр операций").color(c_purple()));
            ui.label(RichText::new("Готовые действия без playbook. 3 уровня: безопасные — сразу; подтверждение; опасные — с причиной.").color(c_tm()));
            ui.add_space(8.0);
            ui.horizontal(|ui| { ui.label("Хосты:"); ui.text_edit_singleline(&mut self.ops.hosts); ui.label("Параметр:"); ui.text_edit_singleline(&mut self.ops.op_param); });
            ui.add_space(6.0);
            egui::ScrollArea::vertical().max_height(320.0).show(ui, |ui| {
                for item in ops::ops_catalog() {
                    let (icon, color) = match item.safety {
                        ops::Safety::Safe => ("🟢", c_green()),
                        ops::Safety::Confirm => ("🟡", c_orange()),
                        ops::Safety::Danger => ("🔴", c_red()),
                    };
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(icon).color(color));
                        ui.label(RichText::new(item.name).color(c_tx()));
                        if ui.small_button("▶").clicked() {
                            let args = item.args.replace("{p}", &self.ops.op_param);
                            match item.safety {
                                ops::Safety::Safe => self.spawn_raw(self.ops.hosts.clone(), item.module.into(), args),
                                ops::Safety::Confirm => self.ops.pending = Some(PendingOp { name: item.name.into(), hosts: self.ops.hosts.clone(), module: item.module.into(), args, safety: "confirm".into(), reason: String::new() }),
                                ops::Safety::Danger => self.ops.pending = Some(PendingOp { name: item.name.into(), hosts: self.ops.hosts.clone(), module: item.module.into(), args, safety: "danger".into(), reason: String::new() }),
                            }
                        }
                    });
                }
            });
            // Окно подтверждения
            let mut run_now: Option<(String, String, String)> = None;
            let mut cancel = false;
            if let Some(p) = &mut self.ops.pending {
                egui::Window::new("⚠️ Подтверждение операции").anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0]).show(ctx, |ui| {
                    ui.label(RichText::new(format!("Операция: {}", p.name)).strong());
                    ui.label(format!("Хосты: {}", p.hosts));
                    ui.label(format!("Команда: {}", p.args));
                    if p.safety == "danger" {
                        ui.label(RichText::new("⚠️ ОПАСНО! Укажите причину:").color(c_red()));
                        ui.text_edit_singleline(&mut p.reason);
                    }
                    ui.horizontal(|ui| {
                        let can = p.safety != "danger" || !p.reason.is_empty();
                        if ui.add_enabled(can, egui::Button::new("✅ Выполнить")).clicked() { run_now = Some((p.hosts.clone(), p.module.clone(), p.args.clone())); }
                        if ui.button("❌ Отмена").clicked() { cancel = true; }
                    });
                });
            }
            if cancel { self.ops.pending = None; }
            if let Some((h, m, a)) = run_now { self.ops.pending = None; self.spawn_raw(h, m, a); }
            ui.add_space(8.0);
            ui.label(RichText::new("📜 Консоль:").strong());
            Frame::none().fill(c_bgd()).rounding(Rounding::same(8.0)).inner_margin(Margin::same(8.0)).show(ui, |ui| {
                egui::ScrollArea::vertical().max_height(220.0).stick_to_bottom(true).show(ui, |ui| {
                    for l in &self.ops.ops_log { ui.label(RichText::new(l).monospace().color(c_tx())); }
                });
            });
        });
    }

    // === 🛡 COMPLIANCE ===
    pub fn show_compliance(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default()
            .frame(Frame::none().inner_margin(Margin::same(14.0)))
            .show(ctx, |ui| {
                ui.heading(RichText::new("🛡 Drift и Compliance").color(c_purple()));
                ui.label(
                    RichText::new(
                        "Audit — только проверка и отчёт. Enforce — автоисправление отклонений.",
                    )
                    .color(c_tm()),
                );
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.label("Хосты:");
                    ui.text_edit_singleline(&mut self.ops.hosts);
                    ui.checkbox(&mut self.ops.comp_enforce, "Enforce (автоисправление)");
                    if ui.button("➕ Проверка").clicked() {
                        self.ops.comp_checks.push(CompCheck {
                            kind: "service".into(),
                            name: "sshd".into(),
                        });
                    }
                    if ui.button("▶ Запустить").clicked() {
                        self.run_compliance();
                    }
                });
                ui.add_space(4.0);
                let mut remove: Vec<usize> = Vec::new();
                for (i, c) in self.ops.comp_checks.iter_mut().enumerate() {
                    ui.horizontal(|ui| {
                        egui::ComboBox::from_id_salt(format!("ck{}", i))
                            .selected_text(&c.kind)
                            .show_ui(ui, |ui| {
                                for k in ["service", "package", "port", "file"] {
                                    ui.selectable_value(&mut c.kind, k.to_string(), k);
                                }
                            });
                        ui.text_edit_singleline(&mut c.name);
                        if ui.button("🗑").clicked() {
                            remove.push(i);
                        }
                    });
                }
                for i in remove.into_iter().rev() {
                    self.ops.comp_checks.remove(i);
                }
                ui.add_space(8.0);
                ui.label(RichText::new("Результаты:").strong());
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for r in &self.ops.comp_results {
                        ui.horizontal(|ui| {
                            ui.colored_label(
                                if r.ok { c_green() } else { c_red() },
                                if r.ok { "✓" } else { "✗" },
                            );
                            ui.label(RichText::new(&r.check).color(c_tx()));
                            ui.label(
                                RichText::new(format!(
                                    "ожидалось: {} | факт: {}",
                                    r.expected, r.actual
                                ))
                                .color(c_tm()),
                            );
                            if r.fixed {
                                ui.colored_label(c_blue(), "исправлено");
                            }
                        });
                    }
                });
            });
    }

    fn run_compliance(&mut self) {
        let checks = self.ops.comp_checks.clone();
        let hosts = self.ops.hosts.clone();
        let enforce = self.ops.comp_enforce;
        let (tx, rx) = std::sync::mpsc::channel();
        self.comp_rx = Some(rx);
        std::thread::spawn(move || {
            for c in checks {
                let audit = ops::comp_audit_cmd(&c);
                let (ok, actual) = Self::run_shell_sync(&hosts, &audit);
                let passed = ops::comp_ok(&c, ok, &actual);
                let mut fixed = false;
                if enforce && !passed {
                    let fix = ops::comp_fix(&c);
                    let (fok, _) = Self::run_shell_sync(&hosts, &fix);
                    fixed = fok;
                }
                let _ = tx.send(ComplianceResult {
                    check: format!("{}:{}", c.kind, c.name),
                    expected: ops::comp_expected(&c),
                    actual,
                    ok: passed,
                    fixed,
                });
            }
        });
    }
}
