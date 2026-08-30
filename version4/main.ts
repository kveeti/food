import { sql } from "drizzle-orm";
import { migrate } from "drizzle-orm/node-postgres/migrator";
import { app } from "./app.tsx";
import { initializeOidc } from "./auth/oidc.ts";
import { config } from "./config.ts";
import { startSessionCleanup } from "./data/sessions.ts";
import { db } from "./db/client.ts";

console.log("pinging db...");
await db.execute(sql`select 1`);

console.log("running migrations");
await migrate(db, { migrationsFolder: "./drizzle" });

console.log("starting session cleanup in the bg...");
await startSessionCleanup();

console.log("discovering OIDC issuer config...");
await initializeOidc();

Deno.serve(
  {
    port: config.port,
    onListen: (addr) => {
      console.log(`listening at ${addr.hostname}:${addr.port}`);
    },
  },
  app.fetch,
);
