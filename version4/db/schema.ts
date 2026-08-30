import { sql } from "drizzle-orm";
import {
  index,
  pgTable,
  text,
  timestamp,
  uniqueIndex,
  uuid,
} from "drizzle-orm/pg-core";

export const users = pgTable("users", {
  id: uuid().defaultRandom().primaryKey(),
  issuer: text().notNull(),
  subject: text().notNull(),
  email: text(),
  createdAt: timestamp("created_at", { withTimezone: true }).default(sql`now()`)
    .notNull(),
  updatedAt: timestamp("updated_at", { withTimezone: true }).default(sql`now()`)
    .notNull(),
}, (table) => [
  uniqueIndex("users_issuer_subject_idx").on(table.issuer, table.subject),
]);

export const sessions = pgTable("sessions", {
  id: uuid().defaultRandom().primaryKey(),
  tokenHash: text("token_hash").notNull(),
  userId: uuid("user_id").notNull().references(() => users.id, {
    onDelete: "cascade",
  }),
  accessToken: text("access_token").notNull(),
  refreshToken: text("refresh_token").notNull(),
  oidcSessionId: text("oidc_session_id"),
  refreshRetryAfter: timestamp("refresh_retry_after", { withTimezone: true }),
  expiresAt: timestamp("expires_at", { withTimezone: true }).notNull(),
  createdAt: timestamp("created_at", { withTimezone: true }).default(sql`now()`)
    .notNull(),
}, (table) => [
  uniqueIndex("sessions_token_hash_idx").on(table.tokenHash),
  index("sessions_user_idx").on(table.userId),
  index("sessions_oidc_session_idx").on(table.oidcSessionId),
  index("sessions_expires_idx").on(table.expiresAt),
]);
