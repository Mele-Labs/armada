// How a long Uncommitted list is held in the card. Mock-only: `?take=scroll` (the default) caps the list in a box that scrolls,
// `?take=reveal` shows a few lines and a Show all. The owner picks one and the other goes; nothing in the real app sets it.

export type CheckoutTake = "scroll" | "reveal";

/** Set by a scenario, so a walk plays its take without the url naming it. */
let chosen: CheckoutTake | undefined;

export const chooseCheckoutTake = (take: CheckoutTake | undefined): void => {
  chosen = take;
};

export const checkoutTake = (): CheckoutTake =>
  chosen ?? (new URLSearchParams(window.location.search).get("take") === "reveal" ? "reveal" : "scroll");
