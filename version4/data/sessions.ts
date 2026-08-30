import { and, eq, gt, inArray, lte } from "drizzle-orm";
import { db } from "../db/client.ts";
import { sessions, users } from "../db/schema.ts";

export type AuthSession = {
  id: string;
  accessToken: string;
  refreshToken: string;
  refreshRetryAfter: Date | null;
  expiresAt: Date;
  userId: string;
  email: string | null;
  issuer: string;
  subject: string;
};

export type NewAuthSession = {
  id: string;
  tokenHash: string;
  accessToken: string;
  refreshToken: string;
  oidcSessionId: string;
  expiresAt: Date;
  identity: {
    issuer: string;
    subject: string;
    email: string | null;
  };
};

export type UpdateHandle = {
  session: AuthSession;
  deleteSession(): Promise<void>;
  saveTokens(
    accessToken: string,
    refreshToken: string,
    expiresAt: Date,
  ): Promise<void>;
  setRefreshRetryAfter(retryAfter: Date): Promise<void>;
};

const sessionSelection = {
  id: sessions.id,
  accessToken: sessions.accessToken,
  refreshToken: sessions.refreshToken,
  refreshRetryAfter: sessions.refreshRetryAfter,
  expiresAt: sessions.expiresAt,
  userId: users.id,
  email: users.email,
  issuer: users.issuer,
  subject: users.subject,
};

export async function insertAuthSession(
  input: NewAuthSession,
): Promise<void> {
  await db.transaction(async (tx) => {
    const [user] = await tx
      .insert(users)
      .values({
        issuer: input.identity.issuer,
        subject: input.identity.subject,
        email: input.identity.email,
      })
      .onConflictDoUpdate({
        target: [users.issuer, users.subject],
        set: { email: input.identity.email, updatedAt: new Date() },
      })
      .returning({ id: users.id });

    if (!user) {
      throw new Error("OIDC user upsert returned no user");
    }

    await tx.insert(sessions).values({
      id: input.id,
      tokenHash: input.tokenHash,
      userId: user.id,
      accessToken: input.accessToken,
      refreshToken: input.refreshToken,
      oidcSessionId: input.oidcSessionId,
      expiresAt: input.expiresAt,
    });
  });
}

export async function getAuthSessionByTokenHash(
  tokenHash: string,
): Promise<AuthSession | null> {
  const [session] = await db
    .select(sessionSelection)
    .from(sessions)
    .innerJoin(users, eq(sessions.userId, users.id))
    .where(
      and(
        eq(sessions.tokenHash, tokenHash),
        gt(sessions.expiresAt, new Date()),
      ),
    )
    .limit(1);

  return session ?? null;
}

export async function updateAuthSessionAtomically<T>(
  tokenHash: string,
  options: { waitForUpdate: boolean },
  update: (handle: UpdateHandle) => Promise<T>,
): Promise<T | null> {
  return await db.transaction(async (tx) => {
    const query = tx
      .select(sessionSelection)
      .from(sessions)
      .innerJoin(users, eq(sessions.userId, users.id))
      .where(
        and(
          eq(sessions.tokenHash, tokenHash),
          gt(sessions.expiresAt, new Date()),
        ),
      )
      .limit(1);

    const rows = options.waitForUpdate
      ? await query.for("update", { of: sessions })
      : await query.for("update", { of: sessions, skipLocked: true });
    const session = rows[0];

    if (!session) {
      return null;
    }

    return await update({
      session,

      async deleteSession() {
        await tx.delete(sessions).where(eq(sessions.id, session.id));
      },

      async saveTokens(accessToken, refreshToken, expiresAt) {
        await tx
          .update(sessions)
          .set({
            accessToken,
            refreshToken,
            refreshRetryAfter: null,
            expiresAt,
          })
          .where(eq(sessions.id, session.id));
      },

      async setRefreshRetryAfter(refreshRetryAfter) {
        await tx
          .update(sessions)
          .set({ refreshRetryAfter })
          .where(eq(sessions.id, session.id));
      },
    });
  });
}

export async function startSessionCleanup(): Promise<void> {
  let running = false;

  const clean = async () => {
    if (running) {
      return;
    }

    running = true;
    try {
      const result = await db
        .delete(sessions)
        .where(lte(sessions.expiresAt, new Date()));

      if (result.rowCount) {
        console.info(`deleted ${result.rowCount} expired sessions`);
      }
    } catch (error) {
      console.warn("expired session cleanup failed", error);
    } finally {
      running = false;
    }
  };

  await clean();
  setInterval(clean, 24 * 60 * 60 * 1000);
}

export async function deleteAuthSessionsByOidcSession(
  issuer: string,
  oidcSessionId: string,
): Promise<void> {
  await db
    .delete(sessions)
    .where(
      and(
        eq(sessions.oidcSessionId, oidcSessionId),
        inArray(
          sessions.userId,
          db
            .select({ id: users.id })
            .from(users)
            .where(eq(users.issuer, issuer)),
        ),
      ),
    );
}

export async function deleteAuthSessionByTokenHash(
  tokenHash: string,
): Promise<void> {
  await db
    .delete(sessions)
    .where(eq(sessions.tokenHash, tokenHash));
}
