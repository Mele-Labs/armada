// A pull request named by a link, read the same way wherever Bridge meets one: pasted in Reviews, or handed to the app as
// `armada://pull/<owner>/<name>/<number>`. Pure, since the main process and the renderer both read it.

export const DEEP_LINK_SCHEME = "armada";

export type PullLink = { owner: string; name: string; number: number };

const number = (text: string | undefined): number | undefined => {
  if (text === undefined || !/^[0-9]+$/.test(text)) return undefined;
  const parsed = Number(text);
  return Number.isSafeInteger(parsed) && parsed > 0 ? parsed : undefined;
};

/** The pull request a link names, or nothing where it names none. The forge's own address and the deep link read the same. */
export function parsePullLink(text: string): PullLink | undefined {
  let url: URL;
  try {
    url = new URL(text.trim());
  } catch {
    return undefined;
  }
  // The deep link's host is `pull`, so its path is `/<owner>/<name>/<number>`; the forge's is `/<owner>/<name>/pull/<number>/...`.
  const parts = url.pathname.split("/").filter((part) => part !== "");
  const [owner, name, at] = url.protocol === `${DEEP_LINK_SCHEME}:` ? (url.hostname === "pull" ? parts : []) : url.protocol === "https:" || url.protocol === "http:" ? (parts[2] === "pull" ? [parts[0], parts[1], parts[3]] : []) : [];
  const found = number(at);
  return owner === undefined || name === undefined || found === undefined ? undefined : { owner, name, number: found };
}

/** The deep link that opens a pull request in Bridge. */
export const deepLinkOf = (link: PullLink): string => `${DEEP_LINK_SCHEME}://pull/${link.owner}/${link.name}/${link.number}`;
