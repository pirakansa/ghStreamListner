use eframe::egui;

use super::{screens::stream::ItemAction, GhStreamApp};
use crate::models::{AppConfig, FontSize, SortOrder, StreamSource, Theme};

/// Work whose implementation depends on the application's external environment.
pub(super) enum ExternalEffect {
    SaveSetup(AppConfig),
    TestConnection(AppConfig),
    Refresh,
    OpenItem { id: i64, url: String },
    PreviewQuery { query: String, source: StreamSource },
    ExportQueries(String),
    ImportQueries(String),
    SetDefaultSort(SortOrder),
    SetPollingInterval(u32),
    SetTheme(Theme),
    SetFontSize(FontSize),
}

impl GhStreamApp {
    pub(super) fn execute_effect(&mut self, ctx: &egui::Context, effect: ExternalEffect) {
        match effect {
            ExternalEffect::SaveSetup(config) => self.save_setup_config(config),
            ExternalEffect::TestConnection(config) => {
                let message = match crate::github::test_connection(&config) {
                    Ok(()) => format!(
                        "Connection succeeded. REST: {} GraphQL: {}",
                        config.host.rest_api_base_url(),
                        config.host.graphql_url()
                    ),
                    Err(err) => format!("Configuration is valid, but connection failed: {err}"),
                };
                self.setup.set_validation_message(message);
            }
            ExternalEffect::Refresh => self.refresh_now(ctx.clone()),
            ExternalEffect::OpenItem { id, url } => self.item_action(ItemAction::Open { id, url }),
            ExternalEffect::PreviewQuery { query, source } => self.preview_query(&query, source),
            ExternalEffect::ExportQueries(path) => self.export_queries(&path),
            ExternalEffect::ImportQueries(path) => self.import_queries(&path),
            ExternalEffect::SetDefaultSort(sort) => self.update_default_sort(sort),
            ExternalEffect::SetPollingInterval(seconds) => {
                self.update_polling_interval(seconds);
                self.stream.polling_interval_draft = 0;
            }
            ExternalEffect::SetTheme(theme) => self.update_theme(ctx, theme),
            ExternalEffect::SetFontSize(size) => self.update_font_size(ctx, size),
        }
    }
}
