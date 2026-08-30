import { Hono } from "hono";
import { registerAuth } from "./auth/routes.ts";
import { requireUser } from "./auth/sessions.ts";
import type { AppEnv } from "./auth/types.ts";
import { Layout } from "./components/layout.tsx";

export const app = new Hono<AppEnv>();

registerAuth(app);

app.get("/sign-in", (c) => {
  const returnTo = c.req.query("return_to");
  const signInUrl = returnTo
    ? `/auth/sign-in?return_to=${encodeURIComponent(returnTo)}`
    : "/auth/sign-in";

  return c.html(
    <Layout title="Sign in">
      <h1>Sign in</h1>
      <p>
        <a href={signInUrl}>Sign in</a>
      </p>
    </Layout>,
  );
});

app.use("/", requireUser);
app.get("/", (c) =>
  c.html(
    <Layout user={c.get("user")}>
      <h1>Food</h1>
    </Layout>,
  ));
