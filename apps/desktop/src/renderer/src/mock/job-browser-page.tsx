// The page the mock's Job browser shows: an invented shop's checkout, where the discount is applied and the total does not take it.
// A web page and not a Bridge surface, so it is drawn plain, from the primitives, the way a page in the pane would be.

import { Button, Input } from "@armada/components";

/** What is in the code's field, kept beside the page so the page in a window is the page that was docked. */
let code = "SPRING10";

export function resetCheckout(): void {
  code = "SPRING10";
}

export function CheckoutPage() {
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
    </article>
  );
}
