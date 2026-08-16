// @refresh reload
import { mount, StartClient } from "@solidjs/start/client";

if (import.meta.env.DEV && new URLSearchParams(location.search).has("pwa")) {
  document.documentElement.classList.add("dev-pwa");
}

mount(() => <StartClient />, document.getElementById("app")!);
