# Validation Coverage

The current test suite covers:

- Configuration normalization and validation
- PAT redaction from error messages
- REST Search response parsing
- REST Search discovery ordering remains updated-descending
- GraphQL enrichment parsing and review status derivation
- GraphQL Discussion Search discovery and discussion item persistence
- ProjectV2 discovery, persistence, and unread change detection
- Refresh write-before-render behavior
- Refresh failure preserving existing stored items
- GraphQL enrichment failure preserving existing pull request metadata
- GraphQL recovery at the same item timestamp updating relations without marking
  read items unread, and identical subsequent enrichment reporting no change
- Delayed refresh persistence retaining each query's discovery start time as the
  delta cursor, with failed persistence preserving the previous cursor
- Query definition edits resetting sync metadata and matches atomically while
  preserving shared item state, and obsolete refresh results being discarded
- Shared item metadata save deduplication across overlapping saved queries
- GraphQL enrichment deduplication and bounded batch execution
- Successful GraphQL batch application when another batch fails
- Host initialization without storing the PAT
- Item state persistence across metadata upserts
- Archived unread badge behavior
- Saved query updates, source persistence, import/export, and filter stream transfer
- Saved query source persistence and discussion preview routing
- Stream toolbar sort ordering for selected saved query views
- Polling interval persistence and status history retention
- Local SQL-backed toolbar filter validation and matching
- UI state/event handling
- `egui_kittest` component interactions for toolbar, left pane, and item list
