// A story module drawn without Storybook: the desktop app cannot import `@storybook/react`, and the Variant tool
// only needs the story's element. Args merge meta then story, a render replaces the component, decorators wrap
// the story's own then the meta's, and `play` is never run.

import { Component, type ComponentType, type ReactElement, type ReactNode } from "react";

type Args = Record<string, unknown>;

/** What a decorator is told about the story it wraps. No story reads it in its render, so it is the decorators' alone. */
export type StoryContext = {
  args: Args;
  parameters: Record<string, unknown>;
  globals: Record<string, unknown>;
  name: string;
};

type Render = (args: Args, context: StoryContext) => ReactNode;
type Decorator = (Story: ComponentType, context: StoryContext) => ReactNode;
type Parts = {
  args?: Args;
  parameters?: Record<string, unknown>;
  render?: Render;
  decorators?: Decorator[];
};

/** A story file's exports: `default` is the meta, every other export a story. */
export type StoryModule = Record<string, unknown> & {
  default: Parts & {
    component?: ComponentType<Args>;
  };
};

/** A story that throws draws nothing, so a tile that cannot show it shows its name alone. */
export class StoryBoundary extends Component<
  {
    children: ReactNode;
  },
  {
    failed: boolean;
  }
> {
  override state = {
    failed: false,
  };
  static getDerivedStateFromError(): {
    failed: boolean;
  } {
    return {
      failed: true,
    };
  }
  override render(): ReactNode {
    return this.state.failed ? null : this.props.children;
  }
}

/** The element for story `name` of `module`, or null where the module has no such story or nothing to draw it with. */
export function storyElement(module: StoryModule, name: string): ReactElement | null {
  const meta = module.default;
  const story = module[name] as Parts | undefined;
  if (typeof story !== "object" || story === null) return null;
  const args = {
    ...meta.args,
    ...story.args,
  };
  const Component = meta.component;
  const render =
    story.render ??
    meta.render ??
    (Component === undefined ? undefined : (given: Args) => <Component {...given} />);
  if (render === undefined) return null;
  const context: StoryContext = {
    args,
    parameters: {
      ...meta.parameters,
      ...story.parameters,
    },
    globals: {},
    name,
  };
  // A function component, so a render that uses hooks runs inside one.
  let Drawn: ComponentType = function Drawn() {
    return render(args, context);
  };
  for (const decorator of [
    ...(story.decorators ?? []),
    ...(meta.decorators ?? []),
  ]) {
    const Inner: ComponentType = Drawn;
    Drawn = function Decorated(): ReactNode {
      return decorator(Inner, context);
    };
  }
  return <Drawn />;
}
