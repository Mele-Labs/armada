// Fixture data for the screens that have no Gateway route yet: sessions (#2004) and
// dispatch (#2006). Jobs come from the Gateway, through `./data`.

export type SessionAsk = { id: string; title: string; repository: string; age: string; question: string; options: string[] };

export const HOSTED_ASK: SessionAsk = { id: "s-71", title: "Trim the pool", repository: "armada", age: "12m", question: "Close the idle bays now, or when their Jobs end?", options: ["Now", "When their Jobs end"] };

export const TERMINAL_WAITING = { id: "s-68", title: "Terminal in armada", repository: "armada", age: "27m" };

export const REPOSITORIES = ["armada", "ledger", "docs-site"];
