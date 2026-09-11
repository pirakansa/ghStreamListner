pub mod catalog;
pub mod components;
mod effects;
mod filter_stream_actions;
pub mod fonts;
mod item_actions;
mod mark_read_actions;
mod preferences;
mod refresh;
mod saved_query_crud;
mod saved_query_transfer;
pub mod screens;
mod view;

use std::path::PathBuf;
use std::time::Instant;

use eframe::egui;

use crate::config;
use crate::models::{AppConfig, LibraryCounts, SavedQuery, StreamItem};
use crate::storage::Storage;

pub struct GhStreamApp {
    config_path: PathBuf,
    database_path: PathBuf,
    mode: AppMode,
    setup: screens::setup::SetupState,
    stream: screens::stream::StreamState,
    status: String,
    status_history: Vec<StatusEntry>,
    last_poll_at: Option<Instant>,
    refresh_rx: Option<std::sync::mpsc::Receiver<refresh::RefreshOutcome>>,
}

pub(super) struct Runtime {
    config: AppConfig,
    storage: Storage,
    host_id: i64,
    library_counts: LibraryCounts,
    saved_queries: Vec<SavedQuery>,
    items: Vec<StreamItem>,
}

pub(super) enum AppMode {
    Setup {
        previous_runtime: Option<Box<Runtime>>,
    },
    Main(Box<Runtime>),
}

const STATUS_HISTORY_LIMIT: usize = 200;

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub(in crate::app) enum StatusLevel {
    #[default]
    Info,
    Error,
}

pub(in crate::app) struct StatusEntry {
    pub message: String,
    pub level: StatusLevel,
}

impl StatusEntry {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            level: StatusLevel::Info,
        }
    }

    fn new_error(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            level: StatusLevel::Error,
        }
    }
}

impl GhStreamApp {
    pub fn new() -> Self {
        Self::from_paths(
            config::default_config_path(),
            config::default_database_path(),
        )
    }

    fn from_paths(config_path: PathBuf, database_path: PathBuf) -> Self {
        let (mode, status) = match config::load_config(&config_path) {
            Ok(config) => match Self::open_runtime(config, &database_path) {
                Ok(runtime) => (AppMode::Main(Box::new(runtime)), "Ready".to_owned()),
                Err(err) => (
                    AppMode::Setup {
                        previous_runtime: None,
                    },
                    format!("Database initialization failed: {err}"),
                ),
            },
            Err(err) => (
                AppMode::Setup {
                    previous_runtime: None,
                },
                first_run_status(&err),
            ),
        };
        Self::initialize(
            config_path,
            database_path,
            mode,
            screens::setup::SetupState::default(),
            status,
        )
    }

    /// Initialize the shared app from caller-owned storage without loading user files.
    fn from_storage(
        config: AppConfig,
        storage: Storage,
        status: &str,
    ) -> crate::storage::Result<Self> {
        let setup = screens::setup::SetupState::from_config(&config);
        let runtime = Self::runtime_from_storage(config, storage)?;
        Ok(Self::initialize(
            PathBuf::new(),
            PathBuf::new(),
            AppMode::Main(Box::new(runtime)),
            setup,
            status.to_owned(),
        ))
    }

    fn initialize(
        config_path: PathBuf,
        database_path: PathBuf,
        mode: AppMode,
        setup: screens::setup::SetupState,
        status: String,
    ) -> Self {
        let mut app = Self {
            config_path,
            database_path,
            mode,
            setup,
            stream: screens::stream::StreamState::default(),
            status: status.clone(),
            status_history: vec![StatusEntry::new(status)],
            last_poll_at: None,
            refresh_rx: None,
        };
        app.reload_current_view();
        app
    }

    fn open_runtime(
        config: AppConfig,
        database_path: &std::path::Path,
    ) -> crate::storage::Result<Runtime> {
        Self::runtime_from_storage(config, Storage::open(database_path)?)
    }

    fn runtime_from_storage(
        config: AppConfig,
        storage: Storage,
    ) -> crate::storage::Result<Runtime> {
        let host_id = storage.ensure_host(&config.host)?;
        let (library_counts, saved_queries) = view::load_sidebar_data(&storage, host_id)?;
        Ok(Runtime {
            config,
            storage,
            host_id,
            library_counts,
            saved_queries,
            items: Vec::new(),
        })
    }

    fn save_setup_config(&mut self, config: AppConfig) {
        match config::write_config(&self.config_path, &config) {
            Ok(()) => match Self::open_runtime(config, &self.database_path) {
                Ok(runtime) => {
                    self.mode = AppMode::Main(Box::new(runtime));
                    Self::replace_status(
                        &mut self.status,
                        &mut self.status_history,
                        "Configuration saved. PAT is stored as plain text in v1.",
                    );
                    self.reload_current_view();
                }
                Err(err) => {
                    Self::replace_status_error(
                        &mut self.status,
                        &mut self.status_history,
                        format!("Configuration saved, but database failed: {err}"),
                    );
                }
            },
            Err(err) => {
                Self::replace_status_error(
                    &mut self.status,
                    &mut self.status_history,
                    err.to_string(),
                );
            }
        }
    }

    fn open_setup_settings(&mut self) {
        let mode = std::mem::replace(
            &mut self.mode,
            AppMode::Setup {
                previous_runtime: None,
            },
        );
        match mode {
            AppMode::Main(runtime) => {
                self.setup = screens::setup::SetupState::from_config(&runtime.config);
                self.mode = AppMode::Setup {
                    previous_runtime: Some(runtime),
                };
                Self::replace_status(
                    &mut self.status,
                    &mut self.status_history,
                    "Editing host settings.",
                );
            }
            setup @ AppMode::Setup { .. } => {
                self.mode = setup;
            }
        }
    }

    fn cancel_setup(&mut self, previous_runtime: Option<Box<Runtime>>) {
        if let Some(runtime) = previous_runtime {
            self.mode = AppMode::Main(runtime);
            Self::replace_status(
                &mut self.status,
                &mut self.status_history,
                "Host settings unchanged.",
            );
            self.reload_current_view();
        } else {
            self.mode = AppMode::Setup {
                previous_runtime: None,
            };
        }
    }

    fn replace_status(
        status: &mut String,
        status_history: &mut Vec<StatusEntry>,
        value: impl Into<String>,
    ) {
        Self::push_entry(status, status_history, StatusEntry::new(value));
    }

    fn replace_status_error(
        status: &mut String,
        status_history: &mut Vec<StatusEntry>,
        value: impl Into<String>,
    ) {
        Self::push_entry(status, status_history, StatusEntry::new_error(value));
    }

    fn push_entry(status: &mut String, status_history: &mut Vec<StatusEntry>, entry: StatusEntry) {
        *status = entry.message.clone();
        status_history.push(entry);
        if status_history.len() > STATUS_HISTORY_LIMIT {
            let overflow = status_history.len() - STATUS_HISTORY_LIMIT;
            status_history.drain(0..overflow);
        }
    }
}

impl Default for GhStreamApp {
    fn default() -> Self {
        Self::new()
    }
}

impl eframe::App for GhStreamApp {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_refresh_result();
        self.maybe_poll(ctx);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if let Some(effect) = self.show(ui) {
            self.execute_effect(ui.ctx(), effect);
        }
    }
}

impl GhStreamApp {
    /// Draw the shared application and handle local actions, returning external work.
    fn show(&mut self, ui: &mut egui::Ui) -> Option<effects::ExternalEffect> {
        match &self.mode {
            AppMode::Setup { previous_runtime } => {
                let event = screens::setup::show(
                    ui,
                    &mut self.setup,
                    &self.status,
                    previous_runtime.is_some(),
                );
                match event {
                    Some(screens::setup::SetupEvent::Save(config)) => {
                        return Some(effects::ExternalEffect::SaveSetup(config))
                    }
                    Some(screens::setup::SetupEvent::Test(config)) => {
                        return Some(effects::ExternalEffect::TestConnection(config))
                    }
                    Some(screens::setup::SetupEvent::Cancel) => {
                        let AppMode::Setup { previous_runtime } = std::mem::replace(
                            &mut self.mode,
                            AppMode::Setup {
                                previous_runtime: None,
                            },
                        ) else {
                            unreachable!()
                        };
                        self.cancel_setup(previous_runtime);
                    }
                    None => {}
                }
            }
            AppMode::Main(runtime) => {
                preferences::apply_theme_from_config(ui.ctx(), &runtime.config);
                preferences::apply_font_size_from_config(ui.ctx(), &runtime.config);
                self.stream.avatar_cache.poll(ui.ctx());
                let event = screens::stream::show(
                    ui,
                    &mut self.stream,
                    &runtime.config,
                    &runtime.library_counts,
                    &runtime.saved_queries,
                    &runtime.items,
                    &self.status_history,
                );
                return event.and_then(|event| self.handle_stream_event(event));
            }
        }
        None
    }

    fn handle_stream_event(
        &mut self,
        event: screens::stream::StreamEvent,
    ) -> Option<effects::ExternalEffect> {
        match event {
            screens::stream::StreamEvent::Select(selection) => self.select(selection),
            screens::stream::StreamEvent::SetFilter(filter) => self.set_filter(filter),
            screens::stream::StreamEvent::SetLocalFilter(filter) => self.set_local_filter(filter),
            screens::stream::StreamEvent::AddLocalFilterInputTerm(term) => {
                self.add_local_filter_input_term(&term)
            }
            screens::stream::StreamEvent::AddFilterStream {
                saved_query_id,
                name,
                filter_query,
                enabled,
            } => self.add_filter_stream(saved_query_id, &name, &filter_query, enabled),
            screens::stream::StreamEvent::AddQuery {
                name,
                query,
                source,
                enabled,
            } => self.add_query(&name, &query, source, enabled),
            screens::stream::StreamEvent::PreviewQuery { query, source } => {
                return Some(effects::ExternalEffect::PreviewQuery { query, source })
            }
            screens::stream::StreamEvent::UpdateFilterStream {
                id,
                name,
                filter_query,
                enabled,
            } => self.update_filter_stream(id, &name, &filter_query, enabled),
            screens::stream::StreamEvent::UpdateQuery {
                id,
                name,
                query,
                source,
                enabled,
            } => self.update_query(id, &name, &query, source, enabled),
            screens::stream::StreamEvent::DeleteFilterStream(id) => self.delete_filter_stream(id),
            screens::stream::StreamEvent::DeleteQuery(id) => self.delete_query(id),
            screens::stream::StreamEvent::MoveQueryUp(id) => self.move_query_up(id),
            screens::stream::StreamEvent::MoveQueryDown(id) => self.move_query_down(id),
            screens::stream::StreamEvent::MarkLibraryRead(library) => {
                self.mark_library_read(library)
            }
            screens::stream::StreamEvent::MarkFilterStreamRead(id) => {
                self.mark_filter_stream_read(id)
            }
            screens::stream::StreamEvent::MarkSavedQueryRead(id) => self.mark_saved_query_read(id),
            screens::stream::StreamEvent::ExportQueries(path) => {
                return Some(effects::ExternalEffect::ExportQueries(path))
            }
            screens::stream::StreamEvent::ImportQueries(path) => {
                return Some(effects::ExternalEffect::ImportQueries(path))
            }
            screens::stream::StreamEvent::RefreshNow => {
                return Some(effects::ExternalEffect::Refresh)
            }
            screens::stream::StreamEvent::ShowRemoteUpdates => self.reload_current_view(),
            screens::stream::StreamEvent::SetDefaultSort(sort) => {
                return Some(effects::ExternalEffect::SetDefaultSort(sort))
            }
            screens::stream::StreamEvent::SetPollingInterval(seconds) => {
                return Some(effects::ExternalEffect::SetPollingInterval(seconds));
            }
            screens::stream::StreamEvent::SetTheme(theme) => {
                return Some(effects::ExternalEffect::SetTheme(theme))
            }
            screens::stream::StreamEvent::SetFontSize(size) => {
                return Some(effects::ExternalEffect::SetFontSize(size))
            }
            screens::stream::StreamEvent::OpenSetup => self.open_setup_settings(),
            screens::stream::StreamEvent::ItemAction(screens::stream::ItemAction::Open {
                id,
                url,
            }) => {
                return Some(effects::ExternalEffect::OpenItem { id, url });
            }
            screens::stream::StreamEvent::ItemAction(action) => self.item_action(action),
        }
        None
    }
}

fn first_run_status(error: &config::ConfigError) -> String {
    match error {
        config::ConfigError::Read(err) if err.kind() == std::io::ErrorKind::NotFound => {
            "First-run setup required. No config.yml exists yet.".to_owned()
        }
        _ => format!("Setup required: {error}"),
    }
}

#[cfg(test)]
mod tests;
