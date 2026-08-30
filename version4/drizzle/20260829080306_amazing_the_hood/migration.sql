ALTER TABLE "sessions" ADD COLUMN "oidc_session_id" text;--> statement-breakpoint
CREATE INDEX "sessions_oidc_session_idx" ON "sessions" ("oidc_session_id");