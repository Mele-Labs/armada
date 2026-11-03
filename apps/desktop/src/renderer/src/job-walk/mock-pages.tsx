// The pages the mock's walks show, drawn as plain DOM in the walk's page area, since the mock has no native page view. **Mock only**: the real wiring lays Bridge's page over the same
// area and rings its own target. Each target the steps point at is named by `data-walk-target`.

import type { ReactNode } from "react";

const INVOICES = [
  {
    n: "1042",
    on: "Oct 28",
    amount: "$48.00",
    state: "Paid",
  },
  {
    n: "1038",
    on: "Sep 28",
    amount: "$48.00",
    state: "Paid",
  },
  {
    n: "1031",
    on: "Aug 28",
    amount: "$48.00",
    state: "Paid",
  },
  {
    n: "1025",
    on: "Jul 28",
    amount: "$36.00",
    state: "Refunded",
  },
  {
    n: "1019",
    on: "Jun 28",
    amount: "$36.00",
    state: "Paid",
  },
];

/** Set on the first row only, which is where the steps point. */
const aim = (first: boolean, target: string) =>
  first
    ? {
        "data-walk-target": target,
      }
    : {};

function BillingPage({ take }: { take: string }) {
  const table = take === "With table";
  const rows = take === "Roomy" ? INVOICES.slice(0, 3) : INVOICES;
  return (
    <div className="job-walk-mock" data-density={take === "Roomy" ? "roomy" : "compact"}>
      <h2 className="job-walk-mock__title">Billing</h2>
      <section className="job-walk-mock__card" data-walk-target="plan" aria-label="Plan">
        <div>
          <p className="job-walk-mock__strong">Team plan</p>
          <p className="job-walk-mock__muted">5 seats, renews Nov 28</p>
        </div>
        <p className="job-walk-mock__strong">$48 a month</p>
      </section>
      <section aria-label="Invoices" data-walk-target="invoices">
        <div className="job-walk-mock__head">
          <h3 className="job-walk-mock__sub">Invoices</h3>
          <button type="button" className="job-walk-mock__button" data-walk-target="export">
            Export all
          </button>
        </div>
        {table ? (
          <table className="job-walk-mock__table">
            <thead>
              <tr>
                <th>Invoice</th>
                <th>Date</th>
                <th>Amount</th>
                <th>State</th>
                <th />
              </tr>
            </thead>
            <tbody>
              {rows.map((one, at) => (
                <tr key={one.n}>
                  <td>{one.n}</td>
                  <td>{one.on}</td>
                  <td>{one.amount}</td>
                  <td {...aim(at === 0, "status")}>{one.state}</td>
                  <td>
                    <button
                      type="button"
                      className="job-walk-mock__link"
                      {...aim(at === 0, "receipt")}
                    >
                      Receipt
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        ) : (
          <ul className="job-walk-mock__list">
            {rows.map((one, at) => (
              <li key={one.n} className="job-walk-mock__item">
                <span className="job-walk-mock__strong">{`Invoice ${one.n}`}</span>
                <span className="job-walk-mock__muted">{one.on}</span>
                <span>{one.amount}</span>
                <span {...aim(at === 0, "status")}>{one.state}</span>
                <button
                  type="button"
                  className="job-walk-mock__link"
                  {...aim(at === 0, "receipt")}
                >
                  Receipt
                </button>
              </li>
            ))}
          </ul>
        )}
      </section>
    </div>
  );
}

function SettingsPage() {
  return (
    <div className="job-walk-mock" data-density="roomy">
      <h2 className="job-walk-mock__title">Settings</h2>
      <label className="job-walk-mock__field" data-walk-target="name">
        <span className="job-walk-mock__muted">Display name</span>
        <input className="job-walk-mock__input" defaultValue="Ana Ruiz" readOnly />
      </label>
      <label className="job-walk-mock__check" data-walk-target="notifications">
        <input type="checkbox" defaultChecked readOnly />
        <span>Email me when a walk is ready</span>
      </label>
      <div>
        <button type="button" className="job-walk-mock__button" data-walk-target="save">
          Save
        </button>
      </div>
    </div>
  );
}

function ClockPage() {
  const tests = [
    "reads_the_pinned_clock",
    "expires_after_thirty_seconds",
    "orders_by_the_store_clock",
    "keeps_the_fixture_out",
  ];
  return (
    <div className="job-walk-mock" data-density="compact">
      <h2 className="job-walk-mock__title">Clock report</h2>
      <section className="job-walk-mock__card" data-walk-target="clock" aria-label="Pinned clock">
        <p className="job-walk-mock__strong">Pinned at 2026-11-03 14:00:00</p>
      </section>
      <ul className="job-walk-mock__list" data-walk-target="tests">
        {tests.map((one) => (
          <li key={one} className="job-walk-mock__item">
            <span className="job-walk-mock__strong">{one}</span>
            <span>Passed</span>
          </li>
        ))}
      </ul>
    </div>
  );
}

/** The page a walk shows, by the page's name, in the take on screen. */
export function mockPage(title: string, take: string | undefined): ReactNode {
  switch (title) {
    case "Billing":
      return <BillingPage take={take ?? "Compact"} />;
    case "Settings":
      return <SettingsPage />;
    default:
      return <ClockPage />;
  }
}
