// Bridge's renderer in a plain browser, on a fake `window.armada` — `pnpm mock`.
// Its own Vite config because electron-vite's builds a main process and a
// preload this page has neither of. `docs/practices/running-locally.md`.
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import { annotationsServer } from "./src/main/annotations-server";
import { annotationSource } from "./src/main/annotations-source";
import { annotationStories } from "./src/main/annotations-stories";

export default defineConfig({
  root: "src/renderer/src/mock",
  // `annotationsServer` saves the dev annotation layer's notes (#1226) to
  // `.armada/annotations/`, as main does inside Electron. `annotationSource`
  // stamps each element with the JSX that drew it, which is what lets a note
  // name a file and a line (#1584) — this config alone, see that module.
  // `annotationStories` serves the index of the components' stories, which the
  // note card draws a component's file and its variants from.
  plugins: [react({ babel: { plugins: [annotationSource()] } }), tailwindcss(), annotationsServer(), annotationStories()],
  // The story files the card's Variant tool draws load only when the tool opens, and they import
  // `storybook/test`, which Vite would then find, bundle and reload the page for, mid-walk.
  // Crawled here, it is bundled at start with the rest.
  optimizeDeps: { entries: ["index.html", "../../../../../../packages/components/src/**/*.stories.tsx"] },
  // No `server.open`: `pnpm mock` asks for a browser with `--open`, and a Job
  // serving this for Evidence (`armada.yml`) must not open one on the owner's screen.
});
