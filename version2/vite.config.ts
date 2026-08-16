import { solidStart } from "@solidjs/start/config";
import tailwindcss from "@tailwindcss/vite";
import { nitro } from "nitro/vite";
import { defineConfig, type Plugin } from "vite";

function safariJsonImports(): Plugin {
  return {
    name: "safari-json-imports",
    configureServer(server) {
      server.middlewares.use((request, _response, next) => {
        if (request.url?.includes("/@solidjs/start/package.json?import")) {
          request.headers["sec-fetch-dest"] = "script";
        }
        next();
      });
    },
  };
}

export default defineConfig({
  plugins: [
    safariJsonImports(),
    tailwindcss(),
    solidStart({ devOverlay: false }),
    nitro({ preset: "node-server" }),
  ],
  clearScreen: false,
  server: { host: true },
});
