# Browser tests

Test how people use the app: find food, log a meal, change an entry, or save a
goal. Keep tests easy to follow. A test can be long when its steps belong
together, such as adding, editing, and deleting a meal. Split it when it starts
testing unrelated things, not just because it has many lines.

- Use the browser and check what the user sees and can do.
- Reload the page to check that a change was saved.
- Use the real backend. Fake slow or failed requests only to test those cases.
- Wait for results instead of sleeping for a set time.
- Check that the app works, not exact spacing, fonts, colors, CSS or DOM structure.
- Keep SQL and database checks in backend tests.

Each test must have its own backend and database. The setup here handles that,
loads sample foods from `backend/test-support/`, and cleans up afterward.
