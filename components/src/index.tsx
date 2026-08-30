import { readFileSync } from "node:fs";
import { createRequire } from "node:module";

import { serve } from "@hono/node-server";
import { serveStatic } from "@hono/node-server/serve-static";
import { Hono } from "hono";
import type { FC } from "hono/jsx";

import { findFruit, searchFruits, type Fruit } from "./data.js";

const require = createRequire(import.meta.url);
const htmxSource = readFileSync(require.resolve("htmx.org"), "utf8");
const app = new Hono();

const FruitResults: FC<{ query: string; results: Fruit[] }> = ({ query, results }) => {
  const options = query.length >= 2 ? results : [];
  const status = query.length === 0
    ? ""
    : query.length < 2
      ? "Type at least two characters."
      : options.length === 0
        ? `No fruit matched “${query}”.`
        : `${options.length} ${options.length === 1 ? "result" : "results"} available.`;

  return (
    <>
      <ul id="fruit-listbox" class="m-0 list-none p-0" data-combobox-list>
        {options.map((fruit) => (
          <li>
            <a
              class="block rounded-lg px-3 py-2 text-inherit no-underline outline-none hover:bg-stone-100 focus-visible:bg-stone-100 aria-selected:bg-stone-100 dark:hover:bg-neutral-700 dark:focus-visible:bg-neutral-700 dark:aria-selected:bg-neutral-700"
              data-combobox-option
              href={`/?q=${encodeURIComponent(query)}&fruit=${fruit.id}#selection`}
              hx-get={`/fruits/${fruit.id}`}
              hx-target="#selection"
              hx-swap="outerHTML"
            >
              <span class="block text-sm font-semibold">{fruit.name}</span>
              <span class="mt-0.5 block truncate text-xs text-stone-500 dark:text-stone-400">{fruit.description}</span>
            </a>
          </li>
        ))}
      </ul>
      <p
        class={options.length > 0 || query.length === 0
          ? "sr-only"
          : "px-3 py-2 text-sm text-stone-500 dark:text-stone-400"}
        data-combobox-status
        role="status"
        aria-live="polite"
        aria-atomic="true"
      >
        {status}
      </p>
    </>
  );
};

const Selection: FC<{ fruit?: Fruit }> = ({ fruit }) => (
  <section
    id="selection"
    class="mt-5 min-h-29 rounded-xl border border-stone-200 p-4 dark:border-neutral-700"
    aria-live="polite"
  >
    {fruit ? (
      <>
        <p class="mb-1 text-xs font-semibold uppercase tracking-wider text-stone-500 dark:text-stone-400">Selected fruit</p>
        <h3 class="text-lg font-semibold">{fruit.name}</h3>
        <p class="mt-2 text-sm leading-6 text-stone-500 dark:text-stone-400">{fruit.description}</p>
      </>
    ) : (
      <>
        <p class="mb-1 text-xs font-semibold uppercase tracking-wider text-stone-500 dark:text-stone-400">Try it</p>
        <h3 class="text-lg font-semibold">No fruit selected</h3>
        <p class="mt-2 text-sm leading-6 text-stone-500 dark:text-stone-400">
          Search, then use <kbd class="rounded border border-b-2 border-stone-200 bg-stone-100 px-1.5 py-0.5 text-xs dark:border-neutral-600 dark:bg-neutral-700">↑</kbd>{" "}
          <kbd class="rounded border border-b-2 border-stone-200 bg-stone-100 px-1.5 py-0.5 text-xs dark:border-neutral-600 dark:bg-neutral-700">↓</kbd>{" "}
          and <kbd class="rounded border border-b-2 border-stone-200 bg-stone-100 px-1.5 py-0.5 text-xs dark:border-neutral-600 dark:bg-neutral-700">Enter</kbd>, or choose a result with a pointer.
        </p>
      </>
    )}
  </section>
);

const Page: FC<{ query: string; selected?: Fruit }> = ({ query, selected }) => {
  const results = searchFruits(query);

  return (
    <html lang="en" class="bg-stone-50 text-stone-950 antialiased dark:bg-neutral-900 dark:text-stone-100">
      <head>
        <meta charset="utf-8" />
        <meta name="viewport" content="width=device-width, initial-scale=1" />
        <meta name="color-scheme" content="light dark" />
        <title>Server components</title>
        <link rel="stylesheet" href="/assets/styles.css" />
        <script defer src="/assets/htmx.min.js"></script>
        <script defer src="/assets/combobox.js"></script>
      </head>
      <body class="min-h-screen">
        <main class="mx-auto w-full max-w-152 px-2 py-8 sm:px-4 sm:py-16">
          <header class="mb-10">
            <p class="mb-2 text-xs font-semibold uppercase tracking-widest text-stone-500 dark:text-stone-400">Server components</p>
            <h1 class="text-4xl font-semibold tracking-[-0.045em] sm:text-5xl">Combobox</h1>
            <p class="mt-4 max-w-lg leading-6 text-stone-600 dark:text-stone-400">
              Server-rendered results, htmx updates, and a small web component for keyboard use. The
              same form and links work without JavaScript.
            </p>
          </header>

          <section class="rounded-2xl border border-stone-200 bg-white p-4 shadow-xl shadow-stone-950/5 dark:border-neutral-700 dark:bg-neutral-800 dark:shadow-black/20 sm:p-5" aria-labelledby="fruit-heading">
            <h2 id="fruit-heading" class="text-base font-semibold">Fruit picker</h2>
            <p class="mb-4 mt-1 text-sm leading-5 text-stone-500 dark:text-stone-400">Search by name. Try “ap”, “berry”, or “stone fruit”.</p>

            <server-combobox class="relative block">
              <form
                class="m-0"
                method="get"
                action="/"
                hx-get="/fruits"
                hx-target="#fruit-results"
                hx-swap="innerHTML"
                hx-sync="this:replace"
              >
                <label class="mb-1.5 block text-xs font-semibold" for="fruit-query">
                  Find a fruit
                </label>
                <div class="flex gap-2">
                  <div class="relative min-w-0 flex-1">
                    <input
                      id="fruit-query"
                      class="w-full rounded-xl border border-stone-300 bg-stone-100 px-3 py-2.5 pr-11 text-base text-stone-950 outline-none placeholder:text-stone-400 hover:border-stone-400 focus:border-emerald-700 focus:ring-2 focus:ring-emerald-700/20 dark:border-neutral-600 dark:bg-neutral-700 dark:text-stone-100 dark:placeholder:text-stone-500 dark:hover:border-neutral-500 dark:focus:border-emerald-400 dark:focus:ring-emerald-400/20"
                      data-combobox-input
                      name="q"
                      type="search"
                      value={query}
                      autocomplete="off"
                      spellcheck={false}
                      placeholder="Start typing…"
                      aria-controls="fruit-listbox"
                      hx-get="/fruits"
                      hx-trigger="input changed delay:120ms, search"
                      hx-include="closest form"
                      hx-target="#fruit-results"
                      hx-swap="innerHTML"
                      hx-sync="closest form:replace"
                    />
                    <span
                      class="pointer-events-none absolute inset-y-0 right-2 flex w-7 scale-90 items-center justify-center opacity-0 blur-[4px] transition-[opacity,scale,filter] duration-200 ease-out data-loading:scale-100 data-loading:opacity-100 data-loading:blur-none motion-reduce:transition-none"
                      data-combobox-spinner
                      aria-hidden="true"
                    >
                      <span class="size-4 animate-spin rounded-full border-2 border-stone-300 border-t-stone-900 motion-reduce:animate-none dark:border-neutral-500 dark:border-t-stone-100"></span>
                    </span>
                  </div>
                  <button id="fruit-search-button" class="min-w-22 rounded-xl border border-stone-200 px-3 py-2 text-sm font-semibold hover:bg-stone-100 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-stone-700 dark:border-neutral-600 dark:hover:bg-neutral-700 dark:focus-visible:outline-stone-300" type="submit">
                    Search
                  </button>
                </div>
              </form>

              <div
                id="fruit-results"
                class="mt-0.5 max-h-76 overflow-y-auto data-enhanced:mt-1.5 data-enhanced:rounded-xl data-enhanced:border data-enhanced:border-stone-200 data-enhanced:bg-white data-enhanced:p-1 data-enhanced:text-stone-950 data-enhanced:shadow-2xl data-enhanced:shadow-stone-950/15 dark:data-enhanced:border-neutral-600 dark:data-enhanced:bg-neutral-800 dark:data-enhanced:text-stone-100 dark:data-enhanced:shadow-black/40"
                data-combobox-popup
                hidden={query.length === 0}
              >
                <FruitResults query={query} results={results} />
              </div>
            </server-combobox>

            <Selection fruit={selected} />
          </section>
        </main>
      </body>
    </html>
  );
};

app.get("/assets/htmx.min.js", (c) =>
  c.body(htmxSource, 200, { "content-type": "text/javascript; charset=utf-8" }),
);
app.use("/assets/*", serveStatic({ root: "./public" }));

app.get("/", (c) => {
  const query = (c.req.query("q") ?? "").trim();
  const selected = findFruit(c.req.query("fruit"));
  return c.html(<Page query={query} selected={selected} />);
});

app.get("/fruits", (c) => {
  const query = (c.req.query("q") ?? "").trim();
  return c.html(<FruitResults query={query} results={searchFruits(query)} />);
});

app.get("/fruits/:id", (c) => {
  const fruit = findFruit(c.req.param("id"));
  if (!fruit) return c.notFound();
  return c.html(<Selection fruit={fruit} />);
});

const port = Number(process.env.PORT ?? 3001);
serve({ fetch: app.fetch, port }, (info) => {
  console.log(`Server components: http://localhost:${info.port}`);
});
