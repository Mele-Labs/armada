/// <reference types="vite/client" />
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import "./styles.css";
import "@armada/components/src/look";
import "./pocket.css";
import { Root } from "./App";

if ("serviceWorker" in navigator) void navigator.serviceWorker.register("/sw.js");

async function begin() {
  // The fixture Gateway's two Macs, paired before the app reads its list (`pnpm fixture`).
  if (import.meta.env.MODE === "fixture") await (await import("./fixture/seed")).seed();
  createRoot(document.getElementById("root")!).render(
    <StrictMode>
      <Root />
    </StrictMode>,
  );
}

void begin();
