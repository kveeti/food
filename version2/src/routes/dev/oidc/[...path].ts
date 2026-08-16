import type { APIEvent } from "@solidjs/start/server";
import { authorize, discovery, jwks, token } from "../../../lib/dev-oidc";

export async function GET(event: APIEvent) {
  if (!import.meta.env.DEV) return new Response("Not found", { status: 404 });

  switch (event.params.path) {
    case ".well-known/openid-configuration":
      return Response.json(await discovery());
    case "jwks.json":
      return Response.json(await jwks());
    case "authorize":
      return authorize(new URL(event.request.url));
    default:
      return new Response("Not found", { status: 404 });
  }
}

export async function POST(event: APIEvent) {
  if (!import.meta.env.DEV || event.params.path !== "token") {
    return new Response("Not found", { status: 404 });
  }
  return token(event.request);
}
