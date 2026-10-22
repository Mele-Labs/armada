// Requests with no Job open, over three repositories with the Cockpit narrowed to storefront. storefront and billing keep
// collections and environments of their own; handbook's own requests file would not read, so only what the repository
// commits is listed. The walk `requests-repository-picker` plays it. Every host is `example.test`, and every name is invented.

import type { RepositorySummary } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/repository";

import { onBoard } from "../moment";
import type { Scenario } from "../moment";
import type { RequestsSeed } from "../slices/requests";

const served = (name: string): RepositorySummary => ({
  root: `/Users/user/code/${name}`,
  records_root: `/records/${name}`,
  manifest: { ...repository().manifest!, id: name, repository: name, path: `${name}/armada.yml` },
});

const STOREFRONT = served("storefront");
const BILLING = served("billing");
const HANDBOOK = served("handbook");

const requests: RequestsSeed = {
  repositories: {
    [STOREFRONT.root]: {
      yours: {
        collections: [
          {
            id: "store-api",
            name: "Store API",
            headers: [{ name: "Authorization", value: "Bearer {{token}}" }],
            requests: [{ id: "list-products", name: "List products", method: "GET", url: "{{base}}/products", headers: [] }],
          },
        ],
      },
      repository: {
        collections: [{ id: "health", name: "Health", headers: [], requests: [{ id: "health", name: "Health", method: "GET", url: "{{base}}/health", headers: [] }] }],
        environments: [{ name: "local", variables: [{ name: "base", secret: false, value: "https://store.local.example.test" }] }],
      },
    },
    [BILLING.root]: {
      yours: {
        collections: [{ id: "invoices", name: "Invoices", headers: [], requests: [{ id: "latest-invoice", name: "Latest invoice", method: "GET", url: "{{base}}/invoices/latest", headers: [] }] }],
        environments: [
          {
            name: "staging",
            variables: [
              { name: "base", secret: false, value: "https://billing.staging.example.test" },
              { name: "token", secret: true, set: true },
            ],
          },
        ],
      },
    },
    [HANDBOOK.root]: {
      repository: {
        collections: [{ id: "search", name: "Search", headers: [], requests: [{ id: "search-pages", name: "Search pages", method: "GET", url: "{{base}}/search?q=deploy", headers: [] }] }],
      },
      notRead: [{ place: "yours", why: "/Users/user/.armada/requests/handbook.toml: TOML parse error at line 3, column 7: expected `=`" }],
    },
  },
};

const board = onBoard({ repositories: [STOREFRONT, BILLING, HANDBOOK], picked: STOREFRONT.root });

export const s313RequestsByRepository: Scenario = {
  ...board,
  name: "requests-by-repository",
  says: "Requests with no Job open over three repositories, the Cockpit narrowed to storefront, and handbook's own requests file not reading",
  draft: { ...board.draft, requests },
};
