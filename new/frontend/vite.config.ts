import babel from "@rolldown/plugin-babel";
import tailwindcss from "@tailwindcss/vite";
import react, { reactCompilerPreset } from "@vitejs/plugin-react";
import { defineConfig } from "vite";

export default defineConfig({
  plugins: [
    react(),
    babel({ presets: [reactCompilerPreset()] }),
    tailwindcss(),
  ],
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
