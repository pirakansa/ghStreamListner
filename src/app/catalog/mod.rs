//! The complete shared application UI backed by session-only sample data.

use eframe::egui;

use super::{effects::ExternalEffect, screens, AppMode, GhStreamApp, Runtime, StatusEntry};
use crate::models::{AppConfig, LibraryView, Selection};
use crate::saved_query_io::{ImportedFilterStream, ImportedSavedQuery};
use crate::storage::{Result, Storage};

mod fixtures;
mod transfer;

pub struct CatalogApp {
    app: GhStreamApp,
    exported_queries: Option<Vec<ImportedSavedQuery>>,
}

impl CatalogApp {
    pub fn new() -> Result<Self> {
        let mut config = AppConfig::default_with_pat("demo-token".into());
        config.host.hostname = "github.example.test".into();
        config.host.name = "Sample host".into();
        config.host.kind = crate::models::HostKind::Ghes;
        let storage = Storage::in_memory()?;
        let host_id = storage.ensure_host(&config.host)?;
        fixtures::seed(&storage, host_id)?;
        let status = "Demo: sample data; all changes stay in memory.".to_owned();
        let mut app = GhStreamApp {
            config_path: Default::default(),
            database_path: Default::default(),
            setup: screens::setup::SetupState::from_config(&config),
            stream: Default::default(),
            mode: AppMode::Main(Box::new(Runtime {
                config,
                storage,
                host_id,
                library_counts: Default::default(),
                saved_queries: Vec::new(),
                items: Vec::new(),
            })),
            status: status.clone(),
            status_history: vec![StatusEntry::new(status)],
            last_poll_at: None,
            refresh_rx: None,
        };
        app.reload_queries();
        app.reload_current_view();
        Ok(Self {
            app,
            exported_queries: None,
        })
    }

    pub fn show(&mut self, ui: &mut egui::Ui) {
        if let Some(effect) = self.app.show(ui) {
            if let Err(error) = self.execute_effect(effect) {
                GhStreamApp::replace_status_error(
                    &mut self.app.status,
                    &mut self.app.status_history,
                    format!("Demo operation failed: {error}"),
                );
            }
        }
    }

    fn status(&mut self, message: &str) {
        GhStreamApp::replace_status(&mut self.app.status, &mut self.app.status_history, message);
    }

    fn execute_effect(&mut self, effect: ExternalEffect) -> Result<()> {
        match effect {
            ExternalEffect::TestConnection(_) => {
                self.app.setup.set_validation_message(
                    "Demo connection succeeded (simulated; no network request).".into(),
                );
            }
            ExternalEffect::SaveSetup(config) => {
                let mode = std::mem::replace(
                    &mut self.app.mode,
                    AppMode::Setup {
                        previous_runtime: None,
                    },
                );
                if let AppMode::Setup {
                    previous_runtime: Some(mut runtime),
                } = mode
                {
                    runtime.config.host = config.host;
                    runtime.config.auth = config.auth;
                    self.app.mode = AppMode::Main(runtime);
                    self.status("Demo host settings saved in memory.");
                } else {
                    self.app.mode = mode;
                }
            }
            ExternalEffect::Refresh => {
                if let AppMode::Main(runtime) = &self.app.mode {
                    fixtures::refresh_matches(&runtime.storage, runtime.host_id)?;
                }
                self.app.reload_queries();
                self.app.reload_current_view();
                self.status("Demo refresh completed with sample results; no network request.");
            }
            ExternalEffect::OpenItem { id, .. } => {
                self.app
                    .item_action(screens::stream::ItemAction::MarkRead(id));
                self.status("Demo item opening simulated; no browser was opened.");
            }
            ExternalEffect::PreviewQuery { .. } => {
                self.status("Demo query preview simulated; no browser was opened.");
            }
            ExternalEffect::ExportQueries(path) => self.export_queries(&path),
            ExternalEffect::ImportQueries(path) => self.import_queries(&path)?,
            ExternalEffect::SetDefaultSort(sort) => {
                if let AppMode::Main(runtime) = &mut self.app.mode {
                    runtime.config.ui.default_sort = sort;
                }
                self.app.reload_current_view();
                self.status("Demo sort preference saved in memory.");
            }
            ExternalEffect::SetPollingInterval(seconds) => {
                if let AppMode::Main(runtime) = &mut self.app.mode {
                    runtime.config.refresh.polling_interval_seconds = seconds;
                }
                self.app.stream.polling_interval_draft = 0;
                self.status(
                    "Demo polling preference saved in memory; automatic polling is inactive.",
                );
            }
            ExternalEffect::SetTheme(theme) => {
                if let AppMode::Main(runtime) = &mut self.app.mode {
                    runtime.config.ui.theme = theme;
                }
                self.status("Demo theme saved in memory.");
            }
            ExternalEffect::SetFontSize(size) => {
                if let AppMode::Main(runtime) = &mut self.app.mode {
                    runtime.config.ui.font_size = size;
                }
                self.status("Demo font size saved in memory.");
            }
        }
        Ok(())
    }
}

impl eframe::App for CatalogApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.show(ui);
    }
}

#[cfg(test)]
mod tests;
