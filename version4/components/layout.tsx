import type { PropsWithChildren } from "hono/jsx";
import type { User } from "../auth/types.ts";

type LayoutProps = PropsWithChildren<{
  title?: string;
  user?: User;
}>;

export function Layout({ children, title = "Food", user }: LayoutProps) {
  return (
    <html lang="en">
      <head>
        <meta charset="utf-8" />
        <meta name="viewport" content="width=device-width, initial-scale=1" />
        <title>{title}</title>
      </head>
      <body>
        {user && (
          <header>
            <a href="/">Food</a>
            <span>{user.email ?? "Signed in"}</span>
            <form method="post" action="/logout">
              <button type="submit">Log out</button>
            </form>
          </header>
        )}
        <main>{children}</main>
      </body>
    </html>
  );
}
