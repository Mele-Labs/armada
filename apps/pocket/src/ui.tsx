// The pieces every phone screen is made of: Bridge's panel header, a figure and a refusal's sentence.

import type { ReactNode } from "react";
import { X } from "lucide-react";
import { Button, Readout, Tooltip } from "@armada/components";

/** A header's icon press, named by its tooltip as Bridge's are. */
export function IconButton({
  label,
  onClick,
  children,
}: {
  label: string;
  onClick: () => void;
  children: ReactNode;
}) {
  return (
    <Tooltip label={label} asChild>
      <Button variant="ghost" size="sm" iconOnly aria-label={label} onClick={onClick}>
        {children}
      </Button>
    </Tooltip>
  );
}

/** Back, as Bridge's layers draw it: the close glyph at the header's trailing end. */
export const Back = ({ label, onClick }: { label: string; onClick: () => void }) => (
  <IconButton label={label} onClick={onClick}>
    <X size={16} aria-hidden="true" />
  </IconButton>
);

export function Band({
  title,
  onBack,
  back,
  end,
}: {
  title: string;
  onBack?: () => void;
  back?: string;
  end?: ReactNode;
}) {
  return (
    <header className="deck-band pk-band">
      <h1 className="pk-band__title">{title}</h1>
      {end}
      {onBack === undefined ? null : <Back label={back ?? "Back"} onClick={onBack} />}
    </header>
  );
}

/** A figure with a tooltip naming it, in a fixed column unless `loose`; an empty one keeps the column. */
export function Figure({
  name,
  loose = false,
  children,
}: {
  name: string;
  loose?: boolean;
  children?: string | undefined;
}) {
  if (children === undefined) return <Readout fixed={!loose} />;
  return (
    <Tooltip label={name}>
      <Readout fixed={!loose} aria-label={`${name} ${children}`}>
        {children}
      </Readout>
    </Tooltip>
  );
}

export const Trouble = ({ says }: { says: string }) =>
  says === "" ? null : (
    <p className="pk-fact" role="status">
      {says}
    </p>
  );
