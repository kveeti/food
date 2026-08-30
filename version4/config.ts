function requireEnv(name: string): string {
  const value = Deno.env.get(name);
  if (!value) {
    throw new Error(`${name} must be set`);
  }
  return value;
}

function parsePort(name: string, fallback: string): number {
  const port = Number(Deno.env.get(name) ?? fallback);
  if (!Number.isInteger(port) || port < 1 || port > 65_535) {
    throw new Error(`${name} must be a number between 1 and 65535`);
  }
  return port;
}

function parseEncryptionKey(): Uint8Array {
  let key: Uint8Array;
  try {
    key = Uint8Array.from(
      atob(requireEnv("SESSION_ENCRYPTION_KEY")),
      (character) => character.charCodeAt(0),
    );
  } catch {
    throw new Error("SESSION_ENCRYPTION_KEY must be valid base64");
  }
  if (key.length !== 32) {
    throw new Error("SESSION_ENCRYPTION_KEY must contain 32 bytes");
  }
  return key;
}

function parseUrl(name: string): URL {
  try {
    return new URL(requireEnv(name));
  } catch {
    throw new Error(`${name} must be a URL`);
  }
}

const appUrl = parseUrl("APP_URL");
if (appUrl.pathname !== "/" || appUrl.search || appUrl.hash) {
  throw new Error("APP_URL must contain only an origin");
}

const oidcIssuer = parseUrl("OIDC_ISSUER");
const allowInsecureOidc = Deno.env.get("ALLOW_INSECURE_OIDC") === "1";
if (oidcIssuer.protocol !== "https:" && !allowInsecureOidc) {
  throw new Error("OIDC_ISSUER must use HTTPS");
}

export const config = {
  databaseUrl: requireEnv("DATABASE_URL"),
  port: parsePort("PORT", "8000"),
  appUrl: appUrl.origin,
  oidcIssuer,
  oidcClientId: requireEnv("OIDC_CLIENT_ID"),
  oidcClientSecret: requireEnv("OIDC_CLIENT_SECRET"),
  oidcAudience: Deno.env.get("OIDC_AUDIENCE") || undefined,
  allowInsecureOidc,
  sessionEncryptionKey: parseEncryptionKey(),
  secureCookies: appUrl.protocol === "https:",
} as const;
