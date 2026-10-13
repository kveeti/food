import { renderToReadableStream } from "react-dom/server.edge";
import type { PrerenderArguments } from "vite-prerender-plugin";

import { Root } from "./root.tsx";

export async function prerender({ url }: PrerenderArguments) {
  if (url !== "/sign-in") return { html: "" };

  const stream = await renderToReadableStream(<Root ssrPath={url} />);
  await stream.allReady;

  return { html: await new Response(stream).text() };
}
