use eframe::egui;

use crate::config;
use crate::models::{AppConfig, HostKind, Scheme};

pub struct SetupState {
    name: String,
    scheme: Scheme,
    hostname: String,
    rest_api_base_path: String,
    kind: HostKind,
    pat: String,
    validation_message: String,
}

impl Default for SetupState {
    fn default() -> Self {
        let config = AppConfig::default_with_pat(String::new());
        Self {
            name: config.host.name,
            scheme: config.host.scheme,
            hostname: config.host.hostname,
            rest_api_base_path: config.host.rest_api_base_path,
            kind: config.host.kind,
            pat: String::new(),
            validation_message: String::new(),
        }
    }
}

pub enum SetupEvent {
    Cancel,
    Save(AppConfig),
    Test(AppConfig),
}

impl SetupState {
    pub(in crate::app) fn set_validation_message(&mut self, message: String) {
        self.validation_message = message;
    }

    pub fn from_config(config: &AppConfig) -> Self {
        Self {
            name: config.host.name.clone(),
            scheme: config.host.scheme.clone(),
            hostname: config.host.hostname.clone(),
            rest_api_base_path: config.host.rest_api_base_path.clone(),
            kind: config.host.kind.clone(),
            pat: config.auth.pat.clone(),
            validation_message: String::new(),
        }
    }
}

pub fn show(
    ui: &mut egui::Ui,
    state: &mut SetupState,
    status: &str,
    can_cancel: bool,
) -> Option<SetupEvent> {
    let mut event = None;

    egui::Panel::top("setup-toolbar").show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.heading("Host settings");
            if can_cancel {
                ui.separator();
                if ui.button("Back").clicked() {
                    event = Some(SetupEvent::Cancel);
                }
            }
        });
    });

    egui::CentralPanel::default().show(ui, |ui| {
        ui.label("Configure one GitHub or GHES host. The PAT is stored as plain text in config.yml for v1.");
        ui.add_space(12.0);

        egui::Grid::new("setup-grid")
            .num_columns(2)
            .spacing([16.0, 8.0])
            .show(ui, |ui| {
                ui.label("Host name");
                ui.text_edit_singleline(&mut state.name);
                ui.end_row();

                ui.label("Scheme");
                egui::ComboBox::from_id_salt("setup-scheme")
                    .selected_text(state.scheme.to_string())
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut state.scheme, Scheme::Https, "https");
                        ui.selectable_value(&mut state.scheme, Scheme::Http, "http");
                    });
                ui.end_row();

                ui.label("Hostname");
                ui.text_edit_singleline(&mut state.hostname);
                ui.end_row();

                ui.label("REST API base path");
                ui.text_edit_singleline(&mut state.rest_api_base_path);
                ui.end_row();

                ui.label("Host kind");
                egui::ComboBox::from_id_salt("setup-kind")
                    .selected_text(state.kind.to_string())
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut state.kind, HostKind::GitHub, "github");
                        ui.selectable_value(&mut state.kind, HostKind::Ghes, "ghes");
                    });
                ui.end_row();

                ui.label("Personal access token");
                ui.add(egui::TextEdit::singleline(&mut state.pat).password(true));
                ui.end_row();
            });

        ui.add_space(12.0);
        ui.horizontal(|ui| {
            if ui.button("Test").clicked() {
                match config::validate_config(build_config(state)) {
                    Ok(config) => event = Some(SetupEvent::Test(config)),
                    Err(err) => state.validation_message = err.to_string(),
                }
            }

            if ui.button("Save").clicked() {
                match config::validate_config(build_config(state)) {
                    Ok(config) => event = Some(SetupEvent::Save(config)),
                    Err(err) => state.validation_message = err.to_string(),
                }
            }
        });

        if !state.validation_message.is_empty() {
            ui.add_space(8.0);
            ui.label(&state.validation_message);
        }

        ui.add_space(8.0);
        ui.label(status);
    });

    event
}

fn build_config(state: &SetupState) -> AppConfig {
    let mut config = AppConfig::default_with_pat(state.pat.clone());
    config.host.name = state.name.clone();
    config.host.scheme = state.scheme.clone();
    config.host.hostname = state.hostname.clone();
    config.host.rest_api_base_path = state.rest_api_base_path.clone();
    config.host.kind = state.kind.clone();
    config
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui_kittest::{kittest::Queryable as _, Harness};

    #[test]
    fn test_button_emits_validated_config_without_connecting() {
        let mut config = AppConfig::default_with_pat("sample-token".into());
        config.host.hostname = "never-connect.example.test".into();
        config.host.kind = HostKind::Ghes;
        let mut harness = Harness::new_ui_state(
            |ui, state: &mut (SetupState, Option<SetupEvent>)| {
                if let Some(event) = show(ui, &mut state.0, "", true) {
                    state.1 = Some(event);
                }
            },
            (SetupState::from_config(&config), None),
        );
        harness.get_by_label("Test").click();
        harness.run();
        let Some(SetupEvent::Test(actual)) = &harness.state().1 else {
            panic!("test event")
        };
        assert_eq!(actual.host.hostname, config.host.hostname);
        assert_eq!(actual.auth.pat, config.auth.pat);
    }

    #[test]
    fn test_and_save_reject_invalid_config_in_shared_form() {
        let mut harness = Harness::new_ui_state(
            |ui, state: &mut (SetupState, Option<SetupEvent>)| {
                state.1 = show(ui, &mut state.0, "", true);
            },
            (SetupState::default(), None),
        );
        for label in ["Test", "Save"] {
            harness.get_by_label(label).click();
            harness.run();
            assert!(harness.state().1.is_none());
            assert!(!harness.state().0.validation_message.is_empty());
        }
    }
}
