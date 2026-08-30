# Server components playground

- Use Hono server-rendered HTML, htmx, Tailwind CSS 4, and small web components.
- Every form and action must work with JavaScript off.
- Keep the server in charge of data, filtering, and returned HTML.
- Web components add only browser behavior such as keyboard control and ARIA state.
- Use light DOM. Users render and style each part. Do not hide server HTML in shadow roots.
- Keep components small and direct. Add an API only when a demo needs it.
- Add Playwright coverage for both JavaScript and no-JavaScript use.
