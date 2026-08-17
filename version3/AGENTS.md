# Food app

- Read `docs/` first. Keep domain rules there.
- This is plain CRUD. Use small, direct code. No layers or helpers with no clear use.
- Stack: Rust, Topcoat, Postgres, server HTML, htmx.
- Topcoat source: `~/code/external/topcoat/`.
- htmx source: `~/code/external/htmx/`.
- Local `hx-optimistic`: `public/hx-optimistic.js`. Keep it local.
- UI follows `../../dash/`: mobile first, Geist, quiet gray UI, bottom nav on phone, top nav on wide screens, system dark mode.
- Every link and form must work with JS off. htmx only makes it faster. Use JS only when HTML and htmx cannot do the job.
- Writes should feel instant. Use `hx-optimistic` when rollback is clear.
- Auth: OIDC code flow with PKCE. Store opaque session token hashes in Postgres. Use HttpOnly, SameSite=Lax cookies. Dev OIDC routes live in this server and must not run in prod.
- All user data must have an owner. Never read or write another user's rows.
- PWA uses Workbox 7.4.1. `/service-worker.js` caches only `/_topcoat/assets/` and `/_topcoat/fonts/`. Pages and writes stay network-only. Offline writes need a clear sync plan before code.
- No DB call in a loop. Prefer one clear SQL query.
- Tailwind input is `src/tailwind.css`. `build.rs` scans only `src/` to keep builds quick.
- Keep Cargo features narrow. Do not use `tokio/full` or Topcoat defaults when a small feature set works.
- Dependency debug info is off in dev/test profiles to cut build time.
- Batch edits, then run one check. Rust builds take time. Do not run `cargo check` while `topcoat dev` is building; that does the work twice.
