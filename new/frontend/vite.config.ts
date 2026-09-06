import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

export default defineConfig({
  cacheDir: process.env.VITE_CACHE_DIR ?? "node_modules/.vite",
  plugins: [react(), tailwindcss()],
  server: {
    host: "127.0.0.1",
    port: Number(process.env.VITE_PORT ?? 3000),
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
  },
});
