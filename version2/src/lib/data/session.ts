import { createHash, randomBytes } from "node:crypto";
import { and, eq, gt } from "drizzle-orm";
import { db } from "./db";
import { createUserId } from "./id";
import { sessions, users, type User } from "./schema";

const sessionSeconds = 7 * 24 * 60 * 60;

export async function createUserAndSession(identity: {
  issuer: string;
  subject: string;
  email: string;
}) {
  const token = randomBytes(32).toString("base64url");
  const now = new Date();

  return db.transaction(async (tx) => {
    await tx
      .insert(users)
      .values({
        publicId: createUserId(),
        ...identity,
        createdAt: now,
      })
      .onConflictDoNothing({ target: [users.issuer, users.subject] });

    const [user] = await tx
      .select()
      .from(users)
      .where(and(eq(users.issuer, identity.issuer), eq(users.subject, identity.subject)))
      .limit(1);
    if (!user) throw new Error("OIDC user was not saved");

    await tx.insert(sessions).values({
      userId: user.id,
      tokenHash: hashToken(token),
      createdAt: now,
      expiresAt: new Date(now.getTime() + sessionSeconds * 1000),
    });

    return { token, maxAge: sessionSeconds };
  });
}

export async function userForToken(token: string | undefined): Promise<User | undefined> {
  if (!token) return;
  const [result] = await db
    .select({ user: users })
    .from(sessions)
    .innerJoin(users, eq(sessions.userId, users.id))
    .where(and(eq(sessions.tokenHash, hashToken(token)), gt(sessions.expiresAt, new Date())))
    .limit(1);
  return result?.user;
}

export async function deleteSession(token: string | undefined) {
  if (!token) return;
  await db.delete(sessions).where(eq(sessions.tokenHash, hashToken(token)));
}

function hashToken(token: string) {
  return createHash("sha256").update(token).digest("base64url");
}
