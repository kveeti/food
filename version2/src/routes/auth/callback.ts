import type { APIEvent } from "@solidjs/start/server";
import {
  authCookieName,
  cookie,
  flowCookieName,
  getCookie,
  readSignedCookie,
} from "../../lib/cookies";
import { config } from "../../lib/config";
import { createUserAndSession } from "../../lib/data/session";
import { oidc, oidcClient } from "../../lib/oidc";

type Flow = {
  state: string;
  nonce: string;
  verifier: string;
  expiresAt: number;
};

export async function GET(event: APIEvent) {
  const flow = readSignedCookie<Flow>(getCookie(event.request, flowCookieName));
  if (!flow || flow.expiresAt < Date.now()) {
    return new Response("Invalid or expired OIDC flow", { status: 400 });
  }

  try {
    const requestUrl = new URL(event.request.url);
    const callbackUrl = new URL(config.oidcRedirectUrl);
    callbackUrl.search = requestUrl.search;
    const tokens = await oidc.authorizationCodeGrant(await oidcClient(), callbackUrl, {
      expectedState: flow.state,
      expectedNonce: flow.nonce,
      pkceCodeVerifier: flow.verifier,
      idTokenExpected: true,
    });
    const claims = tokens.claims();
    if (!claims?.sub || typeof claims.email !== "string") {
      return new Response("OIDC provider did not return an email", { status: 401 });
    }

    const session = await createUserAndSession({
      issuer: claims.iss,
      subject: claims.sub,
      email: claims.email,
    });
    const headers = new Headers({ location: config.appUrl });
    headers.append("set-cookie", cookie(authCookieName, session.token, session.maxAge));
    headers.append("set-cookie", cookie(flowCookieName, "", 0));
    return new Response(null, { status: 302, headers });
  } catch (error) {
    console.error("OIDC callback failed", error);
    return new Response("OIDC login failed", { status: 401 });
  }
}
