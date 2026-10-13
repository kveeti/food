import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("../../", import.meta.url));

export default function globalSetup() {
  if (process.env.E2E_PREVIEW === "1") {
    execFileSync("pnpm", ["run", "build"], {
      cwd: fileURLToPath(new URL("../", import.meta.url)),
      stdio: "inherit",
    });
  }

  execFileSync(
    "cargo",
    [
      "build",
      "--manifest-path",
      "backend/Cargo.toml",
      "--bins",
      "--features",
      "test-support",
    ],
    { cwd: root, stdio: "inherit" },
  );
}
