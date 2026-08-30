import { createRemoteJWKSet, jwtVerify } from "jose";
import {
  allowInsecureRequests,
  authorizationCodeGrant,
  buildAuthorizationUrl,
  calculatePKCECodeChallenge,
  type Configuration,
  discovery,
  randomNonce,
  randomPKCECodeVerifier,
  randomState,
  refreshTokenGrant,
  ResponseBodyError,
  type TokenEndpointResponse,
  type TokenEndpointResponseHelpers,
} from "openid-client";
import { config } from "../config.ts";
import type { AccessClaims, OidcIdentity, ProviderTokens } from "./types.ts";

let oidcConfiguration: Promise<Configuration> | undefined;
let accessTokenKeys: ReturnType<typeof createRemoteJWKSet> | undefined;

function getOidcConfiguration(): Promise<Configuration> {
  if (!oidcConfiguration) {
    const options = config.allowInsecureOidc
      ? { execute: [allowInsecureRequests], timeout: 10 }
      : { timeout: 10 };

    oidcConfiguration = discovery(
      config.oidcIssuer,
      config.oidcClientId,
      config.oidcClientSecret,
      undefined,
      options,
    ).catch((error) => {
      oidcConfiguration = undefined;
      throw error;
    });
  }

  return oidcConfiguration;
}

export async function initializeOidc(): Promise<void> {
  let lastError: unknown;

  for (let attempt = 1; attempt <= 10; attempt++) {
    try {
      await getOidcConfiguration();
      return;
    } catch (error) {
      lastError = error;
      console.warn("OIDC discovery failed, retrying", {
        attempt,
        error: error instanceof Error ? error.message : String(error),
      });
      await new Promise((resolve) => setTimeout(resolve, 1000));
    }
  }

  throw lastError;
}

function audienceContains(audience: unknown, expected: string): boolean {
  return audience === expected ||
    Array.isArray(audience) && audience.includes(expected);
}

export async function verifyAccessToken(
  token: string,
): Promise<AccessClaims> {
  const oidc = await getOidcConfiguration();
  const metadata = oidc.serverMetadata();

  if (!metadata.jwks_uri) {
    throw new Error("OIDC provider has no JWKS URI");
  }

  accessTokenKeys ??= createRemoteJWKSet(new URL(metadata.jwks_uri));
  const { payload } = await jwtVerify(token, accessTokenKeys, {
    issuer: metadata.issuer,
  });

  if (!payload.sub || !payload.iss || !payload.exp) {
    throw new Error("Access token has no identity or expiry");
  }

  const expectedAudience = config.oidcAudience;
  const validAudience = expectedAudience
    ? audienceContains(payload.aud, expectedAudience)
    : audienceContains(payload.aud, config.oidcClientId) ||
      payload.azp === config.oidcClientId;

  if (!validAudience) {
    throw new Error("Access token has the wrong audience");
  }

  return {
    issuer: payload.iss,
    subject: payload.sub,
    expiresAt: new Date(payload.exp * 1000),
  };
}

async function readProviderTokens(
  response: TokenEndpointResponse & TokenEndpointResponseHelpers,
  previousRefreshToken?: string,
): Promise<ProviderTokens> {
  if (!response.access_token) {
    throw new Error("OIDC provider returned no access token");
  }

  const refreshToken = response.refresh_token ?? previousRefreshToken;
  if (!refreshToken) {
    throw new Error("OIDC provider returned no refresh token");
  }

  const refreshExpiresIn = response.refresh_expires_in;
  if (
    typeof refreshExpiresIn !== "number" ||
    !Number.isFinite(refreshExpiresIn) ||
    refreshExpiresIn <= 0
  ) {
    throw new Error("OIDC provider returned no refresh token lifetime");
  }

  const accessClaims = await verifyAccessToken(response.access_token);
  const identity = response.claims();

  if (
    identity &&
    (identity.iss !== accessClaims.issuer ||
      identity.sub !== accessClaims.subject)
  ) {
    throw new Error("OIDC tokens identify different users");
  }

  return {
    accessToken: response.access_token,
    refreshToken,
    refreshExpiresAt: new Date(Date.now() + refreshExpiresIn * 1000),
    accessClaims,
  };
}

export async function verifyBackchannelLogout(token: string): Promise<{
  issuer: string;
  sessionId: string;
}> {
  const oidc = await getOidcConfiguration();
  const metadata = oidc.serverMetadata();

  if (!metadata.jwks_uri) {
    throw new Error("OIDC provider has no JWKS URI");
  }

  accessTokenKeys ??= createRemoteJWKSet(new URL(metadata.jwks_uri));
  const { payload } = await jwtVerify(token, accessTokenKeys, {
    issuer: metadata.issuer,
    audience: config.oidcClientId,
  });
  const sessionId = typeof payload.sid === "string" ? payload.sid : undefined;
  const events = payload.events;

  if (
    !payload.iss || !payload.iat || !payload.jti ||
    typeof events !== "object" || events === null ||
    !("http://schemas.openid.net/event/backchannel-logout" in events) ||
    payload.nonce !== undefined || !sessionId
  ) {
    throw new Error("Invalid OIDC back-channel logout token");
  }

  return {
    issuer: payload.iss,
    sessionId,
  };
}

export async function createAuthorizationRequest(): Promise<{
  url: URL;
  state: string;
  nonce: string;
  codeVerifier: string;
}> {
  const oidc = await getOidcConfiguration();
  const state = randomState();
  const nonce = randomNonce();
  const codeVerifier = randomPKCECodeVerifier();
  const codeChallenge = await calculatePKCECodeChallenge(codeVerifier);
  const url = buildAuthorizationUrl(oidc, {
    redirect_uri: `${config.appUrl}/auth/callback`,
    scope: "openid email",
    code_challenge: codeChallenge,
    code_challenge_method: "S256",
    state,
    nonce,
  });

  return { url, state, nonce, codeVerifier };
}

export async function exchangeAuthorizationCode(input: {
  callbackUrl: URL;
  state: string;
  nonce: string;
  codeVerifier: string;
}): Promise<{
  identity: OidcIdentity;
  oidcSessionId: string;
  tokens: ProviderTokens;
}> {
  const oidc = await getOidcConfiguration();
  const response = await authorizationCodeGrant(
    oidc,
    input.callbackUrl,
    {
      pkceCodeVerifier: input.codeVerifier,
      expectedState: input.state,
      expectedNonce: input.nonce,
      idTokenExpected: true,
    },
  );
  const claims = response.claims();

  if (!claims?.iss || !claims.sub || typeof claims.sid !== "string") {
    throw new Error("OIDC provider returned no identity or session ID");
  }

  const tokens = await readProviderTokens(response);
  if (
    tokens.accessClaims.issuer !== claims.iss ||
    tokens.accessClaims.subject !== claims.sub
  ) {
    throw new Error("OIDC tokens identify different users");
  }

  return {
    identity: {
      issuer: claims.iss,
      subject: claims.sub,
      email: typeof claims.email === "string" ? claims.email : null,
    },
    oidcSessionId: claims.sid,
    tokens,
  };
}

export async function refreshProviderTokens(
  refreshToken: string,
): Promise<ProviderTokens> {
  const oidc = await getOidcConfiguration();
  const response = await refreshTokenGrant(oidc, refreshToken);
  return await readProviderTokens(response, refreshToken);
}

export function isInvalidGrant(error: unknown): boolean {
  return error instanceof ResponseBodyError && error.error === "invalid_grant";
}
