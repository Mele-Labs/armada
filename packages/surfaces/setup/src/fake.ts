// Setup's members and its `settingUp` moment as a mock Fleet answers them. Desktop's
// `mock/setup-fake.ts` and `mock/slices/setup.ts` register them; the moment is generic over the app's whole state and API.
export * from "./fake/setup-api";
export * from "./fake/setup-fleet";
