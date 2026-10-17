// Which renderer draws the Cockpit map. The CSS map is the default; `?map=pixi` on the page (the mock, or a walk link) draws the
// canvas one instead, so the two can be set side by side until the owner has chosen. Read once: a page does not change renderer
// under him.

export type MapRenderer = "css" | "pixi";

const asked: MapRenderer = typeof window !== "undefined" && new URLSearchParams(window.location.search).get("map") === "pixi" ? "pixi" : "css";

export const mapRenderer = (): MapRenderer => asked;
