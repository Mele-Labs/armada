// Fleet's offer of a refresh, on the Job holding a branch the preview left out that has moved since: its
// sentence, the Refresh preview key and Skip. Then the Fleet panel, where the moved branch carries a mark
// and the one that has not moved does not. Over `scenarios/preview-offer.ts`.

import { button, deckRow, inside, region, role, text, walk } from "../walk";

const needs = region("Needs you");
const offer = inside(needs, role("listitem", /^feat\/billing-export moved/));

const previewOffer = walk("preview-offer", [
  { press: deckRow(/Export billing as CSV/), say: "The Job holding feat/billing-export" },
  { press: inside(needs, button(/^Needs you/)), say: "Its Needs you panel, with the offer under Commands" },
  { look: inside(offer, text(/moved since the preview left it out/)), say: "The branch, that it moved since the preview left it out, and why it was left out" },
  { hover: inside(offer, button("Refresh preview")), say: "Refresh preview runs scripts/preview --restart" },
  { press: inside(offer, button(/^Skip/)), say: "Skip dismisses the offer. The agent is told nothing" },
  { look: text(/ledger conflict resolved/), say: "The offer is gone from Needs you" },]);

const previewOfferFleetPanel = walk("preview-offer", [
  { press: button(/^Fleet — /), say: "The Fleet panel" },
  { look: role("list", "Left out of the preview"), say: "Two branches the preview left out. feat/billing-export has a mark beside it, feat/invoice-pdf has none" },
  { hover: role("img", "Moved since it was left out", { exact: true }), say: "Moved since it was left out" },
  { hover: button("Refresh preview"), say: "Refresh preview, as before" },
]);

export { previewOffer as "preview-offer", previewOfferFleetPanel as "preview-offer-fleet-panel" };
