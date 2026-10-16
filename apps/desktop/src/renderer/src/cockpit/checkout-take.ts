// How a long Uncommitted list is held in the card. Mock-only: `?take=scroll` (the default) caps the list in a box that scrolls,
// `?take=reveal` shows a few lines and a Show all. The owner picks one and the other goes; nothing in the real app sets it.

export type CheckoutTake = "scroll" | "reveal";

export const checkoutTake = (): CheckoutTake => (new URLSearchParams(window.location.search).get("take") === "reveal" ? "reveal" : "scroll");
