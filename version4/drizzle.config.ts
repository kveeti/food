import { defineConfig } from "drizzle-kit";
import { config } from "./config.ts";

export default defineConfig({
  dialect: "postgresql",
  schema: "./db/schema.ts",
  out: "./drizzle",
  dbCredentials: {
    url: config.databaseUrl,
  },
});
