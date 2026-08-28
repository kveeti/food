import { Hono } from "hono";

export const app = new Hono();

app.get("/", (c) =>
  c.html(
    <html lang="en">
      <head>
        <meta charset="utf-8" />
        <meta name="viewport" content="width=device-width, initial-scale=1" />
        <title>Food</title>
      </head>
      <body>
        <main>
          <h1>Food</h1>
        </main>
      </body>
    </html>,
  ));
