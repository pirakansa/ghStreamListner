# UI Demo

## Baseline and scope

Revised implementation baseline, 2026-09-11: the user requested the complete
production UI populated with dummy data, including all input screens. This
supersedes the component-story catalog. The user authorized this revision and
implementation. The binary remains `ghtl-ui-catalog` for command compatibility.

The demo must use the production screen composition, navigation, forms, and
local action handling. There is no story sidebar or demo-only layout. Only the
native window title and status messages identify the demo. The main stream,
queries, nested filter streams, import/export form, host settings, preferences,
and status log must all be reachable through the usual controls.

## Shared boundary

`GhStreamApp` renders the shared screens and handles local actions. Operations
that touch external systems are returned as a typed effect to the caller. The
production application executes real effects; the separate demo application
executes simulated effects. New external effects must be handled exhaustively
by both callers. No component branches on a demo flag or Cargo feature.

Initialization belongs to `GhStreamApp`. Production startup supplies file paths;
the demo supplies configuration, seeded storage, and an initial status through
`GhStreamApp::from_storage`. Both use the same runtime and application-state
initializers. The demo does not construct runtime fields, file-path placeholders,
status history, or polling/refresh state itself.

After either import mechanism replaces definitions, both callers invoke
`GhStreamApp::finish_query_import`. This shared step reloads sidebar data, selects
the first imported query (or Inbox for an empty import), resets scrolling, opens
the query manager, and reloads the item view. File/snapshot handling and the
completion message remain with each caller.

The setup screen must emit a validated connection-test event rather than perform
network I/O while drawing. The production caller retains the existing connection
test behavior and result wording. The demo caller supplies a simulated result.

The demo initializes the existing `Storage::in_memory()` with fictitious data.
This reuses production query/filter/sort/count/CRUD rules without touching the
on-disk database. It never loads user config or credentials, schedules polling,
opens a browser, or reads/writes import/export files. Avatar URLs are absent,
including nested people, so rendering uses the existing initials placeholders.

## Demo behavior

- Start in the main stream with sample issues, pull requests, discussions,
  saved queries, and nested filter streams. Include read/unread, bookmarked,
  archived, draft, merged, closed, and long-content examples.
- Library/query selection, filters, sorting, item actions, query/filter-stream
  editing, deletion, ordering, and mark-read operations use shared local handlers
  against the in-memory database.
- Preferences update session config only. Host settings use the same validated
  form; Test reports simulated success, Save updates session config and returns
  to the stream, Back discards the draft. Keep sample data after host edits.
- Refresh deterministically populates enabled query matches with sample results
  appropriate to their source. It performs no remote search or automatic polling.
- Item opening marks the sample item read and reports a simulated browser action.
  Query preview reports a simulated action without launching a browser.
- Export keeps a snapshot of query definitions in memory; Import restores that
  snapshot, or built-in definitions if none has been exported. The path input is
  visible but no file is accessed. Reject empty paths. Import clears matches;
  Refresh repopulates them. Status messages explain the simulation.
- Restarting the demo resets all data and preferences. `cargo run` still starts
  the production binary; `cargo run --features ui-demo --bin ghtl-ui-catalog` starts the demo.
  `vorbere run run-catalog` prepares assets and launches the demo, while
  `vorbere run run` retains its production-app behavior.

## Build boundary

The `ui-demo` feature is opt-in (`default = []`). It gates the catalog module,
its library re-export, and the catalog integration test. The demo executable
requires this feature. Normal builds omit the demo code and skip its executable.
Shared UI, local operations, runtime initialization, and external-effect routing
remain feature-independent. Only the caller-owned-storage constructor is gated
with `any(test, feature = "ui-demo")`, since ordinary unit tests also use it.

`run-catalog`, `build-catalog`, `check-catalog`, and `test-catalog` enable the
feature explicitly. Normal build/check/test tasks retain default features.
`test-catalog` depends on the demo build and static checks and runs the complete
suite with the feature enabled. GitHub CI and the local `ci` task validate both
configurations so neither branch of the build boundary goes unchecked.

Feature-boundary validation on 2026-09-12: the default check, test, and build
tasks passed (150 tests); `test-catalog`, including its static-check and build
dependencies, passed with `ui-demo` enabled (159 tests). Requesting the demo
binary without its feature was rejected by Cargo as intended.

## Source layout

- `src/main.rs`: startup for the production executable.
- `src/bin/ghtl-ui-catalog.rs`: startup for the additional demo executable,
  declared in Cargo.toml with `required-features = ["ui-demo"]`. It installs fonts and opens
  the native window; it does not implement the demo's data or operations.
- `src/app/`: shared application screens, state, and local action handling.
- `src/app/catalog/`: demo initialization, fixtures, and simulated effects.

The executable imports the shared library; directory nesting does not describe
the call hierarchy. Keeping startup in `src/bin/` follows the repository's Cargo
layout policy. Keeping the demo implementation under `app/` allows it to reuse
the application's private state without exposing those details as a public API.

## Plan

1. Replace the catalog specification before changing code.
2. Extract a shared render/local-action entry point and typed external effects.
3. Move setup connection testing to caller-side effect handling.
4. Replace the story shell with an in-memory demo caller and sample seeding.
5. Test shared navigation and input forms, local CRUD/filtering, simulated
   effects, and absence of external file or image access.
6. Update the guide and links; run check, test, and build before and after work.

## Acceptance criteria

- The demo shows the production sidebar/menu/toolbar/list and contains no story
  controls. Host settings, query/filter forms, transfer form, and status log are
  reachable through shared navigation, including Back/Save transitions.
- Query/filter edits and item state changes update the same UI using the shared
  persistence rules on an in-memory database.
- Setup Test emits an event with validated input; production handles its real
  connection and demo handles its fake response.
- Every external effect has an explicit demo handler; edited hostnames, tokens,
  and paths cannot cause network access, file access, or browser launching.
- Existing production tests and `vorbere run check`, `vorbere run test`, and
  `vorbere run build` pass. UI tests cover the full demo's forms and navigation.

## Limits

Remote results and connection success are simulated, not verification of GitHub
queries or credentials. Image-backed avatars and screenshot baselines are out
of scope. Relative timestamps use the shared current-clock formatting.

## Implementation and validation

Implemented using `GhStreamApp::show` and the shared local event dispatcher in
`src/app/mod.rs`. `src/app/effects.rs` defines and executes production effects;
`src/app/catalog/` supplies in-memory fixtures and simulated effects. The setup
screen emits `SetupEvent::Test` after normal validation.

Validation on 2026-09-11:

- Before revision, check, test, and build passed.
- After revision, `vorbere run check`, `vorbere run test`, and
  `vorbere run build` passed; all 159 tests succeeded after the shared
  initialization and import-completion refactor.
- `tests/ui_catalog.rs` covers full-screen navigation, query/filter form text
  entry and saving, transfer navigation, and host Test/Save/Back.
- Catalog unit tests cover memory-only storage, avatar inputs, shared local
  mutations/filter validation, preference/host edits, transfer snapshots, effect
  routing, and the shared status-log screen.
- Setup unit tests cover validated Test events and invalid input. A production
  connection-effect test verifies success/failure against a local mock server.
- Startup tests cover persisted data, missing/invalid configuration, and database
  failure. Import-completion tests cover first-query/Inbox selection, scroll reset,
  query-manager navigation, and sidebar/item reloads for nonempty/empty imports.
- Testing caught an invalid sample host kind, corrected to GHES. Test fixtures
  were also adjusted for hostname validation (mock ports are tested at the
  transport boundary), normal post-import navigation, and distinct input nodes.
- Native startup ran without error output for eight seconds before an intentional
  timeout. Manual visual inspection and screenshot comparisons were not performed.
