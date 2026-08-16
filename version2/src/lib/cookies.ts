import { createHmac, timingSafeEqual } from "node:crypto";
import { config } from "./config";

export const authCookieName = "auth";
export const flowCookieName = "oidc_flow";

export function getCookie(request: Request, name: string) {
  for (const part of (request.headers.get("cookie") || "").split(";")) {
    const [key, ...value] = part.trim().split("=");
    if (key === name) return value.join("=");
  }
}

export function cookie(name: string, value: string, maxAge: number) {
  return [
    `${name}=${value}`,
    "Path=/",
    "HttpOnly",
    "SameSite=Lax",
    `Max-Age=${maxAge}`,
    ...(config.secureCookies ? ["Secure"] : []),
  ].join("; ");
}

export function signedCookie(value: unknown) {
  const body = Buffer.from(JSON.stringify(value)).toString("base64url");
  return `${body}.${sign(body)}`;
}

export function readSignedCookie<T>(value: string | undefined): T | undefined {
  if (!value) return;
  const [body, signature] = value.split(".");
  if (!body || !signature) return;

  const expected = Buffer.from(sign(body));
  const actual = Buffer.from(signature);
  if (expected.length !== actual.length || !timingSafeEqual(expected, actual)) return;

  try {
    return JSON.parse(Buffer.from(body, "base64url").toString()) as T;
  } catch {
    return;
  }
}

function sign(value: string) {
  return createHmac("sha256", config.sessionSecret).update(value).digest("base64url");
}
