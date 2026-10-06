// The mock harness a surface's tests mount the whole app with, as one entry.
// `@armada/desktop/mock` points here. Add to it only what a test outside
// `apps/desktop` needs.

export * from "./fake";
export * from "./mount";
export * from "./scenario";
export * from "./testing";
