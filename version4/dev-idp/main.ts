import { Hono } from "hono";
import { html } from "hono/html";
import {
  calculateJwkThumbprint,
  exportJWK,
  generateKeyPair,
  SignJWT,
} from "jose";

function requireEnv(name: string): string {
  const value = Deno.env.get(name);
  if (!value) {
    throw new Error(`${name} must be set`);
  }
  return value;
}

function parsePort(): number {
  const port = Number(Deno.env.get("PORT") ?? "8001");
  if (!Number.isInteger(port) || port < 1 || port > 65_535) {
    throw new Error("PORT must be a number between 1 and 65535");
  }
  return port;
}

function accessTokenLifetime(): number {
  const seconds = Number(Deno.env.get("IDP_ACCESS_TOKEN_LIFETIME") ?? "300");
  if (!Number.isInteger(seconds) || seconds < 1) {
    throw new Error("IDP_ACCESS_TOKEN_LIFETIME must be a positive number");
  }
  return seconds;
}

function randomToken(): string {
  const bytes = crypto.getRandomValues(new Uint8Array(32));
  return btoa(String.fromCharCode(...bytes))
    .replaceAll("+", "-")
    .replaceAll("/", "_")
    .replace(/=+$/, "");
}

async function pkceChallenge(verifier: string): Promise<string> {
  const digest = await crypto.subtle.digest(
    "SHA-256",
    new TextEncoder().encode(verifier),
  );
  return btoa(String.fromCharCode(...new Uint8Array(digest)))
    .replaceAll("+", "-")
    .replaceAll("/", "_")
    .replace(/=+$/, "");
}

function formValue(form: Record<string, string | File>, name: string): string {
  const value = form[name];
  return typeof value === "string" ? value : "";
}

const port = parsePort();
const accessTokenLifetimeSeconds = accessTokenLifetime();
const refreshDelayMs = Number(Deno.env.get("IDP_REFRESH_DELAY") ?? "0");
if (!Number.isInteger(refreshDelayMs) || refreshDelayMs < 0) {
  throw new Error("IDP_REFRESH_DELAY must be a non-negative number");
}
const appUrl = requireEnv("APP_URL").replace(/\/$/, "");
const issuer = requireEnv("OIDC_ISSUER").replace(/\/$/, "");
const clientId = requireEnv("OIDC_CLIENT_ID");
const clientSecret = requireEnv("OIDC_CLIENT_SECRET");
const redirectUri = `${appUrl}/auth/callback`;
const backchannelLogoutUri = `${appUrl}/auth/backchannel-logout`;

const { privateKey, publicKey } = await generateKeyPair("RS256", {
  extractable: true,
});
const publicJwk = await exportJWK(publicKey);
const keyId = await calculateJwkThumbprint(publicJwk);
Object.assign(publicJwk, { alg: "RS256", kid: keyId, use: "sig" });

type PendingCode = {
  subject: string;
  email: string;
  sessionId: string;
  nonce: string;
  codeChallenge: string;
  redirectUri: string;
  createdAt: number;
};

type PendingRefresh = {
  subject: string;
  email: string;
  sessionId: string;
  expiresAt: number;
};

const codes = new Map<string, PendingCode>();
const refreshTokens = new Map<string, PendingRefresh>();
const oidcSessions = new Map<string, { subject: string; email: string }>();
const users = {
  alice: "alice@dev.local",
  bob: "bob@dev.local",
} as const;

const app = new Hono();

app.get("/.well-known/openid-configuration", (c) =>
  c.json({
    issuer,
    authorization_endpoint: `${issuer}/authorize`,
    token_endpoint: `${issuer}/token`,
    jwks_uri: `${issuer}/jwks.json`,
    response_types_supported: ["code"],
    subject_types_supported: ["public"],
    id_token_signing_alg_values_supported: ["RS256"],
    scopes_supported: ["openid", "email", "offline_access"],
    grant_types_supported: ["authorization_code", "refresh_token"],
    token_endpoint_auth_methods_supported: ["client_secret_post"],
    code_challenge_methods_supported: ["S256"],
  }));

app.get("/jwks.json", (c) => c.json({ keys: [publicJwk] }));

app.get("/authorize", (c) => {
  const query = c.req.query();
  const scopes = query.scope?.split(" ") ?? [];
  if (
    query.client_id !== clientId || query.redirect_uri !== redirectUri ||
    query.response_type !== "code" || !scopes.includes("openid") ||
    query.code_challenge_method !== "S256" || !query.code_challenge ||
    !query.state || !query.nonce
  ) {
    return c.text("Invalid OIDC authorization request", 400);
  }

  const subject = query.sub;
  if (subject) {
    const email = users[subject as keyof typeof users];
    if (!email) {
      return c.text("Unknown dev user", 400);
    }

    for (const [code, pending] of codes) {
      if (Date.now() - pending.createdAt >= 10 * 60 * 1000) {
        codes.delete(code);
      }
    }

    const code = randomToken();
    const sessionId = randomToken();
    oidcSessions.set(sessionId, { subject, email });
    codes.set(code, {
      subject,
      email,
      sessionId,
      nonce: query.nonce,
      codeChallenge: query.code_challenge,
      redirectUri: query.redirect_uri,
      createdAt: Date.now(),
    });

    const callback = new URL(query.redirect_uri);
    callback.searchParams.set("code", code);
    callback.searchParams.set("state", query.state);
    return c.redirect(callback.toString());
  }

  const userUrl = (subject: keyof typeof users) => {
    const url = new URL(c.req.url);
    url.searchParams.set("sub", subject);
    return url.toString();
  };

  return c.html(html`
    <!doctype html>
    <html lang="en">
      <head>
        <meta charset="utf-8" />
        <meta name="viewport" content="width=device-width, initial-scale=1" />
        <title>Dev login</title>
      </head>
      <body>
        <main>
          <h1>Pick a dev user</h1>
          <p><a href="${userUrl("alice")}">alice - ${users.alice}</a></p>
          <p><a href="${userUrl("bob")}">bob - ${users.bob}</a></p>
        </main>
      </body>
    </html>
  `);
});

async function issueTokens(
  subject: string,
  email: string,
  sessionId: string,
  nonce?: string,
) {
  const now = Math.floor(Date.now() / 1000);
  const accessToken = await new SignJWT({ sid: sessionId })
    .setProtectedHeader({ alg: "RS256", kid: keyId, typ: "JWT" })
    .setIssuer(issuer)
    .setSubject(subject)
    .setAudience(clientId)
    .setIssuedAt(now)
    .setExpirationTime(now + accessTokenLifetimeSeconds)
    .sign(privateKey);
  const refreshToken = randomToken();
  const refreshExpiresIn = 7 * 24 * 60 * 60;
  refreshTokens.set(refreshToken, {
    subject,
    email,
    sessionId,
    expiresAt: Date.now() + refreshExpiresIn * 1000,
  });

  const response: Record<string, string | number> = {
    access_token: accessToken,
    refresh_token: refreshToken,
    token_type: "Bearer",
    expires_in: accessTokenLifetimeSeconds,
    refresh_expires_in: refreshExpiresIn,
  };
  if (nonce) {
    response.id_token = await new SignJWT({
      email,
      email_verified: true,
      nonce,
      sid: sessionId,
    })
      .setProtectedHeader({ alg: "RS256", kid: keyId, typ: "JWT" })
      .setIssuer(issuer)
      .setSubject(subject)
      .setAudience(clientId)
      .setIssuedAt(now)
      .setExpirationTime(now + 60 * 60)
      .sign(privateKey);
  }
  return response;
}

app.post("/token", async (c) => {
  c.header("Cache-Control", "no-store");
  c.header("Pragma", "no-cache");
  const form = await c.req.parseBody();
  if (
    formValue(form, "client_id") !== clientId ||
    formValue(form, "client_secret") !== clientSecret
  ) {
    return c.json({ error: "invalid_client" }, 401);
  }

  const grantType = formValue(form, "grant_type");
  if (grantType === "authorization_code") {
    const code = formValue(form, "code");
    const pending = codes.get(code);
    codes.delete(code);
    if (
      !pending || Date.now() - pending.createdAt >= 10 * 60 * 1000 ||
      pending.redirectUri !== formValue(form, "redirect_uri") ||
      pending.redirectUri !== redirectUri ||
      pending.codeChallenge !==
        await pkceChallenge(formValue(form, "code_verifier"))
    ) {
      return c.json({ error: "invalid_grant" }, 400);
    }
    return c.json(
      await issueTokens(
        pending.subject,
        pending.email,
        pending.sessionId,
        pending.nonce,
      ),
    );
  }

  if (grantType === "refresh_token") {
    await new Promise((resolve) => setTimeout(resolve, refreshDelayMs));
    const token = formValue(form, "refresh_token");
    const pending = refreshTokens.get(token);
    refreshTokens.delete(token);
    if (!pending || pending.expiresAt <= Date.now()) {
      return c.json({ error: "invalid_grant" }, 400);
    }
    return c.json(
      await issueTokens(pending.subject, pending.email, pending.sessionId),
    );
  }

  return c.json({ error: "unsupported_grant_type" }, 400);
});

app.post("/revoke/:subject", async (c) => {
  const subject = c.req.param("subject");
  const sessions = Array.from(oidcSessions).filter(
    ([, identity]) => identity.subject === subject,
  );
  if (sessions.length === 0) {
    return c.text("OIDC session not found", 404);
  }

  const sessionIds = new Set(sessions.map(([sessionId]) => sessionId));
  for (const [token, refresh] of refreshTokens) {
    if (sessionIds.has(refresh.sessionId)) {
      refreshTokens.delete(token);
    }
  }

  for (const [sessionId] of sessions) {
    const logoutToken = await new SignJWT({
      events: {
        "http://schemas.openid.net/event/backchannel-logout": {},
      },
      sid: sessionId,
    })
      .setProtectedHeader({ alg: "RS256", kid: keyId, typ: "logout+jwt" })
      .setIssuer(issuer)
      .setAudience(clientId)
      .setIssuedAt()
      .setJti(randomToken())
      .sign(privateKey);
    const body = new URLSearchParams({ logout_token: logoutToken });
    const response = await fetch(backchannelLogoutUri, {
      method: "POST",
      headers: { "content-type": "application/x-www-form-urlencoded" },
      body,
    });

    if (!response.ok) {
      return c.text("Back-channel logout failed", 502);
    }

    oidcSessions.delete(sessionId);
  }

  return c.body(null, 204);
});

Deno.serve(
  {
    port,
    onListen: (addr) => {
      console.log(`dev IdP listening at ${addr.hostname}:${addr.port}`);
    },
  },
  app.fetch,
);
