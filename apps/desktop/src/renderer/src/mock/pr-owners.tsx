// The owners registered to pull requests, as the mock holds them: a scenario's rows, and the switch
// kept for each repository (`draft.prOwners`). Fleet serves none of it yet. A repository whose switch
// is off registers nothing, so its pull requests draw no chip.

import { useEffect, useMemo, useState } from "react";
import type { ReactNode } from "react";
import { PrOwnerRegistryProvider } from "@armada/components";
import type { PrOwnerRegistry } from "@armada/components";

import type { MockDraft } from "./moment";

function Held({ owners, children }: { owners: NonNullable<MockDraft["prOwners"]>; children: ReactNode }) {
  const [registering, setRegistering] = useState<Readonly<Record<string, boolean>>>(() =>
    Object.fromEntries(owners.repositories.map((one) => [one.root, one.registering])),
  );
  // The switch is the open Manifest's, so it follows the repository the window has picked.
  const [picked, setPicked] = useState<string | null>(null);
  useEffect(() => {
    void window.armada.state().then((state) => setPicked(state.repository));
    return window.armada.subscribe((state) => setPicked(state.repository));
  }, []);

  const value = useMemo<PrOwnerRegistry>(
    () => ({
      ownerOf: (repository, number) => {
        const served = owners.repositories.find((one) => one.name === repository);
        if (served === undefined || registering[served.root] !== true) return undefined;
        return owners.rows.find((one) => one.repository === repository && one.number === number)?.owner;
      },
      registering: picked !== null && registering[picked] === true,
      setRegistering: (on) => {
        if (picked !== null) setRegistering((was) => ({ ...was, [picked]: on }));
      },
    }),
    [owners, registering, picked],
  );
  return <PrOwnerRegistryProvider value={value}>{children}</PrOwnerRegistryProvider>;
}

/** Hands the window the registry a scenario carries. Nothing carried, nothing is drawn. */
export function PrOwnersFrom({ owners, children }: { owners: MockDraft["prOwners"]; children: ReactNode }) {
  return owners === undefined ? <>{children}</> : <Held owners={owners}>{children}</Held>;
}
