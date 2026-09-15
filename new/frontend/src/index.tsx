import { createRoot, hydrateRoot } from "react-dom/client";

import { Root } from "./root.tsx";

import "./styles.css";

const container = document.getElementById("root")!;

if (container.hasChildNodes()) {
  hydrateRoot(container, <Root />);
} else {
  createRoot(container).render(<Root />);
}
