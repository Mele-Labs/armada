// A sign-in flow an agent drew with every shape and colour a sketch has: a pill to start, a diamond to decide, a
// cylinder for the store, a hexagon for the work, a circle for a step on the side. Mock only, as `sketches.ts` is.

import type { SceneNode } from "@armada/components";

import { arrow, box, scene } from "../scenes";

export const SHAPES_TITLE = "How a request gets signed in";

const shaped = (node: SceneNode, shape: NonNullable<SceneNode["shape"]>, color: NonNullable<SceneNode["color"]>): SceneNode => ({ ...node, shape, color });

export const SHAPES = scene(
  [
    shaped(box("start", 0, 40, "Request arrives"), "pill", "green"),
    shaped(box("read", 320, 0, "Read the cookie", "Session id from the request headers", { icon: "lock" }), "box", "blue"),
    shaped(box("signed", 680, 0, "Signed in?"), "diamond", "amber"),
    shaped(box("build", 1060, 0, "Build the page", "Render with the user's own data"), "hexagon", "teal"),
    shaped(box("store", 680, 260, "Sessions", "SQLite, one row a session"), "cylinder", "grey"),
    shaped(box("login", 1060, 260, "Sign in"), "circle", "orange"),
    shaped(box("refused", 1360, 260, "Refused", "Three tries in a minute"), "box", "red"),
  ],
  [
    arrow("start", "read"),
    arrow("read", "signed"),
    arrow("signed", "store", "looks up"),
    { ...arrow("signed", "build", "yes"), color: "green" },
    { ...arrow("signed", "login", "no"), color: "orange" },
    { ...arrow("login", "refused", "too many"), color: "red" },
  ],
);
