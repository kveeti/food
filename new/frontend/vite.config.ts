import { fileURLToPath } from "node:url";

import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import { defineConfig, type Plugin } from "vite";
import { vitePrerenderPlugin } from "vite-prerender-plugin";

export default defineConfig({
  cacheDir: process.env.VITE_CACHE_DIR ?? "node_modules/.vite",
  plugins: [react(), tailwindcss(), ...prerenderLogin()],
  server: {
    host: process.env.VITE_HOST ?? "127.0.0.1",
    port: Number(process.env.VITE_PORT ?? 3000),
    strictPort: true,
    proxy: {
      "/api": {
        target: process.env.BACKEND_URL ?? "http://127.0.0.1:8000",
        changeOrigin: false,
      },
      "/auth": {
        target: process.env.BACKEND_URL ?? "http://127.0.0.1:8000",
        changeOrigin: false,
      },
      "/logout": {
        target: process.env.BACKEND_URL ?? "http://127.0.0.1:8000",
        changeOrigin: false,
      },
    },
  },
  clearScreen: false,
  build: {
    outDir: "dist",
    target: "esnext",
    rolldownOptions: {
      output: [
        {
          codeSplitting: {
            groups: [
              {
                name: "react-server",
                test: /\/react-dom\/(?:server|cjs\/react-dom-server)/,
                includeDependenciesRecursively: false,
              },
              {
                name: "react",
                test: /\/node_modules\/(?:react|react-dom|scheduler)\//,
              },
            ],
          },
        },
      ],
    },
  },
});

function prerenderLogin() {
  const page = "/sign-in";

  const [prerenderPlugin] = vitePrerenderPlugin({
    renderTarget: "#root",
    prerenderScript: fileURLToPath(
      new URL("./src/prerender.tsx", import.meta.url),
    ),
    additionalPrerenderRoutes: ["/sign-in"],
  });

  const previewServerPlugin = {
    name: "sign-in-preview",
    configurePreviewServer(server) {
      server.middlewares.use((req, _res, next) => {
        const url = new URL(req.url ?? "/", "http://localhost");
        if (url.pathname === page) {
          req.url = `${page}/index.html${url.search}`;
        }
        next();
      });
    },
  } satisfies Plugin;

  return [prerenderPlugin, previewServerPlugin];
}
