use crate::ansible::adhoc::AdHocRunner;
use crate::ansible::graph::draw_graph;
use crate::ansible::inventory::parse_inventory;
use crate::ansible::library::get_builtin_library;
use crate::ansible::{diagnostics, gitops, report, rpmrepo, vault, webapi, zabbix};
use crate::ansible::{diff as diff_mod, roles as roles_mod};
use crate::app::AnsibleApp;
use crate::models::*;
use crate::theme::*;
use base64::{engine::general_purpose, Engine};
use egui::{Align, Color32, Frame, Layout, Margin, RichText, Rounding, Stroke, Vec2};
use std::path::PathBuf;
use tokio::sync::mpsc;

fn badge(ui: &mut egui::Ui, label: &str, count: usize, color: Color32) {
    Frame::none()
        .fill(color.gamma_multiply(0.15))
        .rounding(Rounding::same(6.0))
        .inner_margin(Margin::symmetric(10.0, 5.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(label).color(color).strong().size(12.0));
                ui.label(RichText::new(count.to_string()).color(c_tx()).strong());
            });
        });
}

impl AnsibleApp {
    pub fn show_run(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("sidebar")
            .default_width(330.0)
            .frame(Frame::none().inner_margin(Margin::same(14.0)))
            .show(ctx, |ui| {
                ui.heading(RichText::new("📂 Configuration").color(c_purple()));
                ui.add_space(8.0);
                ui.label(RichText::new("Inventory").color(c_tm()));
                ui.horizontal(|ui| {
                    ui.text_edit_singleline(&mut self.inventory_path);
                    if ui.button("📂").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter(
                                "Ansible Inventory",
                                &["ini", "yml", "yaml", "json", "toml"],
                            )
                            .pick_file()
                        {
                            self.inventory_path = path.to_string_lossy().into_owned();
                            self.error_message = None;
                        }
                    }
                });
                ui.label(RichText::new("Playbook").color(c_tm()));
                ui.horizontal(|ui| {
                    ui.text_edit_singleline(&mut self.playbook_path);
                    if ui.button("📂").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("YAML Files", &["yml", "yaml"])
                            .pick_file()
                        {
                            self.playbook_path = path.to_string_lossy().into_owned();
                            self.error_message = None;
                        }
                    }
                });
                ui.label(RichText::new("Extra vars (-e)").color(c_tm()));
                ui.text_edit_singleline(&mut self.extra_vars);
                if let Some(err) = &self.error_message {
                    ui.add_space(4.0);
                    ui.colored_label(c_red(), RichText::new(format!("⚠️ {}", err)).strong());
                }
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button("📥 Load inv").clicked() {
                        match parse_inventory(std::path::Path::new(&self.inventory_path)) {
                            Ok(h) => {
                                self.hosts = h;
                                self.error_message = None;
                            }
                            Err(e) => {
                                self.hosts.clear();
                                self.error_message = Some(format!("Ошибка парсинга: {}", e));
                            }
                        }
                    }
                    if ui.button("📝 Load pb").clicked() {
                        self.load_playbook_graph();
                    }
                });
                ui.add_space(12.0);
                ui.separator();
                ui.add_space(8.0);
                ui.heading(RichText::new("🔐 SSH / Vault").color(c_purple()));
                ui.label(RichText::new("Private key").color(c_tm()));
                ui.horizontal(|ui| {
                    let mut kp = self
                        .ssh
                        .private_key
                        .as_ref()
                        .map(|p| p.to_string_lossy().into_owned())
                        .unwrap_or_default();
                    ui.text_edit_singleline(&mut kp);
                    if ui.button("📂").clicked() {
                        if let Some(path) = rfd::FileDialog::new().pick_file() {
                            kp = path.to_string_lossy().into_owned();
                        }
                    }
                    self.ssh.private_key = if kp.is_empty() {
                        None
                    } else {
                        Some(PathBuf::from(kp))
                    };
                });
                ui.checkbox(&mut self.ssh.use_become, "Use become (sudo)");
                if self.ssh.use_become {
                    ui.text_edit_singleline(&mut self.ssh.become_user);
                }
                ui.checkbox(&mut self.ssh.ask_vault_pass, "Ask vault password");
                ui.add(
                    egui::DragValue::new(&mut self.ssh.forks)
                        .range(1..=100)
                        .prefix("forks: "),
                );
                ui.add_space(16.0);
                let running = matches!(self.session.lock().unwrap().state, RunState::Running);
                if ui
                    .add_enabled(
                        !running,
                        egui::Button::new(
                            RichText::new(if running {
                                "⏳ Running..."
                            } else {
                                "▶  Run Playbook"
                            })
                            .color(Color32::WHITE)
                            .strong(),
                        )
                        .fill(c_green())
                        .min_size(Vec2::new(ui.available_width(), 42.0))
                        .rounding(Rounding::same(10.0)),
                    )
                    .clicked()
                {
                    self.start_run();
                }
                if running
                    && ui
                        .add_enabled(
                            running,
                            egui::Button::new(
                                RichText::new("⏹ Остановить").color(Color32::WHITE).strong(),
                            )
                            .fill(c_red())
                            .min_size(Vec2::new(ui.available_width(), 32.0))
                            .rounding(Rounding::same(10.0)),
                        )
                        .clicked()
                {
                    self.stop_run();
                }
                ui.add_space(16.0);
                ui.heading(RichText::new("🖥 Hosts").color(c_purple()));
                egui::ScrollArea::vertical().show(ui, |ui| {
                    if self.hosts.is_empty() {
                        ui.colored_label(c_tm(), "No hosts loaded");
                    }
                    for h in &self.hosts {
                        Frame::none()
                            .fill(c_surf())
                            .rounding(Rounding::same(8.0))
                            .inner_margin(Margin::same(9.0))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new(&h.name).strong().color(c_tx()));
                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                        ui.colored_label(c_cyan(), &h.group);
                                    });
                                });
                                if let Some(ah) = &h.ansible_host {
                                    ui.label(RichText::new(format!("→ {ah}")).color(c_tm()));
                                }
                            });
                        ui.add_space(4.0);
                    }
                });
            });
        egui::CentralPanel::default()
            .frame(Frame::none().inner_margin(Margin::same(14.0)))
            .show(ctx, |ui| {
                ui.heading(RichText::new("📜 Execution Log").color(c_purple()));
                ui.add_space(6.0);
                let sess = self.session.lock().unwrap();
                ui.horizontal(|ui| {
                    badge(ui, "OK", sess.summary.ok, c_green());
                    badge(ui, "CHANGED", sess.summary.changed, c_blue());
                    badge(ui, "FAILED", sess.summary.failed, c_red());
                    badge(ui, "SKIPPED", sess.summary.skipped, c_tm());
                });
                ui.add_space(8.0);
                Frame::none()
                    .fill(c_bgd())
                    .rounding(Rounding::same(10.0))
                    .stroke(Stroke::new(1.0_f32, c_surfh()))
                    .inner_margin(Margin::same(10.0))
                    .show(ui, |ui| {
                        egui::ScrollArea::vertical()
                            .stick_to_bottom(self.auto_scroll)
                            .show(ui, |ui| {
                                if sess.events.is_empty() {
                                    ui.colored_label(c_tm(), "Press ▶ Run Playbook to start...");
                                }
                                for ev in &sess.events {
                                    let (icon, color) = match ev.status {
                                        TaskStatus::Ok => ("✓", c_green()),
                                        TaskStatus::Changed => ("↻", c_blue()),
                                        TaskStatus::Failed => ("✗", c_red()),
                                        TaskStatus::Skipped => ("⊘", c_tm()),
                                        TaskStatus::Running => ("●", c_orange()),
                                        TaskStatus::Pending => ("○", c_tm()),
                                    };
                                    ui.horizontal(|ui| {
                                        ui.label(RichText::new(icon).color(color).strong());
                                        ui.label(
                                            RichText::new(format!("[{}]", ev.host))
                                                .color(c_purple()),
                                        );
                                        ui.label(RichText::new(&ev.output).color(color));
                                    });
                                }
                            });
                    });
                drop(sess);
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.auto_scroll, "Auto-scroll");
                    if ui.button("🧹 Clear").clicked() {
                        if let Ok(mut s) = self.session.lock() {
                            s.events.clear();
                            s.recompute();
                        }
                    }
                });
            });
    }

    pub fn show_editor(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default()
            .frame(Frame::none().inner_margin(Margin::same(14.0)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.heading(RichText::new("📝 Playbook Editor").color(c_purple()));
                    if let Some(p) = &self.editor.path {
                        ui.label(RichText::new(p.to_string_lossy()).color(c_tm()));
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.button("💾 Save").clicked() {
                            if let Err(e) = self.editor.save() {
                                self.error_message = Some(format!("Ошибка сохранения: {}", e));
                            } else {
                                self.error_message = None;
                            }
                        }
                    });
                });
                ui.add_space(8.0);
                self.editor.show(ui);
            });
    }

    pub fn show_constructor(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("constructor_side")
            .default_width(350.0)
            .frame(Frame::none().inner_margin(Margin::same(14.0)))
            .show(ctx, |ui| {
                ui.heading(RichText::new("🏗️ Конструктор Playbook").color(c_purple()));
                ui.add_space(12.0);
                ui.label(RichText::new("⚙️ PLAY настройки").color(c_blue()).strong());
                ui.add_space(8.0);
                ui.label(RichText::new("Name").color(c_tm()));
                ui.text_edit_singleline(&mut self.constructor_play.name);
                ui.label(RichText::new("Hosts").color(c_tm()));
                ui.text_edit_singleline(&mut self.constructor_play.hosts);
                ui.checkbox(&mut self.constructor_play.use_become, "Use become (sudo)");
                ui.checkbox(&mut self.constructor_play.gather_facts, "Gather facts");
                ui.add_space(16.0);
                ui.separator();
                ui.add_space(8.0);
                ui.label(RichText::new("📦 Модули").color(c_blue()).strong());
                ui.add_space(4.0);
                egui::ScrollArea::vertical()
                    .max_height(ui.available_height() - 100.0)
                    .show(ui, |ui| {
                        for (category, modules) in ModuleCatalog::categories() {
                            ui.collapsing(category, |ui| {
                                for module in modules {
                                    if ui.button(module).clicked() {
                                        let mut task = ConstructorTask {
                                            module: module.to_string(),
                                            name: format!("{} task", module),
                                            ..Default::default()
                                        };
                                        for param in ModuleCatalog::common_params(module) {
                                            task.params.insert(param.to_string(), String::new());
                                        }
                                        self.constructor_play.tasks.push(task);
                                    }
                                }
                            });
                        }
                    });
            });
        egui::CentralPanel::default()
            .frame(Frame::none().inner_margin(Margin::same(14.0)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.heading(RichText::new("📋 Задачи").color(c_purple()));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.button("👁 Предпросмотр YAML").clicked() {
                            self.show_yaml_preview = !self.show_yaml_preview;
                        }
                        if ui.button("💾 Сохранить как playbook").clicked() {
                            self.save_constructor_to_file();
                        }
                    });
                });
                ui.add_space(8.0);
                if self.show_yaml_preview {
                    ui.collapsing("📄 YAML Preview", |ui| {
                        let yaml = self.constructor_to_yaml();
                        egui::ScrollArea::vertical()
                            .max_height(300.0)
                            .show(ui, |ui| {
                                ui.add(
                                    egui::TextEdit::multiline(&mut { yaml.clone() })
                                        .font(egui::FontId::monospace(12.0))
                                        .desired_width(f32::INFINITY),
                                );
                            });
                    });
                    ui.add_space(8.0);
                }
                egui::ScrollArea::vertical().show(ui, |ui| {
                    if self.constructor_play.tasks.is_empty() {
                        ui.colored_label(c_tm(), "Добавьте задачи из панели слева →");
                    }
                    let task_count = self.constructor_play.tasks.len();
                    for idx in 0..task_count {
                        let frame = Frame::none()
                            .fill(c_surf())
                            .rounding(Rounding::same(10.0))
                            .inner_margin(Margin::same(12.0));
                        let task_module = self.constructor_play.tasks[idx].module.clone();
                        let task_name = self.constructor_play.tasks[idx].name.clone();
                        let task_when = self.constructor_play.tasks[idx].when_condition.clone();
                        let task_register = self.constructor_play.tasks[idx].register_var.clone();
                        let task_notify = self.constructor_play.tasks[idx].notify_handler.clone();
                        frame.show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new(format!("{}. {}", idx + 1, task_module))
                                        .color(c_blue())
                                        .strong(),
                                );
                                let mut name = task_name;
                                ui.text_edit_singleline(&mut name);
                                self.constructor_play.tasks[idx].name = name;
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    if ui.button("🗑️").clicked() {
                                        self.constructor_play.tasks.remove(idx);
                                    }
                                    if idx > 0 && ui.button("↑").clicked() {
                                        self.constructor_play.tasks.swap(idx, idx - 1);
                                    }
                                    if idx < self.constructor_play.tasks.len() - 1
                                        && ui.button("↓").clicked()
                                    {
                                        self.constructor_play.tasks.swap(idx, idx + 1);
                                    }
                                });
                            });
                            ui.add_space(4.0);
                            ui.label(RichText::new("Параметры модуля:").color(c_tm()));
                            let module = self.constructor_play.tasks[idx].module.clone();
                            let params_to_add = ModuleCatalog::common_params(&module);
                            self.constructor_play.tasks[idx]
                                .params
                                .retain(|k, _| params_to_add.contains(&k.as_str()));
                            for param in params_to_add {
                                self.constructor_play.tasks[idx]
                                    .params
                                    .entry(param.to_string())
                                    .or_default();
                            }
                            let params: Vec<(String, String)> = self.constructor_play.tasks[idx]
                                .params
                                .clone()
                                .into_iter()
                                .collect();
                            for (key, value) in params {
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new(format!("{}:", key)).color(c_green()));
                                    let mut val = value;
                                    ui.text_edit_singleline(&mut val);
                                    self.constructor_play.tasks[idx].params.insert(key, val);
                                });
                            }
                            let (module_title, module_desc, module_params) =
                                ModuleReference::get_module_info(&module);
                            ui.collapsing(
                                format!("📖 Справка: {} — {}", module, module_title),
                                |ui| {
                                    ui.label(RichText::new(module_desc).color(c_tm()));
                                    ui.add_space(4.0);
                                    ui.label(RichText::new("Доступные параметры:").strong());
                                    for param in module_params {
                                        ui.label(format!("  • {}", param));
                                    }
                                },
                            );
                            ui.add_space(4.0);
                            ui.horizontal(|ui| {
                                ui.checkbox(
                                    &mut self.constructor_play.tasks[idx].use_become,
                                    "become",
                                );
                                ui.checkbox(
                                    &mut self.constructor_play.tasks[idx].ignore_errors,
                                    "ignore_errors",
                                );
                            });
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("when:").color(c_tm()));
                                let mut when = task_when;
                                ui.text_edit_singleline(&mut when);
                                self.constructor_play.tasks[idx].when_condition = when;
                            });
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("register:").color(c_tm()));
                                let mut reg = task_register;
                                ui.text_edit_singleline(&mut reg);
                                self.constructor_play.tasks[idx].register_var = reg;
                            });
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("notify:").color(c_tm()));
                                let mut notif = task_notify;
                                ui.text_edit_singleline(&mut notif);
                                self.constructor_play.tasks[idx].notify_handler = notif;
                            });
                        });
                        ui.add_space(6.0);
                    }
                });
            });
    }

    pub fn show_admin(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().frame(Frame::none().inner_margin(Margin::same(14.0))).show(ctx, |ui| {
            ui.heading(RichText::new("🛠️ Административные модули").color(c_purple()));
            ui.add_space(8.0);
            ui.collapsing("📖 Как использовать админ-модули?", |ui| {
                ui.label(RichText::new("Быстрое выполнение задач управления системами через GUI.").color(c_tm()));
                ui.label("• Выберите модуль → действие → имя → 'Выполнить задачу'");
            });
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(RichText::new("Целевые хосты").color(c_tm()));
                    ui.text_edit_singleline(&mut self.admin_action.target_hosts);
                    ui.label(RichText::new("Модуль").color(c_tm()));
                    egui::ComboBox::from_id_salt("admin_mod").selected_text(self.admin_action.module_type_to_str()).show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.admin_action.module_type, AdminModuleType::Package, "Package (Установка ПО)");
                        ui.selectable_value(&mut self.admin_action.module_type, AdminModuleType::Service, "Service (Управление службами)");
                        ui.selectable_value(&mut self.admin_action.module_type, AdminModuleType::User, "User (Пользователи)");
                        ui.selectable_value(&mut self.admin_action.module_type, AdminModuleType::File, "File (Файлы/Конфиги)");
                    });
                    let module_hint = match self.admin_action.module_type {
                        AdminModuleType::Package => "Действия: install, remove, update. Имя: название пакета (nginx, httpd, vim)",
                        AdminModuleType::Service => "Действия: started, stopped, restarted, enabled, disabled. Имя: название сервиса (sshd, nginx, docker)",
                        AdminModuleType::User => "Действия: present, absent. Имя: имя пользователя",
                        AdminModuleType::File => "Действия: present, absent, directory. Имя: путь к файлу (/etc/nginx/nginx.conf)",
                    };
                    ui.label(RichText::new(module_hint).color(c_blue()).small());
                    ui.label(RichText::new("Действие").color(c_tm()));
                    ui.text_edit_singleline(&mut self.admin_action.action);
                    ui.label(RichText::new("Имя").color(c_tm()));
                    ui.text_edit_singleline(&mut self.admin_action.name);
                    ui.label(RichText::new("Доп. аргументы").color(c_tm()));
                    ui.text_edit_singleline(&mut self.admin_action.extra_args);
                    ui.checkbox(&mut self.admin_action.use_become, "Use become (sudo)");
                    ui.checkbox(&mut self.admin_action.check_mode, "Check mode (Dry Run)");
                    ui.add_space(16.0);
                    if ui.button("▶ Выполнить задачу").clicked() {
                        let (tx, rx) = mpsc::channel(256);
                        self.job_rx = Some(rx);
                        let action = self.admin_action.clone();
                        let inv = self.inventory_path.clone();
                        tokio::spawn(async move { let _ = AdHocRunner::run_admin_action(action, &inv, tx).await; });
                    }
                });
                ui.vertical(|ui| {
                    ui.label(RichText::new("📊 Активные задачи").color(c_blue()));
                    egui::ScrollArea::vertical().max_height(400.0).show(ui, |ui| {
                        for job in &self.background_jobs {
                            let color = match job.status.as_str() { "Success" => c_green(), "Failed" => c_red(), _ => c_orange() };
                            Frame::none().fill(c_surf()).rounding(Rounding::same(8.0)).inner_margin(Margin::same(9.0)).show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new(&job.name).strong().color(c_tx()));
                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| { ui.colored_label(color, &job.status); });
                                });
                                ui.add(egui::ProgressBar::new(job.progress).show_percentage());
                                ui.collapsing("📋 Вывод", |ui| { for line in &job.output { ui.label(RichText::new(line).monospace().small().color(c_tx())); } });
                            });
                            ui.add_space(4.0);
                        }
                    });
                });
            });
        });
    }

    pub fn show_adhoc(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().frame(Frame::none().inner_margin(Margin::same(14.0))).show(ctx, |ui| {
            ui.heading(RichText::new("💻 Ad-Hoc & REPL Консоль").color(c_purple()));
            ui.add_space(8.0);
            ui.collapsing("📖 Что это и зачем?", |ui| {
                ui.label(RichText::new("Ad-Hoc — разовые команды Ansible БЕЗ создания playbook. Нужны для быстрых операций: проверить связь, перезапустить сервис, поставить пакет, собрать факт.").color(c_tm()));
                ui.label(RichText::new("Формат: ansible <хосты> -m <модуль> -a '<аргументы>'").color(c_blue()).monospace());
                ui.label("• ansible all -m ping — проверить связь со всеми");
                ui.label("• ansible webservers -m service -a 'name=nginx state=restarted'");
                ui.label("• ansible all -m dnf -a 'name=vim state=present'");
            });
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("Хосты:").color(c_tm()));
                ui.text_edit_singleline(&mut self.admin_action.target_hosts);
                ui.label(RichText::new("Модуль:").color(c_tm()));
                let modules = vec!["ping", "shell", "command", "apt", "yum", "dnf", "service", "user", "copy", "template", "file", "debug", "setup"];
                egui::ComboBox::from_id_salt("adhoc_mod").selected_text(&self.adhoc_module).show_ui(ui, |ui| { for m in modules { ui.selectable_value(&mut self.adhoc_module, m.to_string(), m); } });
                ui.label(RichText::new("Аргументы:").color(c_tm()));
                ui.text_edit_singleline(&mut self.adhoc_args);
                if ui.button("▶ Выполнить").clicked() {
                    self.spawn_raw(self.admin_action.target_hosts.clone(), self.adhoc_module.clone(), self.adhoc_args.clone());
                    self.toast("💻 Ad-Hoc запущена".into());
                }
            });
            ui.add_space(12.0);
            ui.collapsing("📚 Справочник модулей", |ui| {
                egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                    for module in vec!["ping", "apt", "yum", "dnf", "service", "user", "copy", "file", "command", "shell", "debug", "setup"] {
                        let (title, desc, params) = ModuleReference::get_module_info(module);
                        ui.collapsing(format!("{} — {}", module, title), |ui| {
                            ui.label(RichText::new(desc).color(c_tm()));
                            if !params.is_empty() {
                                ui.label(RichText::new("Параметры:").strong());
                                for param in params { ui.label(format!("  • {}", param)); }
                            }
                        });
                    }
                });
            });
            ui.add_space(12.0);
            ui.label(RichText::new("📜 Вывод:").strong());
            Frame::none().fill(c_bgd()).rounding(Rounding::same(10.0)).inner_margin(Margin::same(10.0)).show(ui, |ui| {
                egui::ScrollArea::vertical().stick_to_bottom(true).show(ui, |ui| {
                    for line in &self.ops.ops_log { ui.label(RichText::new(line).color(c_tx()).monospace()); }
                });
            });
        });
    }

    pub fn show_library(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default()
            .frame(Frame::none().inner_margin(Margin::same(14.0)))
            .show(ctx, |ui| {
                ui.heading(RichText::new("📚 Библиотека готовых решений").color(c_purple()));
                ui.add_space(8.0);
                ui.collapsing(
                    "📖 Как использовать библиотеку?",
                    |ui| {
                        ui.label(
                            RichText::new("Готовые playbook'и для типичных задач.").color(c_tm()),
                        );
                        ui.label("• '📋 Копировать в Editor' — отредактировать");
                        ui.label("• '▶ Deploy' — сохранить во временный файл и сразу запустить");
                    },
                );
                ui.add_space(12.0);
                for pb in get_builtin_library() {
                    Frame::none()
                        .fill(c_surf())
                        .rounding(Rounding::same(10.0))
                        .inner_margin(Margin::same(14.0))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.vertical(|ui| {
                                    ui.label(
                                        RichText::new(&pb.name).strong().size(16.0).color(c_tx()),
                                    );
                                    ui.label(RichText::new(&pb.category).color(c_cyan()));
                                    ui.label(RichText::new(&pb.description).color(c_tm()));
                                });
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    if ui.button("📋 Копировать в Editor").clicked() {
                                        self.editor.content = pb.yaml_content.clone();
                                        self.tab = Tab::Editor;
                                    }
                                    if ui.button("▶ Deploy").clicked() {
                                        let tmp = std::env::temp_dir().join(format!(
                                            "lib_deploy_{}.yml",
                                            pb.id.chars().take(8).collect::<String>()
                                        ));
                                        if std::fs::write(&tmp, &pb.yaml_content).is_ok() {
                                            self.playbook_path = tmp.to_string_lossy().into_owned();
                                            self.start_run();
                                            self.toast(format!("🚀 Запуск: {}", pb.name));
                                        }
                                    }
                                });
                            });
                        });
                    ui.add_space(8.0);
                }
            });
    }

    pub fn show_graph(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default()
            .frame(Frame::none().inner_margin(Margin::same(14.0)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.heading(RichText::new("🕸 Task Dependency Graph").color(c_purple()));
                    if let Some(g) = &self.graph {
                        if g.order_changed {
                            ui.colored_label(c_orange(), "⚠️ Порядок изменён");
                            if ui.button("💾 Сохранить в playbook").clicked() {
                                self.save_playbook_order();
                            }
                        }
                    }
                });
                ui.add_space(8.0);
                if let Some(g) = &mut self.graph {
                    ui.label(
                        RichText::new("💡 Перетащите узлы мышью, чтобы изменить порядок задач")
                            .color(c_tm()),
                    );
                    ui.label(
                        RichText::new(format!(
                            "Nodes: {} | Edges: {}",
                            g.graph.node_count(),
                            g.graph.edge_count()
                        ))
                        .color(c_tm()),
                    );
                    ui.add_space(8.0);
                    Frame::none()
                        .fill(c_bgd())
                        .rounding(Rounding::same(10.0))
                        .inner_margin(Margin::same(12.0))
                        .show(ui, |ui| {
                            if let Some(pb) = &mut self.playbook_data {
                                draw_graph(ui, g, pb);
                            } else {
                                ui.colored_label(c_tm(), "Нет данных плейбука");
                            }
                        });
                    ui.add_space(8.0);
                    ui.collapsing("📋 Текущий порядок задач", |ui| {
                        if let Some(pb) = &self.playbook_data {
                            for (i, task) in pb.tasks.iter().enumerate() {
                                ui.label(format!("{}. {}", i + 1, task.name));
                            }
                        }
                    });
                } else {
                    ui.colored_label(c_tm(), "Load a playbook first (Run tab → 📝 Load pb)");
                }
            });
    }

    pub fn show_terraform(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("tf_side")
            .default_width(290.0)
            .frame(Frame::none().inner_margin(Margin::same(14.0)))
            .show(ctx, |ui| {
                ui.heading(RichText::new("🌍 Terraform").color(c_purple()));
                ui.add_space(8.0);
                let mut wd = self.tf.workdir.to_string_lossy().into_owned();
                ui.label(RichText::new("Working directory").color(c_tm()));
                ui.horizontal(|ui| {
                    ui.text_edit_singleline(&mut wd);
                    if ui.button("📂").clicked() {
                        if let Some(path) = rfd::FileDialog::new().pick_folder() {
                            wd = path.to_string_lossy().into_owned();
                        }
                    }
                });
                self.tf.workdir = PathBuf::from(wd);
                ui.checkbox(&mut self.tf.auto_approve, "Auto-approve");
                ui.add_space(12.0);
                ui.horizontal_wrapped(|ui| {
                    if ui.button("🔧 init").clicked() {
                        self.start_tf("init");
                    }
                    if ui.button("📋 plan").clicked() {
                        self.start_tf("plan");
                    }
                    if ui.button("▶ apply").clicked() {
                        self.start_tf("apply");
                    }
                    if ui.button("💣 destroy").clicked() {
                        self.start_tf("destroy");
                    }
                });
            });
        egui::CentralPanel::default()
            .frame(Frame::none().inner_margin(Margin::same(14.0)))
            .show(ctx, |ui| {
                ui.heading(RichText::new("📜 Terraform Output").color(c_purple()));
                ui.add_space(8.0);
                Frame::none()
                    .fill(c_bgd())
                    .rounding(Rounding::same(10.0))
                    .stroke(Stroke::new(1.0_f32, c_surfh()))
                    .inner_margin(Margin::same(10.0))
                    .show(ui, |ui| {
                        egui::ScrollArea::vertical()
                            .stick_to_bottom(true)
                            .show(ui, |ui| {
                                if self.tf_log.is_empty() {
                                    ui.colored_label(c_tm(), "Run terraform command...");
                                }
                                for ev in &self.tf_log {
                                    let c = if ev.is_error { c_red() } else { c_tx() };
                                    ui.label(RichText::new(&ev.line).color(c).monospace());
                                }
                            });
                    });
            });
    }

    pub fn show_tools(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().frame(Frame::none().inner_margin(Margin::same(14.0))).show(ctx, |ui| {
            ui.heading(RichText::new("🧰 Инструменты").color(c_purple()));
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.collapsing("🩺 Диагностика (ping / ssh / порт)", |ui| {
                    ui.label(RichText::new("Проверяет каждый хост из inventory: доступен ли по ping, открыт ли SSH(22) и ваш порт. Работает в фоне.").color(c_tm()));
                    ui.horizontal(|ui| {
                        ui.label("Порт:"); ui.text_edit_singleline(&mut self.diag_port);
                        if ui.button("▶ Проверить все хосты").clicked() {
                            if self.hosts.is_empty() {
                                if let Ok(h) = parse_inventory(std::path::Path::new(&self.inventory_path)) { self.hosts = h; }
                            }
                            if self.hosts.is_empty() {
                                self.toast("⚠️ Нет хостов: загрузите inventory".into());
                            } else {
                                let hosts: Vec<(String, u16)> = self.hosts.iter()
                                    .map(|h| (h.ansible_host.clone().unwrap_or_else(|| h.name.clone()), self.diag_port.parse().unwrap_or(80)))
                                    .collect();
                                let (tx, rx) = std::sync::mpsc::channel();
                                self.diag_rx = Some(rx);
                                std::thread::spawn(move || {
                                    let res: Vec<DiagResult> = hosts.iter().map(|(h, p)| diagnostics::check_host(h, *p)).collect();
                                    let _ = tx.send(res);
                                });
                                self.toast("🩺 Диагностика запущена...".into());
                            }
                        }
                    });
                    for r in &self.diag_results {
                        ui.horizontal(|ui| {
                            ui.colored_label(if r.ping_ok { c_green() } else { c_red() }, if r.ping_ok { "● ping" } else { "○ ping" });
                            ui.colored_label(if r.ssh_ok { c_green() } else { c_red() }, if r.ssh_ok { "● ssh:22" } else { "○ ssh:22" });
                            ui.colored_label(if r.port_ok { c_green() } else { c_red() }, format!("{} port:{}", if r.port_ok { "●" } else { "○" }, r.port));
                            ui.label(&r.host);
                        });
                    }
                });

                ui.collapsing("🔐 Ansible Vault", |ui| {
                    ui.label(RichText::new("Шифрование секретов. Выберите файл → 🔒 Encrypt или 🔓 Decrypt. Путь к пароль-файлу — в поле рядом.").color(c_tm()));
                    ui.horizontal(|ui| {
                        ui.text_edit_singleline(&mut self.vault_file);
                        if ui.button("📂").clicked() { if let Some(p) = rfd::FileDialog::new().pick_file() { self.vault_file = p.to_string_lossy().into_owned(); } }
                        ui.text_edit_singleline(&mut self.vault_pass);
                    });
                    ui.horizontal(|ui| {
                        let pf = if self.vault_pass.is_empty() { None } else { Some(self.vault_pass.as_str()) };
                        if ui.button("🔒 Encrypt").clicked() { self.vault_out = vault::run("encrypt", &self.vault_file, pf).unwrap_or_else(|e| e); }
                        if ui.button("🔓 Decrypt").clicked() { self.vault_out = vault::run("decrypt", &self.vault_file, pf).unwrap_or_else(|e| e); }
                    });
                    ui.label(RichText::new(&self.vault_out).monospace().small());
                });

                ui.collapsing("📦 Локальный RPM-репозиторий", |ui| {
                    ui.label(RichText::new("Папка с .rpm. Кнопка createrepo создаёт repodata — хосты РЕДОС ставят пакеты по сети без интернета.").color(c_tm()));
                    ui.horizontal(|ui| {
                        ui.text_edit_singleline(&mut self.rpm_dir);
                        if ui.button("📂").clicked() { if let Some(p) = rfd::FileDialog::new().pick_folder() { self.rpm_dir = p.to_string_lossy().into_owned(); } }
                        if ui.button("🛠 createrepo").clicked() { self.rpm_out = rpmrepo::createrepo(&self.rpm_dir).unwrap_or_else(|e| e); self.toast("📦 repodata создана".into()); }
                    });
                    let rpms = rpmrepo::list_rpms(&self.rpm_dir);
                    ui.label(format!("Пакетов: {}", rpms.len()));
                    for r in rpms.iter().take(20) { ui.label(format!("  • {}", r)); }
                    ui.label(RichText::new(&self.rpm_out).monospace().small());
                });

                ui.collapsing("🔗 Git (версионирование плейбуков)", |ui| {
                    ui.label(RichText::new("Системный git в выбранной папке: status/log/commit — откат изменений и командная работа.").color(c_tm()));
                    ui.horizontal(|ui| {
                        ui.text_edit_singleline(&mut self.git_dir);
                        if ui.button("📂").clicked() { if let Some(p) = rfd::FileDialog::new().pick_folder() { self.git_dir = p.to_string_lossy().into_owned(); } }
                    });
                    ui.horizontal(|ui| {
                        if ui.button("📊 status").clicked() { self.git_out = gitops::run(&self.git_dir, &["status", "--short"]); }
                        if ui.button("📜 log").clicked() { self.git_out = gitops::run(&self.git_dir, &["log", "--oneline", "-5"]); }
                        ui.text_edit_singleline(&mut self.git_msg);
                        if ui.button("💾 commit all").clicked() {
                            gitops::run(&self.git_dir, &["add", "-A"]);
                            self.git_out = gitops::run(&self.git_dir, &["commit", "-m", &self.git_msg]);
                            self.toast("🔗 Коммит создан".into());
                        }
                    });
                    ui.label(RichText::new(&self.git_out).monospace().small());
                });

                ui.collapsing("🕐 Планировщик задач", |ui| {
                    ui.label(RichText::new("Пока приложение открыто, playbook запускается каждые N минут.").color(c_tm()));
                    if ui.button("➕ Добавить текущий playbook").clicked() {
                        self.sched_tasks.push(SchedTask { playbook: self.playbook_path.clone(), inventory: self.inventory_path.clone(), interval_min: 60, last_run: None });
                    }
                    let mut remove: Vec<usize> = Vec::new();
                    for (i, t) in self.sched_tasks.iter_mut().enumerate() {
                        ui.horizontal(|ui| {
                            ui.add(egui::DragValue::new(&mut t.interval_min).range(1..=1440).suffix(" мин"));
                            ui.label(&t.playbook);
                            if ui.button("🗑️").clicked() { remove.push(i); }
                        });
                    }
                    for i in remove.into_iter().rev() { self.sched_tasks.remove(i); }
                });

                ui.collapsing("🤖 Zabbix (отправка метрик)", |ui| {
                    ui.label(RichText::new("Отправка метрики по протоколу Zabbix sender (host/key/value).").color(c_tm()));
                    ui.horizontal(|ui| {
                        ui.label("Server:"); ui.text_edit_singleline(&mut self.zbx_server);
                        ui.label("Port:"); ui.text_edit_singleline(&mut self.zbx_port);
                    });
                    if ui.button("📡 Отправить тестовую метрику").clicked() {
                        let port: u16 = self.zbx_port.parse().unwrap_or(10051);
                        self.zbx_out = zabbix::send(&self.zbx_server, port, "ansibleops", "ansibleops.status", "1").unwrap_or_else(|e| format!("error: {}", e));
                        self.toast("🤖 Метрика отправлена".into());
                    }
                    ui.label(RichText::new(&self.zbx_out).monospace().small());
                });

                ui.collapsing("📡 REST API / веб-режим", |ui| {
                    ui.label(RichText::new("Read-only API: статус и /api/history (JSON). Требуется bearer-токен.").color(c_tm()));
                    ui.label(RichText::new("⚠️ Не запускайте на 0.0.0.0 без ограничения доступа: API не имеет TLS и RBAC.").color(c_orange()));
                    ui.horizontal(|ui| {
                        ui.label("Bind:");
                        ui.add(egui::TextEdit::singleline(&mut self.web_bind).desired_width(110.0));
                        ui.label("Порт:");
                        ui.add(egui::TextEdit::singleline(&mut self.api_port_str).desired_width(60.0));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Токен:");
                        ui.monospace(&self.web_token);
                        if ui.button("🔄").clicked() { self.web_token = webapi::generate_token(); }
                    });
                    if !self.web_on {
                        if ui.button("🌐 Запустить Web API").clicked() {
                            let dir = self.history_store.clone().map(|s| s.dir.to_string_lossy().into_owned()).unwrap_or_default();
                            let port = self.api_port_str.parse::<u16>().unwrap_or(8080);
                            match webapi::start(port, &self.web_bind, &self.web_token, dir) {
                                Ok(_) => { self.web_on = true; self.toast(format!("🌐 Web API: http://{}:{}", self.web_bind, port)); }
                                Err(e) => self.toast(format!("🌐 Ошибка: {}", e)),
                            }
                        }
                    } else {
                        ui.colored_label(c_green(), format!("● Web API активен: http://{}:{} (доступ только с токеном)", self.web_bind, self.api_port_str));
                    }
                });

                ui.collapsing("📄 Экспорт отчёта (HTML / JSON / CSV)", |ui| {
                    ui.label(RichText::new("Формат → 'Экспортировать' → диалог выбора места сохранения.").color(c_tm()));
                    ui.horizontal(|ui| {
                        ui.label("Формат:");
                        egui::ComboBox::from_id_salt("rep_fmt").selected_text(self.report_format.to_uppercase()).show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.report_format, "html".into(), "HTML");
                            ui.selectable_value(&mut self.report_format, "json".into(), "JSON");
                            ui.selectable_value(&mut self.report_format, "csv".into(), "CSV");
                        });
                        if ui.button("📄 Экспортировать...").clicked() {
                            let ext = self.report_format.clone();
                            let content = {
                                let s = self.session.lock().unwrap();
                                match ext.as_str() { "json" => report::to_json(&s), "csv" => report::to_csv(&s), _ => report::to_html(&s) }
                            };
                            let name = format!("report_{}.{}", chrono::Local::now().format("%Y%m%d_%H%M%S"), ext);
                            if let Some(path) = rfd::FileDialog::new().set_file_name(&name).add_filter("Report", &[ext.as_str()]).save_file() {
                                match std::fs::write(&path, content) {
                                    Ok(_) => self.toast(format!("📄 Сохранено: {}", path.display())),
                                    Err(e) => self.toast(format!("Ошибка: {}", e)),
                                }
                            }
                        }
                    });
                });

                ui.collapsing("📡 Event-правила (автореакция)", |ui| {
                    ui.label(RichText::new("Каждые 60 сек проверяет условия; при нарушении — уведомление и/или действие (с cooldown, лимитом и подтверждением).").color(c_tm()));
                    ui.label(RichText::new("⚠️ Произвольный shell в действии всегда требует ручного подтверждения.").color(c_orange()));
                    if ui.button("➕ Правило").clicked() {
                        self.ops.rules.push(EventRule::default());
                    }
                    let mut rm: Vec<usize> = Vec::new();
                    for (i, r) in self.ops.rules.iter_mut().enumerate() {
                        ui.horizontal(|ui| {
                            ui.checkbox(&mut r.enabled, "");
                            egui::ComboBox::from_id_salt(format!("rule{}", i)).selected_text(&r.kind).show_ui(ui, |ui| {
                                for k in ["disk", "service", "host"] { ui.selectable_value(&mut r.kind, k.to_string(), k); }
                            });
                            ui.text_edit_singleline(&mut r.param);
                            ui.label("действие:");
                            ui.text_edit_singleline(&mut r.action);
                            if ui.button("🗑").clicked() { rm.push(i); }
                        });
                        ui.horizontal(|ui| {
                            ui.add(egui::DragValue::new(&mut r.cooldown_secs).range(0..=86400).suffix(" сек cooldown"));
                            ui.add(egui::DragValue::new(&mut r.max_runs).range(0..=100).prefix("макс. запусков: "));
                            ui.text_edit_singleline(&mut r.allowed_hosts);
                            ui.label("хосты (пусто = все)");
                            ui.checkbox(&mut r.require_confirm, "подтверждение");
                        });
                    }
                    for i in rm.into_iter().rev() { self.ops.rules.remove(i); }
                    // Окно подтверждения ожидающих действий
                    if !self.ops.pending_rules.is_empty() {
                        ui.label(RichText::new(format!("⏸ Ожидают подтверждения: {}", self.ops.pending_rules.len())).color(c_orange()).strong());
                    }
                    let mut approve: Vec<usize> = Vec::new();
                    for (i, r) in self.ops.pending_rules.iter_mut().enumerate() {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(format!("[{}] {}", r.kind, r.action)).monospace().color(c_tx()));
                            if ui.button("✅ Выполнить").clicked() { approve.push(i); }
                            if ui.button("❌ Отклонить").clicked() { r.rejected = true; self.ops.rule_log.push(format!("⛔ Действие отклонено: [{}] {}", r.kind, r.action)); }
                        });
                    }
                    for (n, i) in approve.into_iter().enumerate() {
                        let r = self.ops.pending_rules.remove(i - n);
                        if let Some(rule) = self.ops.rules.iter_mut().find(|x| x.kind == r.kind && x.param == r.param) { rule.runs += 1; rule.last_run = Some(std::time::Instant::now()); }
                        let target = if r.allowed_hosts.is_empty() { self.ops.hosts.clone() } else { r.allowed_hosts.clone() };
                        self.ops.rule_log.push(format!("✅ Выполнено (подтверждено): [{}] {} на {}", r.kind, r.action, target));
                        self.spawn_raw(target, "shell".into(), r.action.clone());
                    }
                    self.ops.pending_rules.retain(|r| !r.rejected);
                    egui::ScrollArea::vertical().max_height(150.0).show(ui, |ui| {
                        for l in self.ops.rule_log.iter().rev().take(50) {
                            ui.label(RichText::new(l).monospace().color(c_tx()));
                        }
                    });
                });
            });
        });
    }

    pub fn show_history(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default()
            .frame(Frame::none().inner_margin(Margin::same(14.0)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.heading(RichText::new("🗂 Run History").color(c_purple()));
                    if ui.button("🔄 Refresh").clicked() {
                        self.refresh_history();
                    }
                    if !self.history.is_empty()
                        && ui.button("🗄 Архивировать всё (retention)").clicked()
                    {
                        if let Some(store) = &self.history_store {
                            for s in &self.history {
                                let _ = store.delete(&s.id);
                            }
                            store.audit("history_archive_all");
                            self.history.clear();
                        }
                    }
                });
                ui.add_space(8.0);
                egui::ScrollArea::vertical().show(ui, |ui| {
                    if self.history.is_empty() {
                        ui.colored_label(c_tm(), "No runs yet.");
                    }
                    for s in &self.history.clone() {
                        Frame::none()
                            .fill(c_surf())
                            .rounding(Rounding::same(10.0))
                            .inner_margin(Margin::same(12.0))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    let (ic, cc) = match &s.state {
                                        RunState::Finished { success: true } => ("✓", c_green()),
                                        RunState::Finished { success: false } => ("✗", c_red()),
                                        _ => ("●", c_orange()),
                                    };
                                    ui.label(RichText::new(ic).color(cc).strong().size(18.0));
                                    ui.vertical(|ui| {
                                        ui.label(RichText::new(&s.playbook).strong().color(c_tx()));
                                        ui.label(
                                            RichText::new(format!(
                                                "{} → {}",
                                                s.started_at
                                                    .map(|t| t.format("%Y-%m-%d %H:%M").to_string())
                                                    .unwrap_or_default(),
                                                s.finished_at
                                                    .map(|t| t.format("%H:%M:%S").to_string())
                                                    .unwrap_or_default()
                                            ))
                                            .color(c_tm()),
                                        );
                                    });
                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                        ui.horizontal(|ui| {
                                            badge(ui, "ok", s.summary.ok, c_green());
                                            badge(ui, "chg", s.summary.changed, c_blue());
                                            badge(ui, "fail", s.summary.failed, c_red());
                                        });
                                        if ui.button("🗄️").clicked() {
                                            if let Some(store) = &self.history_store {
                                                let _ = store.delete(&s.id);
                                                self.history.retain(|x| x.id != s.id);
                                            }
                                        }
                                    });
                                });
                                ui.collapsing("📋 Детали запуска", |ui| {
                                    if s.events.is_empty() {
                                        ui.colored_label(c_tm(), "События не сохранены");
                                    }
                                    for ev in &s.events {
                                        let col = match ev.status {
                                            TaskStatus::Failed => c_red(),
                                            TaskStatus::Changed => c_blue(),
                                            TaskStatus::Ok => c_green(),
                                            _ => c_tm(),
                                        };
                                        ui.label(
                                            RichText::new(format!(
                                                "{:?} [{}] {}",
                                                ev.status, ev.host, ev.output
                                            ))
                                            .monospace()
                                            .small()
                                            .color(col),
                                        );
                                    }
                                });
                            });
                        ui.add_space(6.0);
                    }
                });
            });
    }

    pub fn show_diff(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().frame(Frame::none().inner_margin(Margin::same(14.0))).show(ctx, |ui| {
            ui.heading(RichText::new("🔍 Diff и версии файлов").color(c_purple()));
            ui.label(RichText::new("Dry-run playbook и просмотр изменений. Авто-сохранение версий конфигов перед применением.").color(c_tm()));
            ui.add_space(8.0);
            ui.collapsing("▶ Запустить dry-run текущего playbook", |ui| {
                ui.label(RichText::new("Выполняет ansible-playbook --check --diff и парсит вывод в список изменений.").color(c_tm()));
                if ui.add_enabled(!self.diff.scan_running, egui::Button::new(if self.diff.scan_running { "⏳ Сканирование..." } else { "🔍 Запустить diff-сканирование" })).clicked() {
                    self.diff.scan_running = true;
                    self.diff.changes.clear();
                    let pb = self.playbook_path.clone();
                    let inv = self.inventory_path.clone();
                    let (tx, rx) = std::sync::mpsc::channel();
                    self.diff_rx = Some(rx);
                    std::thread::spawn(move || {
                        let out = std::process::Command::new("ansible-playbook")
                            .arg(&pb).arg("-i").arg(&inv).arg("--check").arg("--diff")
                            .output();
                        match out {
                            Ok(o) => {
                                let _ = tx.send(String::from_utf8_lossy(&o.stdout).into_owned());
                                let _ = tx.send(String::from_utf8_lossy(&o.stderr).into_owned());
                            }
                            Err(e) => { let _ = tx.send(format!("error: {}", e)); }
                        }
                    });
                }
            });
            ui.add_space(8.0);
            ui.collapsing("📑 История версий файлов", |ui| {
                ui.label(RichText::new("Выберите хост и файл, чтобы увидеть все сохранённые версии.").color(c_tm()));
                ui.horizontal(|ui| {
                    ui.label("Хост:"); ui.text_edit_singleline(&mut self.diff.selected_host);
                    ui.label("Файл:"); ui.text_edit_singleline(&mut self.diff.selected_file);
                    if ui.button("🔄 Показать").clicked() {
                        if let Some(store) = &self.versions {
                            self.diff.versions = store.list(&self.diff.selected_host, &self.diff.selected_file);
                        }
                    }
                    if ui.button("⬇ Скачать и сохранить").clicked()
                        && !self.diff.selected_host.is_empty()
                        && !self.diff.selected_file.is_empty()
                    {
                        match diff_mod::VersionStore::fetch_remote(&self.diff.selected_host, &self.diff.selected_file, &self.inventory_path) {
                            Ok(content) => {
                                let result = {
                                    if let Some(store) = &self.versions {
                                        match store.save(&self.diff.selected_host, &self.diff.selected_file, &content, "manual fetch") {
                                            Ok(_) => {
                                                let versions = store.list(&self.diff.selected_host, &self.diff.selected_file);
                                                Some((true, String::new(), versions))
                                            }
                                            Err(e) => Some((false, e, Vec::new())),
                                        }
                                    } else {
                                        None
                                    }
                                };
                                if let Some((ok, err, versions)) = result {
                                    if ok {
                                        self.toast("💾 Версия сохранена".into());
                                        self.diff.versions = versions;
                                    } else {
                                        self.toast(format!("Ошибка: {}", err));
                                    }
                                }
                            }
                            Err(e) => self.toast(format!("Не удалось скачать: {}", e)),
                        }
                    }
                });
                ui.add_space(6.0);
                egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                    let versions = self.diff.versions.clone();
                    for v in &versions {
                        Frame::none().fill(c_surf()).rounding(Rounding::same(8.0)).inner_margin(Margin::same(8.0)).show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(v.saved_at.format("%Y-%m-%d %H:%M:%S").to_string()).color(c_cyan()).strong());
                                ui.label(format!("[{}]", v.source));
                                if ui.button("⬆ Восстановить на хост").clicked() {
                                    let b64 = general_purpose::STANDARD.encode(v.content.as_bytes());
                                    let host = self.diff.selected_host.clone();
                                    let file = self.diff.selected_file.clone();
                                    let (tx, rx) = std::sync::mpsc::channel();
                                    self.ops_rx = Some(rx);
                                    std::thread::spawn(move || {
                                        let cmd = format!("echo '{}' | base64 -d > {}", b64, file);
                                        crate::ansible::adhoc::run_raw(&host, "shell", &cmd, true, tx);
                                    });
                                    self.toast("⬆ Восстановление запущено".into());
                                }
                            });
                            ui.collapsing("Содержимое", |ui| {
                                ui.add(egui::TextEdit::multiline(&mut {v.content.clone()})
                                    .font(egui::FontId::monospace(12.0)).desired_width(f64::INFINITY as f32).desired_rows(12));
                            });
                        });
                        ui.add_space(3.0);
                    }
                    if self.diff.versions.is_empty() { ui.colored_label(c_tm(), "Нет сохранённых версий"); }
                });
            });
            ui.add_space(8.0);
            ui.label(RichText::new("📋 Результат последнего diff-сканирования:").strong());
            egui::ScrollArea::vertical().show(ui, |ui| {
                if self.diff.changes.is_empty() {
                    ui.colored_label(c_tm(), "Нет изменений или сканирование не запускалось");
                }
                for c in &self.diff.changes {
                    Frame::none().fill(c_surf()).rounding(Rounding::same(8.0)).inner_margin(Margin::same(8.0)).show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(&c.host).color(c_cyan()).strong());
                            ui.label(RichText::new(&c.path).color(c_tx()));
                            ui.colored_label(c_green(), format!("+{}", c.added_lines));
                            ui.colored_label(c_red(), format!("-{}", c.removed_lines));
                        });
                        ui.collapsing("Diff", |ui| {
                            if !c.before.is_empty() {
                                ui.label(RichText::new("— до:").color(c_red()));
                                ui.add(egui::TextEdit::multiline(&mut {c.before.clone()}).font(egui::FontId::monospace(11.0)).desired_width(f64::INFINITY as f32));
                            }
                            if !c.after.is_empty() {
                                ui.label(RichText::new("— после:").color(c_green()));
                                ui.add(egui::TextEdit::multiline(&mut {c.after.clone()}).font(egui::FontId::monospace(11.0)).desired_width(f64::INFINITY as f32));
                            }
                        });
                    });
                    ui.add_space(4.0);
                }
            });
        });
    }

    pub fn show_roles(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().frame(Frame::none().inner_margin(Margin::same(14.0))).show(ctx, |ui| {
            ui.heading(RichText::new("📦 Каталог ролей").color(c_purple()));
            ui.label(RichText::new("Готовые роли с метаданными: версия, совместимость, риск, теги, откат. Генерируют playbook в 1 клик.").color(c_tm()));
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.label("Фильтр по тегу:");
                egui::ComboBox::from_id_salt("role_tag").selected_text(if self.roles.filter_tag.is_empty() { "все" } else { &self.roles.filter_tag }).show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.roles.filter_tag, String::new(), "все");
                    for t in roles_mod::role_tags_list() {
                        ui.selectable_value(&mut self.roles.filter_tag, t.to_string(), t);
                    }
                });
                ui.label("Хосты для deploy:");
                ui.text_edit_singleline(&mut self.roles.target_hosts);
            });
            ui.add_space(8.0);
            egui::ScrollArea::vertical().show(ui, |ui| {
                for r in roles_mod::role_catalog() {
                    if !self.roles.filter_tag.is_empty() && !r.tags.contains(&self.roles.filter_tag.as_str()) { continue; }
                    let selected = self.roles.selected == r.name;
                    Frame::none().fill(if selected { c_surfh() } else { c_surf() })
                        .rounding(Rounding::same(10.0)).inner_margin(Margin::same(12.0)).show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new(r.name).strong().size(16.0).color(c_tx()));
                                    ui.label(RichText::new(format!("v{}", r.version)).color(c_cyan()));
                                    let risk_col = match r.risk { "высокий" => c_red(), "средний" => c_orange(), _ => c_green() };
                                    ui.colored_label(risk_col, format!("[risk: {}]", r.risk));
                                });
                                ui.label(RichText::new(r.description).color(c_tx()));
                                ui.label(RichText::new(format!("Совместимость: {}", r.compat.join(", "))).color(c_blue()));
                                ui.label(RichText::new(format!("Теги: {} • Автор: {}", r.tags.join(", "), r.author)).color(c_tm()));
                            });
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if ui.button("▶ Deploy").clicked() {
                                    let yaml = roles_mod::role_to_playbook(&r, &self.roles.target_hosts);
                                    self.editor.content = yaml.clone();
                                    if let Some(path) = rfd::FileDialog::new().set_file_name(format!("{}.yml", r.name)).add_filter("YAML", &["yml"]).save_file() {
                                        if std::fs::write(&path, yaml).is_ok() {
                                            self.playbook_path = path.to_string_lossy().into_owned();
                                            self.start_run();
                                            self.toast(format!("🚀 Роль {} запущена", r.name));
                                        }
                                    }
                                }
                                if ui.button("📋 В Editor").clicked() {
                                    self.editor.content = roles_mod::role_to_playbook(&r, &self.roles.target_hosts);
                                    self.tab = Tab::Editor;
                                }
                            });
                        });
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            if r.requires_confirm { ui.colored_label(c_orange(), "⚠ требует подтверждения"); }
                            if r.has_rollback { ui.colored_label(c_green(), "↩ есть откат"); }
                        });
                    });
                    ui.add_space(6.0);
                }
            });
        });
    }
}
