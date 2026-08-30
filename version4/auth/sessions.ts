import type { MiddlewareHandler } from "hono";
import { deleteCookie, getCookie, setCookie } from "hono/cookie";
import { errors as joseErrors } from "jose";
import { config } from "../config.ts";
import {
  type AuthSession,
  deleteAuthSessionByTokenHash,
  getAuthSessionByTokenHash,
  insertAuthSession,
  updateAuthSessionAtomically,
  type UpdateHandle,
} from "../data/sessions.ts";
import {
  isInvalidGrant,
  refreshProviderTokens,
  verifyAccessToken,
} from "./oidc.ts";
import { TokenCipher } from "./token-cipher.ts";
import type {
  AccessClaims,
  AppEnv,
  OidcIdentity,
  ProviderTokens,
  User,
} from "./types.ts";

export const sessionCookie = "food_session";

const earlyRefreshWindowMs = 60 * 1000;
const refreshRetryCooldownMs = 10 * 1000;
const maxEarlyRefreshes = 5;

type AuthResult =
  | { kind: "ok"; user: User; expiresAt: Date }
  | { kind: "unauthorized" }
  | { kind: "unavailable" };

type AccessStatus =
  | { kind: "valid"; claims: AccessClaims }
  | { kind: "expired" }
  | { kind: "invalid"; error: unknown };

const tokenCipher = await TokenCipher.create(config.sessionEncryptionKey);
const ongoingEarlyRefreshes = new Map<string, Promise<unknown>>();

function randomToken(): string {
  const bytes = crypto.getRandomValues(new Uint8Array(32));
  return btoa(String.fromCharCode(...bytes))
    .replaceAll("+", "-")
    .replaceAll("/", "_")
    .replace(/=+$/, "");
}

async function hashToken(token: string): Promise<string> {
  const hash = await crypto.subtle.digest(
    "SHA-256",
    new TextEncoder().encode(token),
  );
  return Array.from(
    new Uint8Array(hash),
    (byte) => byte.toString(16).padStart(2, "0"),
  ).join("");
}

function matchesIdentity(claims: AccessClaims, session: AuthSession): boolean {
  return claims.issuer === session.issuer && claims.subject === session.subject;
}

function retryReady(retryAfter: Date | null): boolean {
  return !retryAfter || retryAfter.getTime() <= Date.now();
}

async function checkAccessToken(session: AuthSession): Promise<AccessStatus> {
  try {
    const accessToken = await tokenCipher.decrypt(
      session.accessToken,
      `${session.id}:access`,
    );
    const claims = await verifyAccessToken(accessToken);
    return matchesIdentity(claims, session) ? { kind: "valid", claims } : {
      kind: "invalid",
      error: new Error("OIDC access token identity does not match its session"),
    };
  } catch (error) {
    return error instanceof joseErrors.JWTExpired
      ? { kind: "expired" }
      : { kind: "invalid", error };
  }
}

async function refreshSession(
  tokenHash: string,
  options: {
    refreshBeforeExpiryMs: number;
    waitForUpdate: boolean;
  },
): Promise<AuthResult | null> {
  return await updateAuthSessionAtomically(
    tokenHash,
    { waitForUpdate: options.waitForUpdate },
    async (update: UpdateHandle): Promise<AuthResult> => {
      const { session } = update;

      const access = await checkAccessToken(session);

      if (access.kind === "invalid") {
        console.warn(
          "invalid stored OIDC access token; deleting session",
          access.error,
        );
        await update.deleteSession();
        return { kind: "unauthorized" };
      }

      const user = { id: session.userId, email: session.email };

      if (access.kind === "valid") {
        const expiresIn = access.claims.expiresAt.getTime() - Date.now();
        const refreshReady = retryReady(session.refreshRetryAfter);

        if (expiresIn > options.refreshBeforeExpiryMs || !refreshReady) {
          return { kind: "ok", user, expiresAt: session.expiresAt };
        }
      }

      if (!retryReady(session.refreshRetryAfter)) {
        return { kind: "unavailable" };
      }

      let refreshToken: string;
      try {
        refreshToken = await tokenCipher.decrypt(
          session.refreshToken,
          `${session.id}:refresh`,
        );
      } catch (error) {
        console.warn(
          "stored OIDC refresh token decryption failed; deleting session",
          error,
        );
        await update.deleteSession();
        return { kind: "unauthorized" };
      }

      console.info("refreshing OIDC session");

      let newTokens: ProviderTokens;
      try {
        newTokens = await refreshProviderTokens(refreshToken);
      } catch (error) {
        if (isInvalidGrant(error)) {
          console.info("OIDC refresh token rejected");
          await update.deleteSession();
          return { kind: "unauthorized" };
        }

        console.warn("OIDC session refresh failed", error);
        await update.setRefreshRetryAfter(
          new Date(Date.now() + refreshRetryCooldownMs),
        );
        return { kind: "unavailable" };
      }

      console.info("OIDC session refresh succeeded");

      if (!matchesIdentity(newTokens.accessClaims, session)) {
        await update.deleteSession();
        return { kind: "unauthorized" };
      }

      await update.saveTokens(
        await tokenCipher.encrypt(
          newTokens.accessToken,
          `${session.id}:access`,
        ),
        await tokenCipher.encrypt(
          newTokens.refreshToken,
          `${session.id}:refresh`,
        ),
        newTokens.refreshExpiresAt,
      );
      return {
        kind: "ok",
        user,
        expiresAt: newTokens.refreshExpiresAt,
      };
    },
  );
}

function startEarlyRefresh(tokenHash: string): void {
  if (
    ongoingEarlyRefreshes.has(tokenHash) ||
    ongoingEarlyRefreshes.size >= maxEarlyRefreshes
  ) {
    return;
  }

  const refresh = refreshSession(tokenHash, {
    refreshBeforeExpiryMs: earlyRefreshWindowMs,
    waitForUpdate: false,
  })
    .catch((error) => {
      console.warn("early OIDC token refresh failed", error);
    })
    .finally(() => {
      if (ongoingEarlyRefreshes.get(tokenHash) === refresh) {
        ongoingEarlyRefreshes.delete(tokenHash);
      }
    });

  ongoingEarlyRefreshes.set(tokenHash, refresh);
}

async function authenticate(tokenHash: string): Promise<AuthResult> {
  const session = await getAuthSessionByTokenHash(tokenHash);
  if (!session) {
    return { kind: "unauthorized" };
  }

  const access = await checkAccessToken(session);
  if (access.kind !== "valid") {
    const result = await refreshSession(tokenHash, {
      refreshBeforeExpiryMs: 0,
      waitForUpdate: true,
    });
    return result ?? { kind: "unauthorized" };
  }

  const expiresIn = access.claims.expiresAt.getTime() - Date.now();
  const expiresSoon = expiresIn <= earlyRefreshWindowMs;
  const refreshReady = retryReady(session.refreshRetryAfter);

  if (expiresSoon && refreshReady) {
    startEarlyRefresh(tokenHash);
  }

  return {
    kind: "ok",
    user: { id: session.userId, email: session.email },
    expiresAt: session.expiresAt,
  };
}

export async function createSession(
  identity: OidcIdentity,
  oidcSessionId: string,
  tokens: ProviderTokens,
): Promise<{ expiresAt: Date; token: string }> {
  const token = randomToken();
  const tokenHash = await hashToken(token);
  const id = crypto.randomUUID();
  const expiresAt = tokens.refreshExpiresAt;

  await insertAuthSession({
    id,
    tokenHash,
    accessToken: await tokenCipher.encrypt(tokens.accessToken, `${id}:access`),
    refreshToken: await tokenCipher.encrypt(
      tokens.refreshToken,
      `${id}:refresh`,
    ),
    oidcSessionId,
    expiresAt,
    identity,
  });

  return { expiresAt, token };
}

export async function deleteSession(token: string): Promise<void> {
  await deleteAuthSessionByTokenHash(await hashToken(token));
}

export const requireUser: MiddlewareHandler<AppEnv> = async (c, next) => {
  const requestUrl = new URL(c.req.url);
  const returnTo = `${requestUrl.pathname}${requestUrl.search}`;
  const signInUrl = `/sign-in?return_to=${encodeURIComponent(returnTo)}`;
  const token = getCookie(c, sessionCookie);
  if (!token) {
    return c.redirect(signInUrl);
  }

  const result = await authenticate(await hashToken(token));
  if (result.kind === "unavailable") {
    return c.text("Authentication service unavailable", 503);
  }
  if (result.kind !== "ok") {
    deleteCookie(c, sessionCookie, { path: "/" });
    return c.redirect(signInUrl);
  }

  setCookie(c, sessionCookie, token, {
    path: "/",
    httpOnly: true,
    secure: config.secureCookies,
    sameSite: "lax",
    expires: result.expiresAt,
  });
  c.set("user", result.user);
  await next();
};
