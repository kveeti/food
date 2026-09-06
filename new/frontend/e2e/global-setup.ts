import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("../../", import.meta.url));

export default function globalSetup() {
  execFileSync(
    "cargo",
    ["build", "--manifest-path", "backend/Cargo.toml", "--bins"],
    { cwd: root, stdio: "inherit" },
  );
}
