#![cfg(feature = "ui-demo")]

use egui_kittest::{kittest::Queryable as _, Harness};
use ghtl::catalog::CatalogApp;

fn demo() -> Harness<'static, CatalogApp> {
    Harness::new_ui_state(
        |ui, app: &mut CatalogApp| app.show(ui),
        CatalogApp::new().unwrap(),
    )
}

#[test]
fn demo_uses_full_ui_and_shared_query_and_transfer_forms() {
    let mut harness = demo();
    harness.get_by_label("Library");
    harness.get_by_label("Refresh");
    assert!(harness.query_by_label("Reset story").is_none());
    harness.get_by_label("Manage").click();
    harness.run();
    harness.get_by_label("Edit query");
    harness.get_by_label("Save changes").click();
    harness.run();
    harness.get_by_label("F+").click();
    harness.run();
    harness.get_by_label("New filter stream");
    harness.get_by_label("+").click();
    harness.run();
    harness.get_by_label("New query");
    harness.get_by_label("Import / export").click();
    harness.run();
    harness.get_by_label("YAML file");
    harness.get_by_label("Export").click();
    harness.run();
    harness.get_by_label("Import").click();
    harness.run();
    harness.get_by_label("Edit query");
    harness.get_by_label("Back").click();
    harness.run();
    harness.get_by_label("Refresh").click();
    harness.run();
    harness.get_by_label("Library");
}

#[test]
fn demo_host_settings_test_save_and_back_use_shared_form() {
    let mut harness = demo();
    harness.get_by_label("Preferences").click();
    harness.run();
    harness.get_by_label("Host settings").click();
    harness.run();
    harness.get_by_label("Personal access token");
    harness.get_by_label("Test").click();
    harness.run();
    harness.get_by_label("Demo connection succeeded (simulated; no network request).");
    harness.get_by_label("Save").click();
    harness.run();
    harness.get_by_label("Library");
    harness.get_by_label("Preferences").click();
    harness.run();
    harness.get_by_label("Host settings").click();
    harness.run();
    harness.get_by_label("Back").click();
    harness.run();
    harness.get_by_label("Library");
}

#[test]
fn typed_query_and_filter_are_saved_through_shared_forms() {
    use eframe::egui::accesskit::Role;
    let mut harness = demo();
    harness.get_by_label("Manage").click();
    harness.run();
    harness.get_by_label("+").click();
    harness.run();
    for (index, value) in [(0, "UI query"), (1, "repo:demo/catalog")] {
        harness
            .get_all_by_role(Role::TextInput)
            .nth(index)
            .unwrap()
            .click();
        harness.run();
        harness
            .get_all_by_role(Role::TextInput)
            .nth(index)
            .unwrap()
            .type_text(value);
        harness.run();
    }
    harness.get_by_label("Add").click();
    harness.run();
    harness.get_by_label("UI query").click();
    harness.run();
    harness.get_by_label("Edit query");
    assert_eq!(
        harness
            .get_all_by_role(Role::TextInput)
            .nth(1)
            .unwrap()
            .value()
            .as_deref(),
        Some("repo:demo/catalog")
    );
    harness.get_by_label("F+").click();
    harness.run();
    for (index, value) in [(0, "UI filter"), (1, "is:pr")] {
        harness
            .get_all_by_role(Role::TextInput)
            .nth(index)
            .unwrap()
            .click();
        harness.run();
        harness
            .get_all_by_role(Role::TextInput)
            .nth(index)
            .unwrap()
            .type_text(value);
        harness.run();
    }
    harness.get_by_label("Add").click();
    harness.run();
    harness.get_by_label("↳ UI filter").click();
    harness.run();
    harness.get_by_label("Edit filter stream");
    assert_eq!(
        harness
            .get_all_by_role(Role::TextInput)
            .nth(1)
            .unwrap()
            .value()
            .as_deref(),
        Some("is:pr")
    );
}
