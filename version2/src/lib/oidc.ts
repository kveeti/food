import * as oidc from "openid-client";
import { config } from "./config";

let client: Promise<oidc.Configuration> | undefined;

export function oidcClient() {
  if (!client) {
    client = oidc
      .discovery(
        new URL(config.oidcIssuer),
        config.oidcClientId,
        config.oidcClientSecret,
        undefined,
        config.oidcIssuer.startsWith("http://")
          ? { execute: [oidc.allowInsecureRequests] }
          : undefined,
      )
      .catch((error) => {
        client = undefined;
        throw error;
      });
  }
  return client;
}

export { oidc };
