import { useId, useState, type FormEvent } from "react";
import { useKept } from "../../keep";
import { Button } from "../../primitives/Button/Button";
import { Input } from "../../primitives/Input/Input";
import { Select } from "../../primitives/Select/Select";

// Kit's MCP servers as `../KitBrowser` draws them: the add row, the words for the Here control, and a row's shape. #1275.
//
// **Two tiers.** Kit holds the default across every repository; this repository's manifest extends or restricts
// it, and saying nothing is a third state rather than an absent second. **Adding a server turns nothing on**: a new
// row reaches no drone, which is the confinement `docs/scope.md` keeps.

/** A program fleet starts, or an address a drone opens. */
export type KitServerKind = "stdio" | "http";

/** One server, with both tiers and the answer they come to. */
export type KitServerRowProps = {
  name: string;
  /** The program and its arguments, or the URL — one line, as it was given. */
  address: string;
  kind: KitServerKind;
  /** Kit's default, across every repository. */
  reachesByDefault: boolean;
  /** This repository's word. Absent where it has none, and kit's default answers. */
  here?: "extended" | "restricted";
  /** Whether a drone dispatched here gets it. Fleet's own answer, never worked out here. */
  resolves: boolean;
};

const KIND_LABEL: Record<KitServerKind, string> = {
  stdio: "Program",
  http: "Address",
};

export const KIT_HERE_LABEL: Record<"kit" | "extended" | "restricted", string> = {
  kit: "Follow Kit",
  extended: "Allowed here",
  restricted: "Withheld here",
};

/** The add row: a name, a kind and an address. Adding turns nothing on. */
export function KitServerAddForm({ onAdd, keep }: {
  onAdd?: ((adding: { name: string; kind: KitServerKind; address: string }) => void) | undefined;
  /** What the fields are kept under, so a half-typed server survives leaving the panel. Absent keeps nothing. */
  keep?: string;
}) {
  const group = useId();
  const [name, setName, clearName] = useKept(keep === undefined ? undefined : `${keep}:name`, "");
  const [kind, setKind] = useState<KitServerKind>("stdio");
  const [address, setAddress, clearAddress] = useKept(keep === undefined ? undefined : `${keep}:address`, "");

  function add(event: FormEvent): void {
    event.preventDefault();
    if (name.trim() === "" || address.trim() === "") return;
    onAdd?.({ name: name.trim(), kind, address: address.trim() });
    clearName();
    clearAddress();
  }

  return (
    <form className="armada-kit-servers__add" onSubmit={add}>
      <Input
        id={`${group}-name`}
        label="Name"
        value={name}
        placeholder="tracker"
        onChange={(event) => setName(event.target.value)}
      />
      <Select
        id={`${group}-kind`}
        label="Kind"
        value={kind}
        onChange={(event) => setKind(event.target.value as KitServerKind)}
      >
        <option value="stdio">Program</option>
        <option value="http">Address</option>
      </Select>
      <Input
        id={`${group}-address`}
        label={KIND_LABEL[kind]}
        value={address}
        placeholder={kind === "stdio" ? "npx -y @scope/server" : "https://example.com/mcp"}
        onChange={(event) => setAddress(event.target.value)}
      />
      <Button type="submit" variant="secondary" size="sm">
        Add
      </Button>
    </form>
  );
}
