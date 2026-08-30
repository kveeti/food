# Server components playground

A small Hono app for building server-rendered HTML components with htmx and Tailwind CSS 4. Each component must keep a useful plain HTML fallback.

## Run

```sh
pnpm install
pnpm dev
```

Open <http://localhost:3001>.

## Check

```sh
pnpm typecheck
pnpm test
```

The browser tests cover both JavaScript and no-JavaScript use.

## Combobox contract

`<server-combobox>` enhances existing server HTML. It does not fetch, filter, or choose values itself.

- Mark the search input with `data-combobox-input`.
- Mark the stable popover with `data-combobox-popup`.
- Render a list inside it with `data-combobox-list` and give the list an ID.
- Point the input's `aria-controls` at the list ID.
- Optionally mark a loading indicator with `data-combobox-spinner`.
- Render each result as a real link or button with `data-combobox-option`.
- Add a `data-combobox-status` element beside the list for result counts and empty states.
- Swap the popover's inner HTML when loading remote results.
- Render and style every part yourself. The component uses light DOM and ships no visual theme.

The component adds the combobox roles, active option state, busy state, keyboard controls, and a stable loading indicator. The indicator appears only when a request starts. While it is visible, each input extends its hide deadline by 300 ms so a debounced follow-up request does not make it flash. It marks upgraded parts with `data-enhanced`. It upgrades the popup to a native Popover anchored with CSS, while htmx only replaces the popup contents. Choosing an option invokes its real link or button and leaves the search query unchanged. The form and result actions still work when JavaScript is off.

Arrow key selection loops by default. Set `loop="false"` in server HTML, or set the element's `loop` property to `false`, to stop at the first and last options.
