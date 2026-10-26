// A screen opened on a shell redraws everything the shell has printed. The queries in that history, such as a
// background colour asked for and a cursor position, were answered when they were printed; answering them
// again types the replies into the shell, one more batch each time the Job is opened.

import { XtermScreen } from "@armada/components";
import { createRoot } from "react-dom/client";
import { expect, test } from "vitest";

import "./styles/index.css";

const ASK_BACKGROUND = "\u001b]11;?\u0007";
const ASK_CURSOR = "\u001b[6n";

const settle = () => new Promise((done) => setTimeout(done, 100));

test("a screen redrawing history answers none of its queries, and answers a new one once", async () => {
  const host = document.createElement("div");
  host.style.cssText = "width: 640px; height: 240px; display: flex";
  document.body.append(host);
  const sent: string[] = [];
  const write = (data: string) => void sent.push(data);
  const history = `$ bk configure\r\n${ASK_BACKGROUND}${ASK_CURSOR}Organization slug: `;
  const root = createRoot(host);

  root.render(<XtermScreen out={history} run={1} running write={write} onSize={() => undefined} />);
  await settle();
  expect(sent).toEqual([]);

  root.render(<XtermScreen out={history + ASK_CURSOR} run={1} running write={write} onSize={() => undefined} />);
  await settle();
  expect(sent).toHaveLength(1);
  expect(sent[0]).toMatch(/^\u001b\[\d+;\d+R$/);
  root.unmount();
  host.remove();
});
