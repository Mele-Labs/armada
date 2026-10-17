// Which way the Needs you panel joins the composer, read from the address. Mock scaffolding: `?take=band` picks the band, anything else the
// eyebrow, and this file goes once one is chosen.

import type { NeedsYouTake } from "@armada/components";

export const needsTake = (): NeedsYouTake => (new URLSearchParams(window.location.search).get("take") === "band" ? "band" : "eyebrow");
