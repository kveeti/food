import { sql } from "drizzle-orm";
import { index, pgTable, text, timestamp, uniqueIndex, uuid, integer } from "drizzle-orm/pg-core";

const primaryId = uuid()
  .default(sql`uuidv7()`)
  .primaryKey();

export const users = pgTable(
  "users",
  {
    id: primaryId,
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
    id: primaryId,
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

export const waterLog = pgTable(
  "water_log",
  {
    id: primaryId,
    userId: uuid("user_id")
      .notNull()
      .references(() => users.id, { onDelete: "cascade" }),
    amountMl: integer("amount_ml").notNull(),
    consumedAt: timestamp("consumed_at", { withTimezone: true }).notNull(),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
  }
)
