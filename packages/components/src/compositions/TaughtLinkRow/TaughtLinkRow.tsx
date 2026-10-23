import { Button } from "../../primitives/Button/Button";

/**
 * One link rule the owner taught, in Settings › Links: the words it was taught from, where they go, and where it applies.
 * **Forget is the one act**, and it takes the rule out of every Session at once.
 */
export type TaughtLinkRowProps = {
  /** The words the rule was taught from. */
  example: string;
  /** The address those words go to. */
  address: string;
  /** The repository's name, or `Every repository`. */
  reach: string;
  onForget: () => void;
};

export function TaughtLinkRow({ example, address, reach, onForget }: TaughtLinkRowProps) {
  return (
    <div className="armada-taught-link" role="group" aria-label={example}>
      <div className="armada-taught-link__rule">
        <p className="armada-taught-link__line mono">
          <span>{example}</span>
          <span className="armada-taught-link__address">{address}</span>
        </p>
        <p className="armada-taught-link__reach">{reach}</p>
      </div>
      <Button variant="secondary" size="sm" aria-label={`Forget ${example}`} onClick={onForget}>
        Forget
      </Button>
    </div>
  );
}
