// Helm's member and its `helm-talking` moment as a mock Fleet answers them. Desktop's
// `mock/slices/helm.ts` registers them; the moment is generic over the app's whole state and API.
export * from "./fake/helm-api";
export * from "./fake/helm-fleet";
