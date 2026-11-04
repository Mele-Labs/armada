// The pieces the Usage page's views share: the mono band over a panel, a figure that explains itself, a panel's scope line,
// and a skeleton bar standing where a figure will land.

import { Kbd } from "../../primitives/Kbd/Kbd";
import { Skeleton } from "../../primitives/Skeleton/Skeleton";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";

export function Band({ children, keys }: { children: string; keys?: string[] }) {
  return (
    <div className="armada-usage__band">
      <span>{children}</span>
      {keys === undefined ? null : (
        <span className="armada-usage__keys">
          {keys.map((one) => (
            <Kbd key={one}>{one}</Kbd>
          ))}
        </span>
      )}
    </div>
  );
}

export function Fig({ tip, over, children }: { tip: string; over?: boolean; children: string }) {
  return (
    <Tooltip label={tip}>
      <span className="armada-usage__fig" data-over={over || undefined}>
        {children}
      </span>
    </Tooltip>
  );
}

export function Scope({ title, unit }: { title: string; unit?: string }) {
  return (
    <div className="armada-usage__scope">
      <span className="armada-usage__title">{title}</span>
      {unit === undefined ? null : <span className="armada-usage__unit">{unit}</span>}
    </div>
  );
}

export function Bar({ width, end }: { width: string; end?: boolean }) {
  return (
    <span className="armada-usage__skel" data-end={end || undefined}>
      <Skeleton width={width} />
    </span>
  );
}
