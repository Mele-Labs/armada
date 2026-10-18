// Takes of the Cockpit the mock offers on `?take=`, set on the root element by `mock/main.tsx`.

/** Take `no-calls`: the Cockpit draws no Incoming calls panel, and the Work list's pulsing rows carry the calls. */
export const withoutCalls = (): boolean => document.documentElement.getAttribute("data-take") === "no-calls";
