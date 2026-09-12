use super::*;

impl CatalogApp {
    pub(super) fn export_queries(&mut self, path: &str) {
        if path.trim().is_empty() {
            self.status("Saved query export path must not be empty.");
            return;
        }
        if let AppMode::Main(runtime) = &self.app.mode {
            self.exported_queries = Some(
                runtime
                    .saved_queries
                    .iter()
                    .map(|query| ImportedSavedQuery {
                        name: query.name.clone(),
                        query: query.query.clone(),
                        source: query.source,
                        enabled: query.enabled,
                        position: query.position,
                        filter_streams: query
                            .filter_streams
                            .iter()
                            .map(|filter| ImportedFilterStream {
                                name: filter.name.clone(),
                                filter_query: filter.filter_query.clone(),
                                enabled: filter.enabled,
                                position: filter.position,
                            })
                            .collect(),
                    })
                    .collect(),
            );
        }
        self.status("Demo export saved a snapshot in memory; no file was written.");
    }

    pub(super) fn import_queries(&mut self, path: &str) -> Result<()> {
        if path.trim().is_empty() {
            self.status("Saved query import path must not be empty.");
            return Ok(());
        }
        let definitions = self
            .exported_queries
            .clone()
            .unwrap_or_else(fixtures::definitions);
        let mut inserted_ids = Vec::new();
        if let AppMode::Main(runtime) = &self.app.mode {
            inserted_ids = runtime
                .storage
                .replace_saved_queries(runtime.host_id, &definitions)?;
        }
        self.app.finish_query_import(&inserted_ids);
        self.status("Demo import restored sample definitions in memory; no file was read. Refresh to rebuild matches.");
        Ok(())
    }
}
