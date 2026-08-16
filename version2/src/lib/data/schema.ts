import { sql } from "drizzle-orm";
import { index, pgTable, text, timestamp, uniqueIndex, uuid } from "drizzle-orm/pg-core";

export const users = pgTable(
  "users",
  {
    id: uuid()
      .default(sql`uuidv7()`)
      .primaryKey(),
    publicId: text("public_id").notNull().unique(),
    issuer: text().notNull(),
    subject: text().notNull(),
    email: text().notNull(),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull(),
  },
  (table) => [uniqueIndex("users_identity").on(table.issuer, table.subject)],
);

export const sessions = pgTable(
  "sessions",
  {
    id: uuid()
      .default(sql`uuidv7()`)
      .primaryKey(),
    userId: uuid("user_id")
      .notNull()
      .references(() => users.id, { onDelete: "cascade" }),
    tokenHash: text("token_hash").notNull().unique(),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull(),
    expiresAt: timestamp("expires_at", { withTimezone: true }).notNull(),
  },
  (table) => [index("sessions_user_id").on(table.userId)],
);

export type User = typeof users.$inferSelect;
