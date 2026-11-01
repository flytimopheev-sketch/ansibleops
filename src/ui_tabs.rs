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
            .default_width(560.0).min_width(480.0)
            .frame(Frame::none().inner_margin(Margin::same(14.0)))
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.heading(RichText::new("⚙️ Конфигурация").color(c_purple()));
                    ui.add_space(8.0);
                    ui.label(RichText::new("Inventory").color(c_tm()));
                ui.horizontal(|ui| {
                    ui.text_edit_singleline(&mut self.inventory_path);
                    if ui.button("рџ“‚ РћР±Р·РѕСЂ")
                        .on_hover_text("Р’С‹Р±СЂР°С‚СЊ С„Р°Р№Р» РёРЅРІРµРЅС‚Р°СЂСЏ")
                        .clicked()
                    {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter(
                                "Ansible Inventory",
                                &["ini", "yml", "yaml", "json", "toml"],
                            )
                            .pick_file()
                        {
                            self.inventory_path = path.to_string_lossy().into_owned();
                            self.ui.error_message = None;
                        }
                    }
                });
                ui.label(RichText::new("Playbook").color(c_tm()));
                ui.horizontal(|ui| {
                    ui.text_edit_singleline(&mut self.playbook_path);
                    if ui.button("рџ“‚ РћР±Р·РѕСЂ")
                        .on_hover_text("Р’С‹Р±СЂР°С‚СЊ С„Р°Р№Р» РїР»РµР№Р±СѓРєР°")
                        .clicked()
                    {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("YAML Files", &["yml", "yaml"])
                            .pick_file()
                        {
                            self.playbook_path = path.to_string_lossy().into_owned();
                            self.ui.error_message = None;
                        }
                    }
                });
                ui.label(RichText::new("Extra vars (-e)").color(c_tm()));
                ui.text_edit_singleline(&mut self.extra_vars);
                if let Some(err) = &self.ui.error_message {
                    ui.add_space(4.0);
                    ui.colored_label(c_red(), RichText::new(format!("вљ пёЏ {}", err)).strong());
                }
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button("рџ“Ґ Load inv").clicked() {
                        match parse_inventory(std::path::Path::new(&self.inventory_path)) {
                            Ok(h) => {
                                self.hosts = h;
                                self.ui.error_message = None;
                            }
                            Err(e) => {
                                self.hosts.clear();
                                self.ui.error_message = Some(format!("РћС€РёР±РєР° РїР°СЂСЃРёРЅРіР°: {}", e));
                            }
                        }
                    }
                    if ui.button("рџ“ќ Load pb").clicked() {
                        self.load_playbook_graph();
                    }
                });
                ui.add_space(12.0);
                ui.separator();
                ui.add_space(8.0);
                ui.heading(RichText::new("рџ”ђ SSH / Vault").color(c_purple()));
                ui.label(RichText::new("Private key").color(c_tm()));
                ui.horizontal(|ui| {
                    let mut kp = self
                        .ssh
                        .private_key
                        .as_ref()
                        .map(|p| p.to_string_lossy().into_owned())
                        .unwrap_or_default();
                    ui.text_edit_singleline(&mut kp);
                    if ui.button("рџ“‚ РљР»СЋС‡")
                        .on_hover_text("Р’С‹Р±СЂР°С‚СЊ SSH-РєР»СЋС‡")
                        .clicked()
                    {
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
                                "вЏі Running..."
                            } else {
                                "в–¶  Run Playbook"
                            })
                            .color(Color32::WHITE)
                            .strong(),
                        .on_hover_text("Р—Р°РїСѓСЃС‚РёС‚СЊ РїР»РµР№Р±СѓРє")
                        .fill(c_green())
                        .min_size(Vec2::new(ui.available_width(), 42.0))
                        .rounding(Rounding::same(10.0)),
                    )
                    .clicked()
                {
                    if self.playbook_path.is_empty() {
                        self.ui.error_message = Some("РЈРєР°Р¶РёС‚Рµ РїСѓС‚СЊ Рє РїР»РµР№Р±СѓРєСѓ".into());
                    } else if !std::path::Path::new(&self.playbook_path).exists() {
                        self.ui.error_message = Some(format!(
                            "РџР»РµР№Р±СѓРє РЅРµ РЅР°Р№РґРµРЅ: {}",
                            self.playbook_path
                        ));
                    } else if self.inventory_path.is_empty() {
                        self.ui.error_message = Some("РЈРєР°Р¶РёС‚Рµ РїСѓС‚СЊ Рє РёРЅРІРµРЅС‚Р°СЂСЋ".into());
                    } else if !std::path::Path::new(&self.inventory_path).exists() {
                        self.ui.error_message = Some(format!(
                            "РРЅРІРµРЅС‚Р°СЂСЊ РЅРµ РЅР°Р№РґРµРЅ: {}",
                            self.inventory_path
                        ));
                    } else {
                        self.start_run();
                    }
                }
                if running
                    && ui
                        .add_enabled(
                            running,
                            egui::Button::new(
                                RichText::new("вЏ№ РћСЃС‚Р°РЅРѕРІРёС‚СЊ").color(Color32::WHITE).strong(),
                            )
                            .on_hover_text("РћСЃС‚Р°РЅРѕРІРёС‚СЊ РІС‹РїРѕР»РЅРµРЅРёРµ РїР»РµР№Р±СѓРєР°")
                            .fill(c_red())
                            .min_size(Vec2::new(ui.available_width(), 32.0))
                            .rounding(Rounding::same(10.0)),
                        )
                        .clicked()
                {
                    self.stop_confirm = true;
                }
                ui.add_space(16.0);
                ui.heading(RichText::new("рџ–Ґ Hosts").color(c_purple()));
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
                                    ui.label(RichText::new(format!("в†’ {ah}")).color(c_tm()));
                                }
                            });
                        ui.add_space(4.0);
                    }
                });
                });
            });
        // РџРѕРґС‚РІРµСЂР¶РґРµРЅРёРµ РѕСЃС‚Р°РЅРѕРІРєРё
        if self.stop_confirm {
            egui::Window::new("вЏ№ РћСЃС‚Р°РЅРѕРІРёС‚СЊ РІС‹РїРѕР»РЅРµРЅРёРµ?")
                .collapsible(false)
                .resizable(false)
                .default_size(Vec2::new(340.0, 120.0))
                .show(ctx, |ui| {
                    ui.label("Р’С‹ СѓРІРµСЂРµРЅС‹, С‡С‚Рѕ С…РѕС‚РёС‚Рµ РѕСЃС‚Р°РЅРѕРІРёС‚СЊ РІС‹РїРѕР»РЅРµРЅРёРµ РїР»РµР№Р±СѓРєР°?");
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui.button("Р”Р°, РѕСЃС‚Р°РЅРѕРІРёС‚СЊ").clicked() {
                            self.stop_run();
                            self.stop_confirm = false;
                        }
                        ui.add_space(8.0);
                        if ui.button("РћС‚РјРµРЅР°").clicked() {
                            self.stop_confirm = false;
                        }
                    });
                });
        }
        egui::CentralPanel::default()
            .frame(Frame::none().inner_margin(Margin::same(14.0)))
            .show(ctx, |ui| {
                ui.heading(RichText::new("рџ“њ Execution Log").color(c_purple()));
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
                            .stick_to_bottom(self.ui.auto_scroll)
                            .show(ui, |ui| {
                                if sess.events.is_empty() {
                                    ui.colored_label(c_tm(), "Press в–¶ Run Playbook to start...");
                                }
                                for ev in &sess.events {
                                    let (icon, color) = match ev.status {
                                        TaskStatus::Ok => ("вњ“", c_green()),
                                        TaskStatus::Changed => ("в†»", c_blue()),
                                        TaskStatus::Failed => ("вњ—", c_red()),
                                        TaskStatus::Skipped => ("вЉ", c_tm()),
                                        TaskStatus::Running => ("в—Џ", c_orange()),
                                        TaskStatus::Pending => ("в—‹", c_tm()),
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
                    ui.checkbox(&mut self.ui.auto_scroll, "Auto-scroll");
                    if ui.button("рџ§№ Clear").clicked() {
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
                    ui.heading(RichText::new("рџ“ќ Playbook Editor").color(c_purple()));
                    if let Some(p) = &self.editor.path {
                        ui.label(RichText::new(p.to_string_lossy()).color(c_tm()));
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.button("рџ’ѕ Save").clicked() {
                            if let Err(e) = self.editor.save() {
                                self.ui.error_message = Some(format!("РћС€РёР±РєР° СЃРѕС…СЂР°РЅРµРЅРёСЏ: {}", e));
                            } else {
                                self.ui.error_message = None;
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
            .default_width(420.0).min_width(350.0)
            .frame(Frame::none().inner_margin(Margin::same(14.0)))
            .show(ctx, |ui| {
                ui.heading(RichText::new("рџЏ—пёЏ РљРѕРЅСЃС‚СЂСѓРєС‚РѕСЂ Playbook").color(c_purple()));
                ui.add_space(12.0);
                ui.label(RichText::new("вљ™пёЏ PLAY РЅР°СЃС‚СЂРѕР№РєРё").color(c_blue()).strong());
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
                ui.label(RichText::new("рџ“¦ РњРѕРґСѓР»Рё").color(c_blue()).strong());
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
                    ui.heading(RichText::new("рџ“‹ Р—Р°РґР°С‡Рё").color(c_purple()));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.button("рџ‘Ѓ РџСЂРµРґРїСЂРѕСЃРјРѕС‚СЂ YAML").clicked() {
                            self.show_yaml_preview = !self.show_yaml_preview;
                        }
                        if ui.button("рџ’ѕ РЎРѕС…СЂР°РЅРёС‚СЊ РєР°Рє playbook").clicked() {
                            self.save_constructor_to_file();
                        }
                    });
                });
                ui.add_space(8.0);
                if self.show_yaml_preview {
                    ui.collapsing("рџ“„ YAML Preview", |ui| {
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
                        ui.colored_label(c_tm(), "Р”РѕР±Р°РІСЊС‚Рµ Р·Р°РґР°С‡Рё РёР· РїР°РЅРµР»Рё СЃР»РµРІР° в†’");
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
                                    if ui.button("рџ—‘пёЏ").clicked() {
                                        self.constructor_play.tasks.remove(idx);
                                    }
                                    if idx > 0 && ui.button("в†‘").clicked() {
                                        self.constructor_play.tasks.swap(idx, idx - 1);
                                    }
                                    if idx < self.constructor_play.tasks.len() - 1
                                        && ui.button("в†“").clicked()
                                    {
                                        self.constructor_play.tasks.swap(idx, idx + 1);
                                    }
                                });
                            });
                            ui.add_space(4.0);
                            ui.label(RichText::new("РџР°СЂР°РјРµС‚СЂС‹ РјРѕРґСѓР»СЏ:").color(c_tm()));
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
                                format!("рџ“– РЎРїСЂР°РІРєР°: {} вЂ” {}", module, module_title),
                                |ui| {
                                    ui.label(RichText::new(module_desc).color(c_tm()));
                                    ui.add_space(4.0);
                                    ui.label(RichText::new("Р”РѕСЃС‚СѓРїРЅС‹Рµ РїР°СЂР°РјРµС‚СЂС‹:").strong());
                                    for param in module_params {
                                        ui.label(format!("  вЂў {}", param));
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
            ui.heading(RichText::new("рџ› пёЏ РђРґРјРёРЅРёСЃС‚СЂР°С‚РёРІРЅС‹Рµ РјРѕРґСѓР»Рё").color(c_purple()));
            ui.add_space(8.0);
            ui.collapsing("рџ“– РљР°Рє РёСЃРїРѕР»СЊР·РѕРІР°С‚СЊ Р°РґРјРёРЅ-РјРѕРґСѓР»Рё?", |ui| {
                ui.label(RichText::new("Р‘С‹СЃС‚СЂРѕРµ РІС‹РїРѕР»РЅРµРЅРёРµ Р·Р°РґР°С‡ СѓРїСЂР°РІР»РµРЅРёСЏ СЃРёСЃС‚РµРјР°РјРё С‡РµСЂРµР· GUI.").color(c_tm()));
                ui.label("вЂў Р’С‹Р±РµСЂРёС‚Рµ РјРѕРґСѓР»СЊ в†’ РґРµР№СЃС‚РІРёРµ в†’ РёРјСЏ в†’ 'Р’С‹РїРѕР»РЅРёС‚СЊ Р·Р°РґР°С‡Сѓ'");
            });
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(RichText::new("Р¦РµР»РµРІС‹Рµ С…РѕСЃС‚С‹").color(c_tm()));
                    ui.text_edit_singleline(&mut self.admin_action.target_hosts);
                    ui.label(RichText::new("РњРѕРґСѓР»СЊ").color(c_tm()));
                    egui::ComboBox::from_id_salt("admin_mod").selected_text(self.admin_action.module_type_to_str()).show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.admin_action.module_type, AdminModuleType::Package, "Package (РЈСЃС‚Р°РЅРѕРІРєР° РџРћ)");
                        ui.selectable_value(&mut self.admin_action.module_type, AdminModuleType::Service, "Service (РЈРїСЂР°РІР»РµРЅРёРµ СЃР»СѓР¶Р±Р°РјРё)");
                        ui.selectable_value(&mut self.admin_action.module_type, AdminModuleType::User, "User (РџРѕР»СЊР·РѕРІР°С‚РµР»Рё)");
                        ui.selectable_value(&mut self.admin_action.module_type, AdminModuleType::File, "File (Р¤Р°Р№Р»С‹/РљРѕРЅС„РёРіРё)");
                    });
                    let module_hint = match self.admin_action.module_type {
                        AdminModuleType::Package => "Р”РµР№СЃС‚РІРёСЏ: install, remove, update. РРјСЏ: РЅР°Р·РІР°РЅРёРµ РїР°РєРµС‚Р° (nginx, httpd, vim)",
                        AdminModuleType::Service => "Р”РµР№СЃС‚РІРёСЏ: started, stopped, restarted, enabled, disabled. РРјСЏ: РЅР°Р·РІР°РЅРёРµ СЃРµСЂРІРёСЃР° (sshd, nginx, docker)",
                        AdminModuleType::User => "Р”РµР№СЃС‚РІРёСЏ: present, absent. РРјСЏ: РёРјСЏ РїРѕР»СЊР·РѕРІР°С‚РµР»СЏ",
                        AdminModuleType::File => "Р”РµР№СЃС‚РІРёСЏ: present, absent, directory. РРјСЏ: РїСѓС‚СЊ Рє С„Р°Р№Р»Сѓ (/etc/nginx/nginx.conf)",
                    };
                    ui.label(RichText::new(module_hint).color(c_blue()).small());
                    ui.label(RichText::new("Р”РµР№СЃС‚РІРёРµ").color(c_tm()));
                    ui.text_edit_singleline(&mut self.admin_action.action);
                    ui.label(RichText::new("РРјСЏ").color(c_tm()));
                    ui.text_edit_singleline(&mut self.admin_action.name);
                    ui.label(RichText::new("Р”РѕРї. Р°СЂРіСѓРјРµРЅС‚С‹").color(c_tm()));
                    ui.text_edit_singleline(&mut self.admin_action.extra_args);
                    ui.checkbox(&mut self.admin_action.use_become, "Use become (sudo)");
                    ui.checkbox(&mut self.admin_action.check_mode, "Check mode (Dry Run)");
                    ui.add_space(16.0);
                    if ui.button("в–¶ Р’С‹РїРѕР»РЅРёС‚СЊ Р·Р°РґР°С‡Сѓ").clicked() {
                        let (tx, rx) = mpsc::channel(256);
                        self.job_rx = Some(rx);
                        let action = self.admin_action.clone();
                        let inv = self.inventory_path.clone();
                        tokio::spawn(async move { let _ = AdHocRunner::run_admin_action(action, &inv, tx).await; });
                    }
                });
                ui.vertical(|ui| {
                    ui.label(RichText::new("рџ“Љ РђРєС‚РёРІРЅС‹Рµ Р·Р°РґР°С‡Рё").color(c_blue()));
                    egui::ScrollArea::vertical().max_height(400.0).show(ui, |ui| {
                        for job in &self.background_jobs {
                            let color = match job.status.as_str() { "Success" => c_green(), "Failed" => c_red(), _ => c_orange() };
                            Frame::none().fill(c_surf()).rounding(Rounding::same(8.0)).inner_margin(Margin::same(9.0)).show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new(&job.name).strong().color(c_tx()));
                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| { ui.colored_label(color, &job.status); });
                                });
                                ui.add(egui::ProgressBar::new(job.progress).show_percentage());
                                ui.collapsing("рџ“‹ Р’С‹РІРѕРґ", |ui| { for line in &job.output { ui.label(RichText::new(line).monospace().small().color(c_tx())); } });
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
            ui.heading(RichText::new("рџ’» Ad-Hoc & REPL РљРѕРЅСЃРѕР»СЊ").color(c_purple()));
            ui.add_space(8.0);
            ui.collapsing("рџ“– Р§С‚Рѕ СЌС‚Рѕ Рё Р·Р°С‡РµРј?", |ui| {
                ui.label(RichText::new("Ad-Hoc вЂ” СЂР°Р·РѕРІС‹Рµ РєРѕРјР°РЅРґС‹ Ansible Р‘Р•Р— СЃРѕР·РґР°РЅРёСЏ playbook. РќСѓР¶РЅС‹ РґР»СЏ Р±С‹СЃС‚СЂС‹С… РѕРїРµСЂР°С†РёР№: РїСЂРѕРІРµСЂРёС‚СЊ СЃРІСЏР·СЊ, РїРµСЂРµР·Р°РїСѓСЃС‚РёС‚СЊ СЃРµСЂРІРёСЃ, РїРѕСЃС‚Р°РІРёС‚СЊ РїР°РєРµС‚, СЃРѕР±СЂР°С‚СЊ С„Р°РєС‚.").color(c_tm()));
                ui.label(RichText::new("Р¤РѕСЂРјР°С‚: ansible <С…РѕСЃС‚С‹> -m <РјРѕРґСѓР»СЊ> -a '<Р°СЂРіСѓРјРµРЅС‚С‹>'").color(c_blue()).monospace());
                ui.label("вЂў ansible all -m ping вЂ” РїСЂРѕРІРµСЂРёС‚СЊ СЃРІСЏР·СЊ СЃРѕ РІСЃРµРјРё");
                ui.label("вЂў ansible webservers -m service -a 'name=nginx state=restarted'");
                ui.label("вЂў ansible all -m dnf -a 'name=vim state=present'");
            });
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("РҐРѕСЃС‚С‹:").color(c_tm()));
                ui.text_edit_singleline(&mut self.admin_action.target_hosts);
                ui.label(RichText::new("РњРѕРґСѓР»СЊ:").color(c_tm()));
                let modules = vec!["ping", "shell", "command", "apt", "yum", "dnf", "service", "user", "copy", "template", "file", "debug", "setup"];
                egui::ComboBox::from_id_salt("adhoc_mod").selected_text(&self.adhoc_module).show_ui(ui, |ui| { for m in modules { ui.selectable_value(&mut self.adhoc_module, m.to_string(), m); } });
                ui.label(RichText::new("РђСЂРіСѓРјРµРЅС‚С‹:").color(c_tm()));
                ui.text_edit_singleline(&mut self.adhoc_args);
                if ui.button("в–¶ Р’С‹РїРѕР»РЅРёС‚СЊ").clicked() {
                    self.spawn_raw(self.admin_action.target_hosts.clone(), self.adhoc_module.clone(), self.adhoc_args.clone());
                    self.toast("рџ’» Ad-Hoc Р·Р°РїСѓС‰РµРЅР°".into());
                }
            });
            ui.add_space(12.0);
            ui.collapsing("рџ“љ РЎРїСЂР°РІРѕС‡РЅРёРє РјРѕРґСѓР»РµР№", |ui| {
                egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                    for module in vec!["ping", "apt", "yum", "dnf", "service", "user", "copy", "file", "command", "shell", "debug", "setup"] {
                        let (title, desc, params) = ModuleReference::get_module_info(module);
                        ui.collapsing(format!("{} вЂ” {}", module, title), |ui| {
                            ui.label(RichText::new(desc).color(c_tm()));
                            if !params.is_empty() {
                                ui.label(RichText::new("РџР°СЂР°РјРµС‚СЂС‹:").strong());
                                for param in params { ui.label(format!("  вЂў {}", param)); }
                            }
                        });
                    }
                });
            });
            ui.add_space(12.0);
            ui.label(RichText::new("рџ“њ Р’С‹РІРѕРґ:").strong());
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
                ui.heading(RichText::new("рџ“љ Р‘РёР±Р»РёРѕС‚РµРєР° РіРѕС‚РѕРІС‹С… СЂРµС€РµРЅРёР№").color(c_purple()));
                ui.add_space(8.0);
                ui.collapsing(
                    "рџ“– РљР°Рє РёСЃРїРѕР»СЊР·РѕРІР°С‚СЊ Р±РёР±Р»РёРѕС‚РµРєСѓ?",
                    |ui| {
                        ui.label(
                            RichText::new("Р“РѕС‚РѕРІС‹Рµ playbook'Рё РґР»СЏ С‚РёРїРёС‡РЅС‹С… Р·Р°РґР°С‡.").color(c_tm()),
                        );
                        ui.label("вЂў 'рџ“‹ РљРѕРїРёСЂРѕРІР°С‚СЊ РІ Editor' вЂ” РѕС‚СЂРµРґР°РєС‚РёСЂРѕРІР°С‚СЊ");
                        ui.label("вЂў 'в–¶ Deploy' вЂ” СЃРѕС…СЂР°РЅРёС‚СЊ РІРѕ РІСЂРµРјРµРЅРЅС‹Р№ С„Р°Р№Р» Рё СЃСЂР°Р·Сѓ Р·Р°РїСѓСЃС‚РёС‚СЊ");
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
                                    if ui.button("рџ“‹ РљРѕРїРёСЂРѕРІР°С‚СЊ РІ Editor").clicked() {
                                        self.editor.content = pb.yaml_content.clone();
                                        self.tab = Tab::Editor;
                                    }
                                    if ui.button("в–¶ Deploy").clicked() {
                                        let tmp = std::env::temp_dir().join(format!(
                                            "lib_deploy_{}.yml",
                                            pb.id.chars().take(8).collect::<String>()
                                        ));
                                        if std::fs::write(&tmp, &pb.yaml_content).is_ok() {
                                            self.playbook_path = tmp.to_string_lossy().into_owned();
                                            self.start_run();
                                            self.toast(format!("рџљЂ Р—Р°РїСѓСЃРє: {}", pb.name));
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
                    ui.heading(RichText::new("рџ•ё Task Dependency Graph").color(c_purple()));
                    if let Some(g) = &self.graph {
                        if g.order_changed {
                            ui.colored_label(c_orange(), "вљ пёЏ РџРѕСЂСЏРґРѕРє РёР·РјРµРЅС‘РЅ");
                            if ui.button("рџ’ѕ РЎРѕС…СЂР°РЅРёС‚СЊ РІ playbook").clicked() {
                                self.save_playbook_order();
                            }
                        }
                    }
                });
                ui.add_space(8.0);
                if let Some(g) = &mut self.graph {
                    ui.label(
                        RichText::new("рџ’Ў РџРµСЂРµС‚Р°С‰РёС‚Рµ СѓР·Р»С‹ РјС‹С€СЊСЋ, С‡С‚РѕР±С‹ РёР·РјРµРЅРёС‚СЊ РїРѕСЂСЏРґРѕРє Р·Р°РґР°С‡")
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
                                ui.colored_label(c_tm(), "РќРµС‚ РґР°РЅРЅС‹С… РїР»РµР№Р±СѓРєР°");
                            }
                        });
                    ui.add_space(8.0);
                    ui.collapsing("рџ“‹ РўРµРєСѓС‰РёР№ РїРѕСЂСЏРґРѕРє Р·Р°РґР°С‡", |ui| {
                        if let Some(pb) = &self.playbook_data {
                            for (i, task) in pb.tasks.iter().enumerate() {
                                ui.label(format!("{}. {}", i + 1, task.name));
                            }
                        }
                    });
                } else {
                    ui.colored_label(c_tm(), "Load a playbook first (Run tab в†’ рџ“ќ Load pb)");
                }
            });
    }

    pub fn show_terraform(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("tf_side")
            .default_width(350.0).min_width(280.0)
            .frame(Frame::none().inner_margin(Margin::same(14.0)))
            .show(ctx, |ui| {
                ui.heading(RichText::new("рџЊЌ Terraform").color(c_purple()));
                ui.add_space(8.0);
                let mut wd = self.tf.workdir.to_string_lossy().into_owned();
                ui.label(RichText::new("Working directory").color(c_tm()));
                ui.horizontal(|ui| {
                    ui.text_edit_singleline(&mut wd);
                    if ui.button("рџ“‚").clicked() {
                        if let Some(path) = rfd::FileDialog::new().pick_folder() {
                            wd = path.to_string_lossy().into_owned();
                        }
                    }
                });
                self.tf.workdir = PathBuf::from(wd);
                ui.checkbox(&mut self.tf.auto_approve, "Auto-approve");
                ui.add_space(12.0);
                ui.horizontal_wrapped(|ui| {
                    if ui.button("рџ”§ init").clicked() {
                        self.start_tf("init");
                    }
                    if ui.button("рџ“‹ plan").clicked() {
                        self.start_tf("plan");
                    }
                    if ui.button("в–¶ apply").clicked() {
                        self.start_tf("apply");
                    }
                    if ui.button("рџ’Ј destroy").clicked() {
                        self.start_tf("destroy");
                    }
                });
            });
        egui::CentralPanel::default()
            .frame(Frame::none().inner_margin(Margin::same(14.0)))
            .show(ctx, |ui| {
                ui.heading(RichText::new("рџ“њ Terraform Output").color(c_purple()));
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
            ui.heading(RichText::new("рџ§° РРЅСЃС‚СЂСѓРјРµРЅС‚С‹").color(c_purple()));
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.collapsing("рџ©є Р”РёР°РіРЅРѕСЃС‚РёРєР° (ping / ssh / РїРѕСЂС‚)", |ui| {
                    ui.label(RichText::new("РџСЂРѕРІРµСЂСЏРµС‚ РєР°Р¶РґС‹Р№ С…РѕСЃС‚ РёР· inventory: РґРѕСЃС‚СѓРїРµРЅ Р»Рё РїРѕ ping, РѕС‚РєСЂС‹С‚ Р»Рё SSH(22) Рё РІР°С€ РїРѕСЂС‚. Р Р°Р±РѕС‚Р°РµС‚ РІ С„РѕРЅРµ.").color(c_tm()));
                    ui.horizontal(|ui| {
                        ui.label("РџРѕСЂС‚:"); ui.text_edit_singleline(&mut self.diag_port);
                        if ui.button("в–¶ РџСЂРѕРІРµСЂРёС‚СЊ РІСЃРµ С…РѕСЃС‚С‹").clicked() {
                            if self.hosts.is_empty() {
                                match parse_inventory(std::path::Path::new(&self.inventory_path)) { Ok(h) => self.hosts = h, Err(e) => self.ui.error_message = Some(format!("Ошибка загрузки инвентаря: {e}")) }
                            }
                            if self.hosts.is_empty() {
                                self.toast("вљ пёЏ РќРµС‚ С…РѕСЃС‚РѕРІ: Р·Р°РіСЂСѓР·РёС‚Рµ inventory".into());
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
                                self.toast("рџ©є Р”РёР°РіРЅРѕСЃС‚РёРєР° Р·Р°РїСѓС‰РµРЅР°...".into());
                            }
                        }
                    });
                    for r in &self.diag_results {
                        ui.horizontal(|ui| {
                            ui.colored_label(if r.ping_ok { c_green() } else { c_red() }, if r.ping_ok { "в—Џ ping" } else { "в—‹ ping" });
                            ui.colored_label(if r.ssh_ok { c_green() } else { c_red() }, if r.ssh_ok { "в—Џ ssh:22" } else { "в—‹ ssh:22" });
                            ui.colored_label(if r.port_ok { c_green() } else { c_red() }, format!("{} port:{}", if r.port_ok { "в—Џ" } else { "в—‹" }, r.port));
                            ui.label(&r.host);
                        });
                    }
                });

                ui.collapsing("рџ”ђ Ansible Vault", |ui| {
                    ui.label(RichText::new("РЁРёС„СЂРѕРІР°РЅРёРµ СЃРµРєСЂРµС‚РѕРІ. Р’С‹Р±РµСЂРёС‚Рµ С„Р°Р№Р» в†’ рџ”’ Encrypt РёР»Рё рџ”“ Decrypt. РџСѓС‚СЊ Рє РїР°СЂРѕР»СЊ-С„Р°Р№Р»Сѓ вЂ” РІ РїРѕР»Рµ СЂСЏРґРѕРј.").color(c_tm()));
                    ui.horizontal(|ui| {
                        ui.text_edit_singleline(&mut self.vault_file);
                        if ui.button("рџ“‚").clicked() { if let Some(p) = rfd::FileDialog::new().pick_file() { self.vault_file = p.to_string_lossy().into_owned(); } }
                        ui.text_edit_singleline(&mut self.vault_pass);
                    });
                    ui.horizontal(|ui| {
                        let pf = if self.vault_pass.is_empty() { None } else { Some(self.vault_pass.as_str()) };
                        if ui.button("рџ”’ Encrypt").clicked() { self.vault_out = vault::run("encrypt", &self.vault_file, pf).unwrap_or_else(|e| e); }
                        if ui.button("рџ”“ Decrypt").clicked() { self.vault_out = vault::run("decrypt", &self.vault_file, pf).unwrap_or_else(|e| e); }
                    });
                    ui.label(RichText::new(&self.vault_out).monospace().small());
                });

                ui.collapsing("рџ“¦ Р›РѕРєР°Р»СЊРЅС‹Р№ RPM-СЂРµРїРѕР·РёС‚РѕСЂРёР№", |ui| {
                    ui.label(RichText::new("РџР°РїРєР° СЃ .rpm. РљРЅРѕРїРєР° createrepo СЃРѕР·РґР°С‘С‚ repodata вЂ” С…РѕСЃС‚С‹ Р Р•Р”РћРЎ СЃС‚Р°РІСЏС‚ РїР°РєРµС‚С‹ РїРѕ СЃРµС‚Рё Р±РµР· РёРЅС‚РµСЂРЅРµС‚Р°.").color(c_tm()));
                    ui.horizontal(|ui| {
                        ui.text_edit_singleline(&mut self.rpm_git.rpm_dir);
                        if ui.button("рџ“‚").clicked() { if let Some(p) = rfd::FileDialog::new().pick_folder() { self.rpm_git.rpm_dir = p.to_string_lossy().into_owned(); } }
                        if ui.button("рџ›  createrepo").clicked() { self.rpm_git.rpm_out = rpmrepo::createrepo(&self.rpm_git.rpm_dir).unwrap_or_else(|e| e); self.toast("рџ“¦ repodata СЃРѕР·РґР°РЅР°".into()); }
                    });
                    let rpms = rpmrepo::list_rpms(&self.rpm_git.rpm_dir);
                    ui.label(format!("РџР°РєРµС‚РѕРІ: {}", rpms.len()));
                    for r in rpms.iter().take(20) { ui.label(format!("  вЂў {}", r)); }
                    ui.label(RichText::new(&self.rpm_git.rpm_out).monospace().small());
                });

                ui.collapsing("рџ”— Git (РІРµСЂСЃРёРѕРЅРёСЂРѕРІР°РЅРёРµ РїР»РµР№Р±СѓРєРѕРІ)", |ui| {
                    ui.label(RichText::new("РЎРёСЃС‚РµРјРЅС‹Р№ git РІ РІС‹Р±СЂР°РЅРЅРѕР№ РїР°РїРєРµ: status/log/commit вЂ” РѕС‚РєР°С‚ РёР·РјРµРЅРµРЅРёР№ Рё РєРѕРјР°РЅРґРЅР°СЏ СЂР°Р±РѕС‚Р°.").color(c_tm()));
                    ui.horizontal(|ui| {
                        ui.text_edit_singleline(&mut self.rpm_git.git_dir);
                        if ui.button("рџ“‚").clicked() { if let Some(p) = rfd::FileDialog::new().pick_folder() { self.rpm_git.git_dir = p.to_string_lossy().into_owned(); } }
                    });
                    ui.horizontal(|ui| {
                        if ui.button("рџ“Љ status").clicked() { self.rpm_git.git_out = gitops::run(&self.rpm_git.git_dir, &["status", "--short"]); }
                        if ui.button("рџ“њ log").clicked() { self.rpm_git.git_out = gitops::run(&self.rpm_git.git_dir, &["log", "--oneline", "-5"]); }
                        ui.text_edit_singleline(&mut self.rpm_git.git_msg);
                        if ui.button("рџ’ѕ commit all").clicked() {
                            gitops::run(&self.rpm_git.git_dir, &["add", "-A"]);
                            self.rpm_git.git_out = gitops::run(&self.rpm_git.git_dir, &["commit", "-m", &self.rpm_git.git_msg]);
                            self.toast("рџ”— РљРѕРјРјРёС‚ СЃРѕР·РґР°РЅ".into());
                        }
                    });
                    ui.label(RichText::new(&self.rpm_git.git_out).monospace().small());
                });

                ui.collapsing("рџ•ђ РџР»Р°РЅРёСЂРѕРІС‰РёРє Р·Р°РґР°С‡", |ui| {
                    ui.label(RichText::new("РџРѕРєР° РїСЂРёР»РѕР¶РµРЅРёРµ РѕС‚РєСЂС‹С‚Рѕ, playbook Р·Р°РїСѓСЃРєР°РµС‚СЃСЏ РєР°Р¶РґС‹Рµ N РјРёРЅСѓС‚.").color(c_tm()));
                    if ui.button("вћ• Р”РѕР±Р°РІРёС‚СЊ С‚РµРєСѓС‰РёР№ playbook").clicked() {
                        self.scheduler_compliance.sched_tasks.push(SchedTask { playbook: self.playbook_path.clone(), inventory: self.inventory_path.clone(), interval_min: 60, last_run: None });
                    }
                    let mut remove: Vec<usize> = Vec::new();
                    for (i, t) in self.scheduler_compliance.sched_tasks.iter_mut().enumerate() {
                        ui.horizontal(|ui| {
                            ui.add(egui::DragValue::new(&mut t.interval_min).range(1..=1440).suffix(" РјРёРЅ"));
                            ui.label(&t.playbook);
                            if ui.button("рџ—‘пёЏ").clicked() { remove.push(i); }
                        });
                    }
                    for i in remove.into_iter().rev() { self.scheduler_compliance.sched_tasks.remove(i); }
                });

                ui.collapsing("рџ¤– Zabbix (РѕС‚РїСЂР°РІРєР° РјРµС‚СЂРёРє)", |ui| {
                    ui.label(RichText::new("РћС‚РїСЂР°РІРєР° РјРµС‚СЂРёРєРё РїРѕ РїСЂРѕС‚РѕРєРѕР»Сѓ Zabbix sender (host/key/value).").color(c_tm()));
                    ui.horizontal(|ui| {
                        ui.label("Server:"); ui.text_edit_singleline(&mut self.zabbix.server);
                        ui.label("Port:"); ui.text_edit_singleline(&mut self.zabbix.port);
                    });
                    if ui.button("рџ“Ў РћС‚РїСЂР°РІРёС‚СЊ С‚РµСЃС‚РѕРІСѓСЋ РјРµС‚СЂРёРєСѓ").clicked() {
                        let port: u16 = self.zabbix.port.parse().unwrap_or(10051);
                        self.zabbix.output = zabbix::send(&self.zabbix.server, port, "ansibleops", "ansibleops.status", "1").unwrap_or_else(|e| format!("error: {}", e));
                        self.toast("рџ¤– РњРµС‚СЂРёРєР° РѕС‚РїСЂР°РІР»РµРЅР°".into());
                    }
                    ui.label(RichText::new(&self.zabbix.output).monospace().small());
                });

                ui.collapsing("рџ“Ў REST API / РІРµР±-СЂРµР¶РёРј", |ui| {
                    ui.label(RichText::new("Read-only API: СЃС‚Р°С‚СѓСЃ Рё /api/history (JSON). РўСЂРµР±СѓРµС‚СЃСЏ bearer-С‚РѕРєРµРЅ.").color(c_tm()));
                    ui.label(RichText::new("вљ пёЏ РќРµ Р·Р°РїСѓСЃРєР°Р№С‚Рµ РЅР° 0.0.0.0 Р±РµР· РѕРіСЂР°РЅРёС‡РµРЅРёСЏ РґРѕСЃС‚СѓРїР°: API РЅРµ РёРјРµРµС‚ TLS Рё RBAC.").color(c_orange()));
                    ui.horizontal(|ui| {
                        ui.label("Bind:");
                        ui.add(egui::TextEdit::singleline(&mut self.web.bind).desired_width(110.0));
                        ui.label("РџРѕСЂС‚:");
                        ui.add(egui::TextEdit::singleline(&mut self.web.api_port).desired_width(60.0));
                    });
                    ui.horizontal(|ui| {
                        ui.label("РўРѕРєРµРЅ:");
                        ui.monospace(&self.web.token);
                        if ui.button("рџ”„").clicked() { self.web.token = webapi::generate_token(); }
                    });
                    if !self.web.enabled {
                        if ui.button("рџЊђ Р—Р°РїСѓСЃС‚РёС‚СЊ Web API").clicked() {
                            let dir = self.history_store.clone().map(|s| s.dir.to_string_lossy().into_owned()).unwrap_or_default();
                            let port = self.web.api_port.parse::<u16>().unwrap_or(8080);
                            match webapi::start(port, &self.web.bind, &self.web.token, dir) {
                                Ok(_) => { self.web.enabled = true; self.toast(format!("рџЊђ Web API: http://{}:{}", self.web.bind, port)); }
                                Err(e) => self.toast(format!("рџЊђ РћС€РёР±РєР°: {}", e)),
                            }
                        }
                    } else {
                        ui.colored_label(c_green(), format!("в—Џ Web API Р°РєС‚РёРІРµРЅ: http://{}:{} (РґРѕСЃС‚СѓРї С‚РѕР»СЊРєРѕ СЃ С‚РѕРєРµРЅРѕРј)", self.web.bind, self.web.api_port));
                    }
                });

                ui.collapsing("рџ“„ Р­РєСЃРїРѕСЂС‚ РѕС‚С‡С‘С‚Р° (HTML / JSON / CSV)", |ui| {
                    ui.label(RichText::new("Р¤РѕСЂРјР°С‚ в†’ 'Р­РєСЃРїРѕСЂС‚РёСЂРѕРІР°С‚СЊ' в†’ РґРёР°Р»РѕРі РІС‹Р±РѕСЂР° РјРµСЃС‚Р° СЃРѕС…СЂР°РЅРµРЅРёСЏ.").color(c_tm()));
                    ui.horizontal(|ui| {
                        ui.label("Р¤РѕСЂРјР°С‚:");
                        egui::ComboBox::from_id_salt("rep_fmt").selected_text(self.report_format.to_uppercase()).show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.report_format, "html".into(), "HTML");
                            ui.selectable_value(&mut self.report_format, "json".into(), "JSON");
                            ui.selectable_value(&mut self.report_format, "csv".into(), "CSV");
                        });
                        if ui.button("рџ“„ Р­РєСЃРїРѕСЂС‚РёСЂРѕРІР°С‚СЊ...").clicked() {
                            let ext = self.report_format.clone();
                            let content = {
                                let s = self.session.lock().unwrap();
                                match ext.as_str() { "json" => report::to_json(&s), "csv" => report::to_csv(&s), _ => report::to_html(&s) }
                            };
                            let name = format!("report_{}.{}", chrono::Local::now().format("%Y%m%d_%H%M%S"), ext);
                            if let Some(path) = rfd::FileDialog::new().set_file_name(&name).add_filter("Report", &[ext.as_str()]).save_file() {
                                match std::fs::write(&path, content) {
                                    Ok(_) => self.toast(format!("рџ“„ РЎРѕС…СЂР°РЅРµРЅРѕ: {}", path.display())),
                                    Err(e) => self.toast(format!("РћС€РёР±РєР°: {}", e)),
                                }
                            }
                        }
                    });
                });

                ui.collapsing("рџ“Ў Event-РїСЂР°РІРёР»Р° (Р°РІС‚РѕСЂРµР°РєС†РёСЏ)", |ui| {
                    ui.label(RichText::new("РљР°Р¶РґС‹Рµ 60 СЃРµРє РїСЂРѕРІРµСЂСЏРµС‚ СѓСЃР»РѕРІРёСЏ; РїСЂРё РЅР°СЂСѓС€РµРЅРёРё вЂ” СѓРІРµРґРѕРјР»РµРЅРёРµ Рё/РёР»Рё РґРµР№СЃС‚РІРёРµ (СЃ cooldown, Р»РёРјРёС‚РѕРј Рё РїРѕРґС‚РІРµСЂР¶РґРµРЅРёРµРј).").color(c_tm()));
                    ui.label(RichText::new("вљ пёЏ РџСЂРѕРёР·РІРѕР»СЊРЅС‹Р№ shell РІ РґРµР№СЃС‚РІРёРё РІСЃРµРіРґР° С‚СЂРµР±СѓРµС‚ СЂСѓС‡РЅРѕРіРѕ РїРѕРґС‚РІРµСЂР¶РґРµРЅРёСЏ.").color(c_orange()));
                    if ui.button("вћ• РџСЂР°РІРёР»Рѕ").clicked() {
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
                            ui.label("РґРµР№СЃС‚РІРёРµ:");
                            ui.text_edit_singleline(&mut r.action);
                            if ui.button("рџ—‘").clicked() { rm.push(i); }
                        });
                        ui.horizontal(|ui| {
                            ui.add(egui::DragValue::new(&mut r.cooldown_secs).range(0..=86400).suffix(" СЃРµРє cooldown"));
                            ui.add(egui::DragValue::new(&mut r.max_runs).range(0..=100).prefix("РјР°РєСЃ. Р·Р°РїСѓСЃРєРѕРІ: "));
                            ui.text_edit_singleline(&mut r.allowed_hosts);
                            ui.label("С…РѕСЃС‚С‹ (РїСѓСЃС‚Рѕ = РІСЃРµ)");
                            ui.checkbox(&mut r.require_confirm, "РїРѕРґС‚РІРµСЂР¶РґРµРЅРёРµ");
                        });
                    }
                    for i in rm.into_iter().rev() { self.ops.rules.remove(i); }
                    // РћРєРЅРѕ РїРѕРґС‚РІРµСЂР¶РґРµРЅРёСЏ РѕР¶РёРґР°СЋС‰РёС… РґРµР№СЃС‚РІРёР№
                    if !self.ops.pending_rules.is_empty() {
                        ui.label(RichText::new(format!("вЏё РћР¶РёРґР°СЋС‚ РїРѕРґС‚РІРµСЂР¶РґРµРЅРёСЏ: {}", self.ops.pending_rules.len())).color(c_orange()).strong());
                    }
                    let mut approve: Vec<usize> = Vec::new();
                    for (i, r) in self.ops.pending_rules.iter_mut().enumerate() {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(format!("[{}] {}", r.kind, r.action)).monospace().color(c_tx()));
                            if ui.button("вњ… Р’С‹РїРѕР»РЅРёС‚СЊ").clicked() { approve.push(i); }
                            if ui.button("вќЊ РћС‚РєР»РѕРЅРёС‚СЊ").clicked() { r.rejected = true; self.ops.rule_log.push(format!("в›” Р”РµР№СЃС‚РІРёРµ РѕС‚РєР»РѕРЅРµРЅРѕ: [{}] {}", r.kind, r.action)); }
                        });
                    }
                    for (n, i) in approve.into_iter().enumerate() {
                        let r = self.ops.pending_rules.remove(i - n);
                        if let Some(rule) = self.ops.rules.iter_mut().find(|x| x.kind == r.kind && x.param == r.param) { rule.runs += 1; rule.last_run = Some(std::time::Instant::now()); }
                        let target = if r.allowed_hosts.is_empty() { self.ops.hosts.clone() } else { r.allowed_hosts.clone() };
                        self.ops.rule_log.push(format!("вњ… Р’С‹РїРѕР»РЅРµРЅРѕ (РїРѕРґС‚РІРµСЂР¶РґРµРЅРѕ): [{}] {} РЅР° {}", r.kind, r.action, target));
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
                    ui.heading(RichText::new("рџ—‚ Run History").color(c_purple()));
                    if ui.button("рџ”„ Refresh").clicked() {
                        self.refresh_history();
                    }
                    if !self.history.is_empty()
                        && ui.button("рџ—„ РђСЂС…РёРІРёСЂРѕРІР°С‚СЊ РІСЃС‘ (retention)").clicked()
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
                                        RunState::Finished { success: true } => ("вњ“", c_green()),
                                        RunState::Finished { success: false } => ("вњ—", c_red()),
                                        _ => ("в—Џ", c_orange()),
                                    };
                                    ui.label(RichText::new(ic).color(cc).strong().size(18.0));
                                    ui.vertical(|ui| {
                                        ui.label(RichText::new(&s.playbook).strong().color(c_tx()));
                                        ui.label(
                                            RichText::new(format!(
                                                "{} в†’ {}",
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
                                        if ui.button("рџ—„пёЏ").clicked() {
                                            if let Some(store) = &self.history_store {
                                                let _ = store.delete(&s.id);
                                                self.history.retain(|x| x.id != s.id);
                                            }
                                        }
                                    });
                                });
                                ui.collapsing("рџ“‹ Р”РµС‚Р°Р»Рё Р·Р°РїСѓСЃРєР°", |ui| {
                                    if s.events.is_empty() {
                                        ui.colored_label(c_tm(), "РЎРѕР±С‹С‚РёСЏ РЅРµ СЃРѕС…СЂР°РЅРµРЅС‹");
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
            ui.heading(RichText::new("рџ”Ќ Diff Рё РІРµСЂСЃРёРё С„Р°Р№Р»РѕРІ").color(c_purple()));
            ui.label(RichText::new("Dry-run playbook Рё РїСЂРѕСЃРјРѕС‚СЂ РёР·РјРµРЅРµРЅРёР№. РђРІС‚Рѕ-СЃРѕС…СЂР°РЅРµРЅРёРµ РІРµСЂСЃРёР№ РєРѕРЅС„РёРіРѕРІ РїРµСЂРµРґ РїСЂРёРјРµРЅРµРЅРёРµРј.").color(c_tm()));
            ui.add_space(8.0);
            ui.collapsing("в–¶ Р—Р°РїСѓСЃС‚РёС‚СЊ dry-run С‚РµРєСѓС‰РµРіРѕ playbook", |ui| {
                ui.label(RichText::new("Р’С‹РїРѕР»РЅСЏРµС‚ ansible-playbook --check --diff Рё РїР°СЂСЃРёС‚ РІС‹РІРѕРґ РІ СЃРїРёСЃРѕРє РёР·РјРµРЅРµРЅРёР№.").color(c_tm()));
                if ui.add_enabled(!self.diff_versions.diff.scan_running, egui::Button::new(if self.diff_versions.diff.scan_running { "вЏі РЎРєР°РЅРёСЂРѕРІР°РЅРёРµ..." } else { "рџ”Ќ Р—Р°РїСѓСЃС‚РёС‚СЊ diff-СЃРєР°РЅРёСЂРѕРІР°РЅРёРµ" })).clicked() {
                    self.diff_versions.diff.scan_running = true;
                    self.diff_versions.diff.changes.clear();
                    let pb = self.playbook_path.clone();
                    let inv = self.inventory_path.clone();
                    let (tx, rx) = std::sync::mpsc::channel();
                    self.diff_versions.diff_rx = Some(rx);
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
            ui.collapsing("рџ“‘ РСЃС‚РѕСЂРёСЏ РІРµСЂСЃРёР№ С„Р°Р№Р»РѕРІ", |ui| {
                ui.label(RichText::new("Р’С‹Р±РµСЂРёС‚Рµ С…РѕСЃС‚ Рё С„Р°Р№Р», С‡С‚РѕР±С‹ СѓРІРёРґРµС‚СЊ РІСЃРµ СЃРѕС…СЂР°РЅС‘РЅРЅС‹Рµ РІРµСЂСЃРёРё.").color(c_tm()));
                ui.horizontal(|ui| {
                    ui.label("РҐРѕСЃС‚:"); ui.text_edit_singleline(&mut self.diff_versions.diff.selected_host);
                    ui.label("Р¤Р°Р№Р»:"); ui.text_edit_singleline(&mut self.diff_versions.diff.selected_file);
                    if ui.button("рџ”„ РџРѕРєР°Р·Р°С‚СЊ").clicked() {
                        if let Some(store) = &self.diff_versions.versions {
                            self.diff_versions.diff.versions = store.list(&self.diff_versions.diff.selected_host, &self.diff_versions.diff.selected_file);
                        }
                    }
                    if ui.button("в¬‡ РЎРєР°С‡Р°С‚СЊ Рё СЃРѕС…СЂР°РЅРёС‚СЊ").clicked()
                        && !self.diff_versions.diff.selected_host.is_empty()
                        && !self.diff_versions.diff.selected_file.is_empty()
                    {
                        match diff_mod::VersionStore::fetch_remote(&self.diff_versions.diff.selected_host, &self.diff_versions.diff.selected_file, &self.inventory_path) {
                            Ok(content) => {
                                let result = {
                                    if let Some(store) = &self.diff_versions.versions {
                                        match store.save(&self.diff_versions.diff.selected_host, &self.diff_versions.diff.selected_file, &content, "manual fetch") {
                                            Ok(_) => {
                                                let versions = store.list(&self.diff_versions.diff.selected_host, &self.diff_versions.diff.selected_file);
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
                                        self.toast("рџ’ѕ Р’РµСЂСЃРёСЏ СЃРѕС…СЂР°РЅРµРЅР°".into());
                                        self.diff_versions.diff.versions = versions;
                                    } else {
                                        self.toast(format!("РћС€РёР±РєР°: {}", err));
                                    }
                                }
                            }
                            Err(e) => self.toast(format!("РќРµ СѓРґР°Р»РѕСЃСЊ СЃРєР°С‡Р°С‚СЊ: {}", e)),
                        }
                    }
                });
                ui.add_space(6.0);
                egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                    let versions = self.diff_versions.diff.versions.clone();
                    for v in &versions {
                        Frame::none().fill(c_surf()).rounding(Rounding::same(8.0)).inner_margin(Margin::same(8.0)).show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(v.saved_at.format("%Y-%m-%d %H:%M:%S").to_string()).color(c_cyan()).strong());
                                ui.label(format!("[{}]", v.source));
                                if ui.button("в¬† Р’РѕСЃСЃС‚Р°РЅРѕРІРёС‚СЊ РЅР° С…РѕСЃС‚").clicked() {
                                    let b64 = general_purpose::STANDARD.encode(v.content.as_bytes());
                                    let host = self.diff_versions.diff.selected_host.clone();
                                    let file = self.diff_versions.diff.selected_file.clone();
                                    let (tx, rx) = std::sync::mpsc::channel();
                                    self.ops_rx = Some(rx);
                                    std::thread::spawn(move || {
                                        let cmd = format!("echo '{}' | base64 -d > {}", b64, file);
                                        crate::ansible::adhoc::run_raw(&host, "shell", &cmd, true, tx);
                                    });
                                    self.toast("в¬† Р’РѕСЃСЃС‚Р°РЅРѕРІР»РµРЅРёРµ Р·Р°РїСѓС‰РµРЅРѕ".into());
                                }
                            });
                            ui.collapsing("РЎРѕРґРµСЂР¶РёРјРѕРµ", |ui| {
                                ui.add(egui::TextEdit::multiline(&mut {v.content.clone()})
                                    .font(egui::FontId::monospace(12.0)).desired_width(f64::INFINITY as f32).desired_rows(12));
                            });
                        });
                        ui.add_space(3.0);
                    }
                    if self.diff_versions.diff.versions.is_empty() { ui.colored_label(c_tm(), "РќРµС‚ СЃРѕС…СЂР°РЅС‘РЅРЅС‹С… РІРµСЂСЃРёР№"); }
                });
            });
            ui.add_space(8.0);
            ui.label(RichText::new("рџ“‹ Р РµР·СѓР»СЊС‚Р°С‚ РїРѕСЃР»РµРґРЅРµРіРѕ diff-СЃРєР°РЅРёСЂРѕРІР°РЅРёСЏ:").strong());
            egui::ScrollArea::vertical().show(ui, |ui| {
                if self.diff_versions.diff.changes.is_empty() {
                    ui.colored_label(c_tm(), "РќРµС‚ РёР·РјРµРЅРµРЅРёР№ РёР»Рё СЃРєР°РЅРёСЂРѕРІР°РЅРёРµ РЅРµ Р·Р°РїСѓСЃРєР°Р»РѕСЃСЊ");
                }
                for c in &self.diff_versions.diff.changes {
                    Frame::none().fill(c_surf()).rounding(Rounding::same(8.0)).inner_margin(Margin::same(8.0)).show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(&c.host).color(c_cyan()).strong());
                            ui.label(RichText::new(&c.path).color(c_tx()));
                            ui.colored_label(c_green(), format!("+{}", c.added_lines));
                            ui.colored_label(c_red(), format!("-{}", c.removed_lines));
                        });
                        ui.collapsing("Diff", |ui| {
                            if !c.before.is_empty() {
                                ui.label(RichText::new("вЂ” РґРѕ:").color(c_red()));
                                ui.add(egui::TextEdit::multiline(&mut {c.before.clone()}).font(egui::FontId::monospace(11.0)).desired_width(f64::INFINITY as f32));
                            }
                            if !c.after.is_empty() {
                                ui.label(RichText::new("вЂ” РїРѕСЃР»Рµ:").color(c_green()));
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
            ui.heading(RichText::new("рџ“¦ РљР°С‚Р°Р»РѕРі СЂРѕР»РµР№").color(c_purple()));
            ui.label(RichText::new("Р“РѕС‚РѕРІС‹Рµ СЂРѕР»Рё СЃ РјРµС‚Р°РґР°РЅРЅС‹РјРё: РІРµСЂСЃРёСЏ, СЃРѕРІРјРµСЃС‚РёРјРѕСЃС‚СЊ, СЂРёСЃРє, С‚РµРіРё, РѕС‚РєР°С‚. Р“РµРЅРµСЂРёСЂСѓСЋС‚ playbook РІ 1 РєР»РёРє.").color(c_tm()));
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.label("Р¤РёР»СЊС‚СЂ РїРѕ С‚РµРіСѓ:");
                egui::ComboBox::from_id_salt("role_tag").selected_text(if self.roles.filter_tag.is_empty() { "РІСЃРµ" } else { &self.roles.filter_tag }).show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.roles.filter_tag, String::new(), "РІСЃРµ");
                    for t in roles_mod::role_tags_list() {
                        ui.selectable_value(&mut self.roles.filter_tag, t.to_string(), t);
                    }
                });
                ui.label("РҐРѕСЃС‚С‹ РґР»СЏ deploy:");
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
                                    let risk_col = match r.risk { "РІС‹СЃРѕРєРёР№" => c_red(), "СЃСЂРµРґРЅРёР№" => c_orange(), _ => c_green() };
                                    ui.colored_label(risk_col, format!("[risk: {}]", r.risk));
                                });
                                ui.label(RichText::new(r.description).color(c_tx()));
                                ui.label(RichText::new(format!("РЎРѕРІРјРµСЃС‚РёРјРѕСЃС‚СЊ: {}", r.compat.join(", "))).color(c_blue()));
                                ui.label(RichText::new(format!("РўРµРіРё: {} вЂў РђРІС‚РѕСЂ: {}", r.tags.join(", "), r.author)).color(c_tm()));
                            });
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if ui.button("в–¶ Deploy").clicked() {
                                    let yaml = roles_mod::role_to_playbook(&r, &self.roles.target_hosts);
                                    self.editor.content = yaml.clone();
                                    if let Some(path) = rfd::FileDialog::new().set_file_name(format!("{}.yml", r.name)).add_filter("YAML", &["yml"]).save_file() {
                                        if std::fs::write(&path, yaml).is_ok() {
                                            self.playbook_path = path.to_string_lossy().into_owned();
                                            self.start_run();
                                            self.toast(format!("рџљЂ Р РѕР»СЊ {} Р·Р°РїСѓС‰РµРЅР°", r.name));
                                        }
                                    }
                                }
                                if ui.button("рџ“‹ Р’ Editor").clicked() {
                                    self.editor.content = roles_mod::role_to_playbook(&r, &self.roles.target_hosts);
                                    self.tab = Tab::Editor;
                                }
                            });
                        });
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            if r.requires_confirm { ui.colored_label(c_orange(), "вљ  С‚СЂРµР±СѓРµС‚ РїРѕРґС‚РІРµСЂР¶РґРµРЅРёСЏ"); }
                            if r.has_rollback { ui.colored_label(c_green(), "в†© РµСЃС‚СЊ РѕС‚РєР°С‚"); }
                        });
                    });
                    ui.add_space(6.0);
                }
            });
        });
    }
}




