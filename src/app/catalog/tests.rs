use super::*;
use crate::app::effects::ExternalEffect;
use crate::app::screens::stream::{ItemAction, StreamEvent};
use crate::models::{FontSize, StreamSource, Theme};

fn runtime(app: &CatalogApp) -> &Runtime {
    let AppMode::Main(runtime) = &app.app.mode else {
        panic!("expected main screen")
    };
    runtime
}

fn dispatch(app: &mut CatalogApp, event: StreamEvent) {
    if let Some(effect) = app.app.handle_stream_event(event) {
        app.execute_effect(effect).expect("demo effect");
    }
}

#[test]
fn samples_use_memory_storage_and_have_no_remote_images() {
    let app = CatalogApp::new().unwrap();
    let runtime = runtime(&app);
    let filename: String = runtime
        .storage
        .connection()
        .query_row("PRAGMA database_list", [], |row| row.get(2))
        .unwrap();
    assert!(filename.is_empty());
    assert_eq!(runtime.saved_queries.len(), 2);
    assert_eq!(runtime.saved_queries[0].filter_streams.len(), 1);
    let mut items = runtime.items.clone();
    items.extend(
        runtime
            .storage
            .list_items_for_library(
                runtime.host_id,
                LibraryView::Archived,
                None,
                None,
                runtime.config.ui.default_sort,
            )
            .unwrap(),
    );
    assert_eq!(items.len(), 7);
    for item in items {
        assert!(item.author_avatar_url.is_none());
        assert!(item
            .assignees
            .iter()
            .chain(&item.review_requests)
            .all(|person| person.avatar_url.is_none()));
        assert!(item
            .reviewers
            .iter()
            .all(|person| person.avatar_url.is_none()));
    }
    assert!(app.app.refresh_rx.is_none());
    assert!(app.app.last_poll_at.is_none());
}

#[test]
fn shared_actions_update_queries_filters_and_item_state() {
    let mut app = CatalogApp::new().unwrap();
    dispatch(
        &mut app,
        StreamEvent::AddQuery {
            name: "New sample".into(),
            query: "is:issue".into(),
            source: StreamSource::IssueOrPullRequest,
            enabled: true,
        },
    );
    let Selection::SavedQuery(query_id) = app.app.stream.selection else {
        panic!("selection")
    };
    assert!(runtime(&app).items.is_empty());
    dispatch(&mut app, StreamEvent::RefreshNow);
    assert!(!runtime(&app).items.is_empty());
    dispatch(
        &mut app,
        StreamEvent::AddFilterStream {
            saved_query_id: query_id,
            name: "PRs".into(),
            filter_query: "is:pr".into(),
            enabled: true,
        },
    );
    assert!(runtime(&app)
        .items
        .iter()
        .all(|item| item.item_type == crate::models::ItemType::PullRequest));
    let id = runtime(&app).items[0].id;
    dispatch(
        &mut app,
        StreamEvent::ItemAction(ItemAction::Bookmark(id, true)),
    );
    assert!(
        runtime(&app)
            .items
            .iter()
            .find(|item| item.id == id)
            .unwrap()
            .is_bookmarked
    );
    dispatch(
        &mut app,
        StreamEvent::SetLocalFilter(Some("not-a-supported-filter:value".into())),
    );
    assert!(app.app.stream.local_filter.is_none());
    dispatch(&mut app, StreamEvent::DeleteQuery(query_id));
    assert_eq!(
        app.app.stream.selection,
        Selection::Library(LibraryView::Inbox)
    );
    assert_eq!(runtime(&app).saved_queries.len(), 2);
}

#[test]
fn preferences_and_host_save_stay_in_memory_and_keep_sample_data() {
    let mut app = CatalogApp::new().unwrap();
    let original_ids = runtime(&app)
        .items
        .iter()
        .map(|item| item.id)
        .collect::<Vec<_>>();
    dispatch(&mut app, StreamEvent::SetTheme(Theme::Dark));
    dispatch(&mut app, StreamEvent::SetFontSize(FontSize::Large));
    dispatch(&mut app, StreamEvent::SetPollingInterval(120));
    assert_eq!(runtime(&app).config.ui.theme, Theme::Dark);
    assert_eq!(runtime(&app).config.ui.font_size, FontSize::Large);
    assert_eq!(runtime(&app).config.refresh.polling_interval_seconds, 120);
    let mut config = runtime(&app).config.clone();
    config.host.hostname = "changed.example.test".into();
    dispatch(&mut app, StreamEvent::OpenSetup);
    app.execute_effect(ExternalEffect::TestConnection(config.clone()))
        .unwrap();
    app.execute_effect(ExternalEffect::SaveSetup(config))
        .unwrap();
    assert_eq!(runtime(&app).config.host.hostname, "changed.example.test");
    assert_eq!(runtime(&app).config.ui.theme, Theme::Dark);
    assert_eq!(
        runtime(&app)
            .items
            .iter()
            .map(|item| item.id)
            .collect::<Vec<_>>(),
        original_ids
    );
    assert!(app.app.status.contains("in memory"));
}

#[test]
fn transfer_uses_snapshots_without_accessing_the_supplied_path() {
    let mut app = CatalogApp::new().unwrap();
    let path = std::env::temp_dir().join(format!("ghtl-demo-transfer-{}", std::process::id()));
    assert!(!path.exists());
    let path = path.to_string_lossy().into_owned();
    dispatch(&mut app, StreamEvent::ExportQueries(path.clone()));
    assert!(!std::path::Path::new(&path).exists());
    let id = runtime(&app).saved_queries[0].id;
    dispatch(&mut app, StreamEvent::DeleteQuery(id));
    assert_eq!(runtime(&app).saved_queries.len(), 1);
    dispatch(&mut app, StreamEvent::ImportQueries(path));
    assert_eq!(runtime(&app).saved_queries.len(), 2);
    assert!(runtime(&app).items.is_empty());
    dispatch(&mut app, StreamEvent::RefreshNow);
    assert!(!runtime(&app).items.is_empty());
    dispatch(&mut app, StreamEvent::ImportQueries(String::new()));
    assert!(app.app.status.contains("must not be empty"));
}

#[test]
fn external_stream_actions_are_returned_before_execution() {
    let mut app = CatalogApp::new().unwrap();
    for event in [
        StreamEvent::RefreshNow,
        StreamEvent::ExportQueries("unused".into()),
        StreamEvent::ImportQueries("unused".into()),
        StreamEvent::PreviewQuery {
            query: "is:pr".into(),
            source: StreamSource::IssueOrPullRequest,
        },
        StreamEvent::ItemAction(ItemAction::Open {
            id: 1,
            url: "https://example.test".into(),
        }),
        StreamEvent::SetTheme(Theme::Light),
    ] {
        let effect = app
            .app
            .handle_stream_event(event)
            .expect("must return an external effect");
        app.execute_effect(effect).unwrap();
    }
    assert!(app.app.refresh_rx.is_none());
}

#[test]
fn status_log_uses_shared_screen_and_back_navigation() {
    use egui_kittest::{kittest::Queryable as _, Harness};
    let mut app = CatalogApp::new().unwrap();
    app.app.stream.status_log.open = true;
    let mut harness = Harness::new_ui_state(|ui, app: &mut CatalogApp| app.show(ui), app);
    harness.get_by_label("Recent messages");
    harness.get_by_label("Back").click();
    harness.run();
    harness.get_by_label("Manage");
}
