import type { APIEvent } from "@solidjs/start/server";
import { cookie, flowCookieName, signedCookie } from "../../lib/cookies";
import { config } from "../../lib/config";
import { oidc, oidcClient } from "../../lib/oidc";

type Flow = {
  state: string;
  nonce: string;
  verifier: string;
  expiresAt: number;
};

export async function GET(_event: APIEvent) {
  const client = await oidcClient();
  const verifier = oidc.randomPKCECodeVerifier();
  const flow: Flow = {
    state: oidc.randomState(),
    nonce: oidc.randomNonce(),
    verifier,
    expiresAt: Date.now() + 10 * 60 * 1000,
  };
  const location = oidc.buildAuthorizationUrl(client, {
    redirect_uri: config.oidcRedirectUrl,
    scope: "openid email",
    state: flow.state,
    nonce: flow.nonce,
    code_challenge: await oidc.calculatePKCECodeChallenge(verifier),
    code_challenge_method: "S256",
  });

  return new Response(null, {
    status: 302,
    headers: {
      location: location.href,
      "set-cookie": cookie(flowCookieName, signedCookie(flow), 10 * 60),
    },
  });
}
