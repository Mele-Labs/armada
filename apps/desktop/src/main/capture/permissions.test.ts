// Which Job browser page is given local network access: only on its own origin, only while the owner drives.

import { describe, expect, it } from "vitest";

import { granted, LOCAL_NETWORK } from "./address";
import { whileOwnerDrives } from "./permissions";

const PAGE = "https://login.example.com/signin";

describe("a Job browser's local network access", () => {
  it("is given to the page's own origin while the owner drives", () => {
    const may = whileOwnerDrives(() => true, () => PAGE);
    expect(granted(LOCAL_NETWORK, "https://login.example.com/other", may)).toBe(true);
  });

  it("is refused while the agent drives", () => {
    const may = whileOwnerDrives(() => false, () => PAGE);
    expect(granted(LOCAL_NETWORK, PAGE, may)).toBe(false);
  });

  it("is refused to another origin", () => {
    const may = whileOwnerDrives(() => true, () => PAGE);
    expect(granted(LOCAL_NETWORK, "https://elsewhere.example.com/", may)).toBe(false);
    expect(granted(LOCAL_NETWORK, "http://login.example.com/", may)).toBe(false);
  });

  it("is refused to a page that has not loaded", () => {
    const may = whileOwnerDrives(() => true, () => "about:blank");
    expect(granted(LOCAL_NETWORK, "about:blank", may)).toBe(false);
    expect(granted(LOCAL_NETWORK, PAGE, whileOwnerDrives(() => true, () => ""))).toBe(false);
  });

  it("still refuses every other permission", () => {
    const may = whileOwnerDrives(() => true, () => PAGE);
    for (const permission of ["media", "notifications", "clipboard-read", "geolocation"]) expect(granted(permission, PAGE, may)).toBe(false);
  });
});
