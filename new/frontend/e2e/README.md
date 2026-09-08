# End-to-end tests

These tests exercise user journeys through the running React app, Rust API,
authentication flow, and PostgreSQL database. Use accessible browser controls
and assert user-visible outcomes. Reload or return to a day to verify that a
change persisted.

Do not assert CSS classes, computed styles, layout dimensions, animation details,
or DOM node identity. Test behavior, not how the UI implements it.

E2E tests must not contain SQL, inspect database tables, import backend data
modules, or depend on how the backend stores a feature. Public HTTP requests
may coordinate requests or set up a scenario when there is an appropriate
public API, but the journey under test should use the UI. Database constraints,
snapshot internals, and data-access rules belong in backend integration tests.

The test harness owns process and database isolation. Each test gets a fresh
database and a backend; workers have independent frontend and dev sign-in
servers. Database creation and removal are infrastructure, not test assertions.
The harness invokes the backend-owned `seed-test-catalog` command for a small
known catalog. Its schema knowledge and SQL stay under `backend/test-support`.
Tests may rely on the fixture foods' visible names and nutrition, not their
database IDs. Do not move SQL into a frontend helper to hide it from a test.

Use real responses for successful flows. Network interception is appropriate
for controlled latency or a specific failure, such as holding the detail
response to observe a skeleton or rejecting one save to exercise rollback.
Release delayed requests explicitly; avoid fixed sleeps and mocked happy paths.
Verify retry and persistence against the real backend afterward.

Run `pnpm run e2e` from `frontend` in the Nix development shell, or `make e2e`
from `new`. The harness builds the backend with the `test-support` feature;
the seed command is excluded from normal builds and only accepts disposable
E2E databases. Run one file with `pnpm run e2e food.test.ts`.
