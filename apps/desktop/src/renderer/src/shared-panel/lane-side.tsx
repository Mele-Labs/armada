// A pick on the Workflows page's lanes, in the side panel. **The page draws it**: the lanes are a component and the panel is the window's, so this holds only the shell and
// hands its body to the page as a slot (`useLanePanel`).

import { SidePanel } from "./side";
import { useBack } from "./panels";
import { closePanel, holdLaneSlot } from "./store";
import type { PanelItem } from "./store";

export function LaneSide({ item }: { item: Extract<PanelItem, { kind: "lane" }> }) {
  const back = useBack();
  return (
    <SidePanel title={item.title} heading={item.heading} {...(item.crumb === "" ? {} : { crumb: item.crumb })} back={back} onClose={closePanel}>
      <div ref={holdLaneSlot} />
    </SidePanel>
  );
}
