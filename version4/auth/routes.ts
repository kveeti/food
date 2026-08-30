import type { Hono } from "hono";
import { deleteCookie, getCookie, setCookie } from "hono/cookie";
import { config } from "../config.ts";
import { deleteAuthSessionsByOidcSession } from "../data/sessions.ts";
import {
  createAuthorizationRequest,
  exchangeAuthorizationCode,
  verifyBackchannelLogout,
} from "./oidc.ts";
import { createSession, deleteSession, sessionCookie } from "./sessions.ts";
import type { AppEnv } from "./types.ts";

const stateCookie = "oidc_state";
const nonceCookie = "oidc_nonce";
const codeVerifierCookie = "oidc_code_verifier";
const returnToCookie = "oidc_return_to";

function safeReturnPath(value: string | undefined): string {
  if (!value) {
    return "/";
  }

  try {
    const url = new URL(value, config.appUrl);
    return url.origin === config.appUrl ? `${url.pathname}${url.search}` : "/";
  } catch {
    return "/";
  }
}

export function registerAuth(app: Hono<AppEnv>): void {
  app.get("/auth/sign-in", async (c) => {
    const requestUrl = new URL(c.req.url);
    const returnTo = safeReturnPath(
      requestUrl.searchParams.get("return_to") ?? undefined,
    );

    if (requestUrl.host !== new URL(config.appUrl).host) {
      const signInUrl = new URL("/auth/sign-in", config.appUrl);
      signInUrl.searchParams.set("return_to", returnTo);
      return c.redirect(signInUrl.toString());
    }

    const { url, state, nonce, codeVerifier } =
      await createAuthorizationRequest();
    const cookieOptions = {
      path: "/",
      httpOnly: true,
      secure: config.secureCookies,
      sameSite: "lax" as const,
      maxAge: 10 * 60,
    };

    setCookie(c, stateCookie, state, cookieOptions);
    setCookie(c, nonceCookie, nonce, cookieOptions);
    setCookie(c, codeVerifierCookie, codeVerifier, cookieOptions);
    setCookie(c, returnToCookie, returnTo, cookieOptions);
    return c.redirect(url.toString());
  });

  app.post("/auth/backchannel-logout", async (c) => {
    const body = await c.req.parseBody();
    const token = body.logout_token;
    if (typeof token !== "string") {
      return c.text("Missing logout token", 400);
    }

    try {
      const logout = await verifyBackchannelLogout(token);
      await deleteAuthSessionsByOidcSession(
        logout.issuer,
        logout.sessionId,
      );

      console.info("OIDC back-channel logout completed");
      return c.body(null, 200);
    } catch (error) {
      console.warn(
        "OIDC back-channel logout rejected",
        error instanceof Error ? error.message : String(error),
      );
      return c.text("Invalid logout token", 400);
    }
  });

  app.get("/auth/callback", async (c) => {
    const state = getCookie(c, stateCookie);
    const nonce = getCookie(c, nonceCookie);
    const codeVerifier = getCookie(c, codeVerifierCookie);
    const returnTo = safeReturnPath(getCookie(c, returnToCookie));

    deleteCookie(c, stateCookie, { path: "/" });
    deleteCookie(c, nonceCookie, { path: "/" });
    deleteCookie(c, codeVerifierCookie, { path: "/" });
    deleteCookie(c, returnToCookie, { path: "/" });

    if (!state || !nonce || !codeVerifier) {
      return c.text("Missing OIDC login cookie", 400);
    }

    try {
      const requestUrl = new URL(c.req.url);
      const callbackUrl = new URL(
        `${requestUrl.pathname}${requestUrl.search}`,
        config.appUrl,
      );
      const login = await exchangeAuthorizationCode({
        callbackUrl,
        state,
        nonce,
        codeVerifier,
      });
      const session = await createSession(
        login.identity,
        login.oidcSessionId,
        login.tokens,
      );

      setCookie(c, sessionCookie, session.token, {
        path: "/",
        httpOnly: true,
        secure: config.secureCookies,
        sameSite: "lax",
        expires: session.expiresAt,
      });
      return c.redirect(returnTo, 303);
    } catch (error) {
      console.error("OIDC callback failed", error);
      return c.text("OIDC login failed", 401);
    }
  });

  app.post("/logout", async (c) => {
    const token = getCookie(c, sessionCookie);
    if (token) {
      await deleteSession(token);
    }

    deleteCookie(c, sessionCookie, { path: "/" });
    return c.redirect("/sign-in", 303);
  });
}
