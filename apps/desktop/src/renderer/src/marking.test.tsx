import { useMarking, type Mark } from "@armada/components";
import { createRoot } from "react-dom/client";
import { act } from "react";
import { expect, test } from "vitest";

const MARKS: readonly Mark[] = [{ of: ".outer", as: "data-in" }];

function Inner() {
  const ref = useMarking(MARKS);
  return <i ref={ref} />;
}

function mount() {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  const draw = (n: number) => act(() => root.render(<div className="outer">{Array.from({ length: n }, (_, i) => <Inner key={i} />)}</div>));
  return { host, draw, root };
}

test("an ancestor is marked while something that asked for it is there, and loses the mark with the last of them", async () => {
  (globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;
  const { host, draw, root } = mount();
  await draw(0);
  expect(host.querySelector(".outer")!.hasAttribute("data-in")).toBe(false);
  await draw(2);
  expect(host.querySelector(".outer")!.hasAttribute("data-in")).toBe(true);
  await draw(1);
  expect(host.querySelector(".outer")!.hasAttribute("data-in")).toBe(true);
  await draw(0);
  expect(host.querySelector(".outer")!.hasAttribute("data-in")).toBe(false);
  await act(() => root.unmount());
  host.remove();
});
