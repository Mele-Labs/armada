// What a pasted or dropped link is: a pull request, an issue, or anything else. Pure, built on `parsePullLink` so the two never
// disagree about what a pull request's address is.

import { parsePullLink, type PullLink } from "./pull-link";

export type LinkClass =
  | { kind: "pull"; link: PullLink }
  /** A forge issue carries its repository; a tracker's key does not, and is read where it is opened. */
  | { kind: "issue"; number: string; address: string; owner?: string; name?: string }
  | { kind: "other" };

const KEY = /^[A-Z][A-Z0-9]+-[0-9]+$/;

export function classifyLink(text: string): LinkClass {
  const pull = parsePullLink(text);
  if (pull !== undefined) return { kind: "pull", link: pull };
  let url: URL;
  try {
    url = new URL(text.trim());
  } catch {
    return { kind: "other" };
  }
  if (url.protocol !== "https:" && url.protocol !== "http:") return { kind: "other" };
  const parts = url.pathname.split("/").filter((part) => part !== "");
  const address = text.trim();
  if (parts.length >= 4 && parts[2] === "issues" && /^[0-9]+$/.test(parts[3]!) && Number(parts[3]) > 0) return { kind: "issue", number: parts[3]!, owner: parts[0]!, name: parts[1]!, address };
  if (parts[0] === "browse" && parts[1] !== undefined && KEY.test(parts[1])) return { kind: "issue", number: parts[1], address };
  return { kind: "other" };
}
