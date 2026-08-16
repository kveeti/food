import { resolve } from "node:path";
import { drizzle } from "drizzle-orm/node-postgres";
import { migrate } from "drizzle-orm/node-postgres/migrator";
import { Pool } from "pg";
import { config } from "../config";
import * as schema from "./schema";

function openDatabase() {
  const pool = new Pool({ connectionString: config.databaseUrl });
  const db = drizzle(pool, { schema });
  return {
    db,
    ready: migrate(db, { migrationsFolder: resolve(config.migrationsPath) }),
  };
}

type DatabaseState = ReturnType<typeof openDatabase>;
const state = globalThis as typeof globalThis & { __foodDb?: DatabaseState };
const databaseState = (state.__foodDb ??= openDatabase());

console.log("connecting to db...");
await databaseState.ready;
console.log("db connected");
export const db = databaseState.db;
