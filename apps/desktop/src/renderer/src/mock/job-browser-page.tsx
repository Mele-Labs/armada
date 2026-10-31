// The pages the mock's Job browser shows: an invented shop's cart, and its checkout, where the discount is applied and the total
// does not take it. Any other address is the shop's own not found page.
// A web page and not a Bridge surface, so it is drawn plain, from the primitives, the way a page in the pane would be.

import { Button, Input } from "@armada/components";

import { requestPermission } from "../job-browser/store";

/** What is in the code's field, kept beside the page so the page in a window is the page that was docked. */
let code = "SPRING10";

export function resetCheckout(): void {
  code = "SPRING10";
}

export function CheckoutPage({ sessionId }: { sessionId?: string }) {
  return (
    <article className="armada-mock-checkout" aria-label="Fernwood Supply checkout">
      <h1>Checkout</h1>
      <ul>
        <li>Linen apron, 2 at $24.00</li>
        <li>Enamel mug, $36.00</li>
      </ul>
      <div className="armada-mock-checkout__code">
        <Input label="Discount code" defaultValue={code} onChange={(event) => void (code = event.target.value)} />
        <Button variant="secondary" size="sm">
          Apply
        </Button>
      </div>
      <p>SPRING10 applied, 10% off</p>
      <p>Subtotal $84.00</p>
      <p>Discount $8.40</p>
      <p className="armada-mock-checkout__total">Total $84.00</p>
      <Button variant="primary">Place order</Button>
      {sessionId === undefined ? null : (
        <div className="armada-mock-checkout__asks">
          <Button variant="ghost" size="sm" onClick={() => requestPermission(sessionId, "local_network")}>
            Connect Okta Verify
          </Button>
          <Button variant="ghost" size="sm" onClick={() => requestPermission(sessionId, "location")}>
            Use my location
          </Button>
        </div>
      )}
    </article>
  );
}

export function CartPage() {
  return (
    <article className="armada-mock-checkout" aria-label="Fernwood Supply cart">
      <h1>Cart</h1>
      <ul>
        <li>Linen apron, 2 at $24.00</li>
        <li>Enamel mug, $36.00</li>
      </ul>
      <Button variant="primary">Check out</Button>
    </article>
  );
}

function NotFound({ url }: { url: string }) {
  return (
    <article className="armada-mock-checkout" aria-label="Fernwood Supply not found">
      <h1>Not found</h1>
      <p>{`Nothing at ${url}`}</p>
    </article>
  );
}

/** The page at an address on the shop's dev server. */
export function shopPage(url: string, sessionId?: string) {
  const path = url.replace(/^https?:\/\/[^/]+/, "");
  return path === "/checkout" ? <CheckoutPage {...(sessionId === undefined ? {} : { sessionId })} /> : path === "/cart" ? <CartPage /> : <NotFound url={url} />;
}
