import { Button } from "../../primitives/Button/Button";

/** The name the field a link was pasted into is given, for the offer to stand beside it. */
export const LINK_OFFER_ANCHOR = "--armada-link-offer";

/**
 * The offer a link pasted into a text box makes: the paste went in as typed, and this stands beside the box for a
 * few seconds in case the link was meant for the link panel. **Anchored by CSS** to the field, which the caller names
 * `LINK_OFFER_ANCHOR` for as long as the offer stands.
 */
export function LinkOffer({ onOpen }: { onOpen: () => void }) {
  return (
    <div className="armada-link-offer" role="group" aria-label="Link pasted">
      <Button type="button" variant="secondary" size="sm" onClick={onOpen}>
        Add a link
      </Button>
    </div>
  );
}
