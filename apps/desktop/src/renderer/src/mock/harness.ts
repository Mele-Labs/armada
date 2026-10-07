// The mock harness a surface's tests mount the whole app with, as one entry.
// `@armada/desktop/mock` points here. Add to it only what a test outside
// `apps/desktop` needs.

export * from "./fake";
export * from "./mount";
export * from "./scenario";
export * from "./testing";

// Studios' mock fleet lives in `@armada/studios/fake`; its tests reach it, fixed to desktop's
// state, through the harness.
export * from "./studios-fake";

// Manifest's mock Fleet lives in `@armada/manifest/fake`; its tests reach it, fixed to desktop's
// state, through the harness.
export * from "./manifest-fake";

// Setup's mock Fleet lives in `@armada/setup/fake`; its tests reach it, fixed to desktop's state,
// through the harness.
export * from "./setup-fake";
// Both surfaces keep the same ended Verify sheet; the harness hands out Setup's.
export { VERIFY_ENDED } from "./setup-fake";
export * from "./scrolled";
