import { createHash, randomBytes } from "node:crypto";
import { exportJWK, generateKeyPair, SignJWT, type CryptoKey } from "jose";
import { config } from "./config";

type Pending = {
  clientId: string;
  redirectUri: string;
  nonce: string;
  codeChallenge: string;
  subject: string;
  email: string;
};

type DevOidcState = {
  keys: Promise<{ privateKey: CryptoKey; publicKey: CryptoKey }>;
  codes: Map<string, Pending>;
  users: Map<string, string>;
};

const globals = globalThis as typeof globalThis & { __foodDevOidc?: DevOidcState };
export const devOidc = (globals.__foodDevOidc ??= {
  keys: generateKeyPair("RS256", { extractable: true }),
  codes: new Map(),
  users: new Map([
    ["alice", "alice@dev.local"],
    ["bob", "bob@dev.local"],
  ]),
});

export const devIssuer = () => config.oidcIssuer.replace(/\/$/, "");

export async function discovery() {
  const issuer = devIssuer();
  return {
    issuer,
    authorization_endpoint: `${issuer}/authorize`,
    token_endpoint: `${issuer}/token`,
    jwks_uri: `${issuer}/jwks.json`,
    response_types_supported: ["code"],
    subject_types_supported: ["public"],
    id_token_signing_alg_values_supported: ["RS256"],
    token_endpoint_auth_methods_supported: ["client_secret_post"],
    scopes_supported: ["openid", "email"],
    code_challenge_methods_supported: ["S256"],
  };
}

export async function jwks() {
  const jwk = await exportJWK((await devOidc.keys).publicKey);
  return { keys: [{ ...jwk, use: "sig", alg: "RS256", kid: "dev" }] };
}

export function authorize(url: URL) {
  const query = url.searchParams;
  const redirectUri = query.get("redirect_uri");
  const clientId = query.get("client_id");
  const state = query.get("state");
  const nonce = query.get("nonce");
  const challenge = query.get("code_challenge");
  if (
    !redirectUri ||
    clientId !== config.oidcClientId ||
    !state ||
    !nonce ||
    !challenge ||
    query.get("response_type") !== "code" ||
    query.get("code_challenge_method") !== "S256"
  ) {
    return new Response("Invalid authorization request", { status: 400 });
  }

  const subject = query.get("sub");
  if (!subject) return picker(url);

  const email = devOidc.users.get(subject) || `${subject}@dev.local`;
  devOidc.users.set(subject, email);
  const code = random();
  devOidc.codes.set(code, {
    clientId,
    redirectUri,
    nonce,
    codeChallenge: challenge,
    subject,
    email,
  });

  const callback = new URL(redirectUri);
  callback.searchParams.set("code", code);
  callback.searchParams.set("state", state);
  return Response.redirect(callback, 302);
}

export async function token(request: Request) {
  const form = await request.formData();
  const code = String(form.get("code") || "");
  const pending = devOidc.codes.get(code);
  devOidc.codes.delete(code);

  if (
    !pending ||
    form.get("grant_type") !== "authorization_code" ||
    form.get("client_id") !== pending.clientId ||
    form.get("client_secret") !== config.oidcClientSecret ||
    form.get("redirect_uri") !== pending.redirectUri ||
    challenge(String(form.get("code_verifier") || "")) !== pending.codeChallenge
  ) {
    return Response.json({ error: "invalid_grant" }, { status: 400 });
  }

  const now = Math.floor(Date.now() / 1000);
  const idToken = await new SignJWT({ email: pending.email, nonce: pending.nonce })
    .setProtectedHeader({ alg: "RS256", kid: "dev" })
    .setIssuer(devIssuer())
    .setSubject(pending.subject)
    .setAudience(pending.clientId)
    .setIssuedAt(now)
    .setExpirationTime(now + 3600)
    .sign((await devOidc.keys).privateKey);

  return Response.json({
    access_token: random(),
    token_type: "Bearer",
    expires_in: 3600,
    id_token: idToken,
  });
}

function picker(url: URL) {
  const links = [...devOidc.users].map(([subject, email]) => {
    const target = new URL(url);
    target.searchParams.set("sub", subject);
    return `<a href="${escape(target.pathname + target.search)}">${escape(subject)} — ${escape(email)}</a>`;
  });
  const hidden = [...url.searchParams]
    .map(([name, value]) => `<input type="hidden" name="${escape(name)}" value="${escape(value)}">`)
    .join("");

  return new Response(
    `<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1"><title>Dev login</title><style>body{font:16px system-ui;max-width:24rem;margin:4rem auto;padding:0 1rem}a,form{display:block;margin:.75rem 0}input,button{padding:.5rem}</style></head><body><h1>Pick a dev user</h1>${links.join("")}<form action="${escape(url.pathname)}">${hidden}<input name="sub" placeholder="new-user-sub" required><button>Log in as new user</button></form></body></html>`,
    { headers: { "content-type": "text/html; charset=utf-8" } },
  );
}

function random() {
  return randomBytes(24).toString("base64url");
}

function challenge(verifier: string) {
  return createHash("sha256").update(verifier).digest("base64url");
}

function escape(value: string) {
  return value.replace(/[&<>"']/g, (char) => `&#${char.charCodeAt(0)};`);
}
