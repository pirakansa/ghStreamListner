# UI Demo

Run the normal application UI with disposable sample data:

```sh
vorbere run run-catalog
```

A graphical desktop is required. The window title is **ghTimeLine Demo**.
`cargo run` and `vorbere run run` still start the production application.
The catalog task prepares the shared font asset and runs
`cargo run --bin ghtl-ui-catalog`. Use it for day-to-day UI development; use
`vorbere run run` when checking real host integration.

The demo opens directly into the usual stream view, with the same sidebar,
menus, toolbar, item cards, and forms. There is no separate component catalog.

- Use the library, saved queries, and nested filter streams to navigate sample
  items. Filtering, sorting, read state, bookmarks, and archives behave through
  the normal local application logic.
- Click **Manage** to edit saved queries. Use **+** for a new query and **F+** for
  a nested filter stream. Add, save, delete, and reorder definitions as usual.
- Open **Import / export** from the query manager. Export takes an in-memory
  snapshot; Import restores it, or the built-in definitions before the first
  export. The path field is present but no file is accessed. After importing,
  return to the stream and click **Refresh** to repopulate sample matches.
- Open **Preferences → Host settings** to inspect the real host input form.
  **Test** reports a simulated successful connection. **Save** keeps validated
  input for this session; **Back** discards edits. No credentials are needed:
  the initial form contains a fictitious token.
- Use **Preferences** to change theme, font size, and polling interval. The
  interval is editable, but the demo does not run automatic network refreshes.
- Click the status icon at the bottom to open the normal status log, including
  messages explaining simulated actions.

**Refresh** returns deterministic sample results according to the query's source;
GitHub query text is not sent to a server. Item opening marks the sample read,
and item/query previews record a simulated action without opening a browser.
Avatars use initials and do not fetch images.

All data lives in an in-memory SQLite database and session configuration. The
production database, configuration, and transfer files are untouched. Restarting
resets the demo. See the [design and implementation plan](../specifications/ui-catalog.md)
for the shared UI and external-effect boundaries.
