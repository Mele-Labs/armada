// The film's staging checkout: the pages Flight 1's Job browser shows while the Job drives it, cart, card details, paying and the
// order confirmation. A web page and not a Bridge surface, so it is drawn plain from the primitives, as `job-browser-page.tsx` is.
// Any other address is the shop's own not found page. Mock only.

import { Button, Input } from "@armada/components";

const STAGING = "https://staging.relay.test";
export const STAGING_CART = `${STAGING}/cart`;
export const STAGING_CARD = `${STAGING}/checkout/card`;
export const STAGING_PAY = `${STAGING}/checkout/pay`;
export const STAGING_DONE = `${STAGING}/checkout/confirmed`;

const ITEMS = (
  <ul>
    <li>Relay Team plan, 5 seats at $12.00</li>
    <li>Priority support, $24.00</li>
  </ul>
);

function Cart() {
  return (
    <article className="armada-mock-checkout" aria-label="Relay staging cart">
      <h1>Cart</h1>
      {ITEMS}
      <p className="armada-mock-checkout__total">Total $84.00</p>
      <Button variant="primary">Check out</Button>
    </article>
  );
}

function Card() {
  return (
    <article className="armada-mock-checkout" aria-label="Relay staging card details">
      <h1>Card details</h1>
      {ITEMS}
      <Input label="Card number" defaultValue="4242 4242 4242 4242" readOnly />
      <Input label="Expiry" defaultValue="12 / 29" readOnly />
      <p className="armada-mock-checkout__total">Total $84.00</p>
      <Button variant="primary">Pay $84.00</Button>
    </article>
  );
}

function Paying() {
  return (
    <article className="armada-mock-checkout" aria-label="Relay staging payment">
      <h1>Paying</h1>
      <p>Charging the card ending 4242</p>
      <p className="armada-mock-checkout__total">Total $84.00</p>
    </article>
  );
}

function Done() {
  return (
    <article className="armada-mock-checkout" aria-label="Relay staging order confirmed">
      <h1>Order confirmed</h1>
      <p>Order 10482</p>
      {ITEMS}
      <p className="armada-mock-checkout__total">Paid $84.00</p>
    </article>
  );
}

/** The page at an address on the staging site. */
export function stagingPage(url: string) {
  switch (url) {
    case STAGING_CART:
      return <Cart />;
    case STAGING_CARD:
      return <Card />;
    case STAGING_PAY:
      return <Paying />;
    case STAGING_DONE:
      return <Done />;
    default:
      return (
        <article className="armada-mock-checkout" aria-label="Relay staging not found">
          <h1>Not found</h1>
          <p>{`Nothing at ${url}`}</p>
        </article>
      );
  }
}
