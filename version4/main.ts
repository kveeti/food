import { sql } from "drizzle-orm";
import { migrate } from "drizzle-orm/node-postgres/migrator";
import { app } from "./app.tsx";
import { config } from "./config.ts";
import { db } from "./db/client.ts";

console.log("pinging db...");
await db.execute(sql`select 1`);

console.log("running migrations");
await migrate(db, { migrationsFolder: "./drizzle" });

Deno.serve(
  {
    port: config.port,
    onListen: (addr) => {
      console.log(`listening at ${addr.hostname}:${addr.port}`);
    },
  },
  app.fetch,
);
