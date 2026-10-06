// Manifest's members and its `manifesting` moment as a mock Fleet answers them. Desktop's
// `mock/manifest-fake.ts` and `mock/slices/manifest.ts` register them; the moment is generic over the app's whole state and API.
export * from "./fake/kit-inventory";
export * from "./fake/manifest-api";
export * from "./fake/manifest-applied";
export * from "./fake/manifest-fleet";
export * from "./fake/manifest-rootless";
