// The Cockpit as the app, built on one familiar pattern: a workspace tab bar. Places are pinned tabs; a Job or
// a Session opens as a tab of its own, and closing it returns to the tab it was reached from, as it was left.
// A throwaway prototype on proto/cockpit-three-directions.

import { button, inside, region, role, tab, walk } from "../walk";

const bar = role("tablist", "Open tabs");
const tabIn = (name: string | RegExp) => inside(bar, tab(name));
const title = region("Title line");
const all = role("tab", "All", { exact: true });
const row = (group: string, name: string) => inside(region(group), role("listitem", name));

const cockpitDirectionTabs = walk("cockpit-ship-tabs", [
  { look: tabIn(/^Cockpit/), say: "The ship at rest. One tab is raised and bold: where he is. The rest are quiet" },
  { hover: inside(title, role("img", /^Merge line/)), say: "One title line says what this is, and the roll call sits on it. A mark names why on hover" },
  { look: row("Needs you", "Shorten the reconnect wait"), say: "Amber is what he is owed. The one red row is the Job whose Check is failing now" },
  { key: "3", on: bar, say: "3 jumps to Sessions: the real page" },
  { press: all, say: "Its own filter, set to All" },
  { press: button(/Cache key/), say: "The Session that replied opens as a tab" },
  { look: tabIn(/Cache key/), say: "Its own tab, to the right, with its state mark" },
  { type: "Hash. It survives a move.\n", into: role("textbox"), say: "Answered in the thread" },
  { key: "x", on: bar, say: "Sent where the page sends it. x closes the tab" },
  { look: all, say: "Back on Sessions as he left it: the All filter is still on" },
  { key: "1", on: bar, say: "1 is the Cockpit" },
  { press: inside(row("Needs you", "Add the pause marker column"), button("Open")), say: "Open a Job from its row" },
  { look: tabIn(/Add the pause marker column/), say: "A Job is a tab too, with the real Job page under it" },
  { later: bar, say: "A Session replies while the Job is open" },
  { look: button(/Pin the store clock/), say: "Its call waits in a row above the page, not over it" },
  { key: "w", on: bar, say: "w opens it over this tab" },
  { type: "Good. Merge it.\n", into: inside(region(/^Session: Pin the store clock/), role("textbox", "Message")), say: "Answered" },
  { key: "Meta+w", on: tabIn(/Add the pause marker column/), say: "Back on the Job's tab. Command W closes it, back to the Cockpit" },
  { key: "5", on: bar, say: "5 is the Merge line: the real page, which has no Merge" },
  { press: inside(title, button("Merge #1890")), say: "A ready pull request is merged from the title line" },
  { key: "2", on: bar, say: "2 is Jobs" },
  { later: bar, say: "1890 lands, and main goes red" },
  { look: inside(bar, role("img", "Main is failing")), say: "The one red thing, on the Cockpit tab wherever he is" },
  { key: ".", on: bar, say: "The period goes to what needs him most: main" },
  { press: button("Fix main in armada"), say: "Fix" },
  { look: role("img", "Fixing main"), say: "Handed to a Job, so main is quiet while it is fixed" },
  { later: bar, say: "Its fix lands" },
  { press: inside(row("Stuck or held", "Service worker audit"), button("Stop")), say: "Stop the stuck Session" },
  { look: row("Done", "Service worker audit"), say: "It moves to Done. Main is green again: the only red left is the Job whose Check is failing" },
]);

export { cockpitDirectionTabs };
