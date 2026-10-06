// The mock harness a surface's tests mount the whole app with, as one entry.
// `@armada/desktop/mock` points here. Add to it only what a test outside
// `apps/desktop` needs.

export * from "./fake";
export * from "./mount";
export * from "./scenario";
export * from "./testing";

// Studios' mock fleet stays here beside `fake.ts`, which composes it, so its
// tests reach it through the harness.
export * from "./studio-fleet";
export * from "./studio-read-in";
export * from "./studio-read-nothing";
export * from "./studio-zone-proposal";
export { sheet } from "./manifest-fleet";
