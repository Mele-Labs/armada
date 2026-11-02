// The stories that already exist, read into an index the annotation layer's note
// card draws from: which file a component is in and which variants it has.
// Node, as `annotations-server.ts` is, and wired in `vite.mock.config.ts` alone.
//
//   GET /__armada/stories   the index, as JSON
//
// **Read, not run.** A story file is parsed with the TypeScript compiler API and
// never imported, so a story that throws, or one that needs a browser, still
// indexes. Only what a literal says is kept: `args: { variant: "ghost" }` is
// `{ variant: "ghost" }`, and a JSX child or a function is dropped.

import { existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, join, relative, resolve, sep } from "node:path";
import type { IncomingMessage, ServerResponse } from "node:http";
import ts from "typescript";
import type { Plugin } from "vite";

import { STORIES_PATH, type IndexedStory, type StoryArgs, type StoryIndex } from "../shared/annotations";
import { repositoryRoot } from "./annotations";

/** Where the components' stories are, from the repository root. */
const STORIES_DIR = ["packages", "components", "src"] as const;

const SUFFIX = ".stories.tsx";

function storyFiles(dir: string, found: string[] = []): string[] {
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    if (entry.name === "node_modules") continue;
    const path = join(dir, entry.name);
    if (entry.isDirectory()) storyFiles(path, found);
    else if (entry.name.endsWith(SUFFIX)) found.push(path);
  }
  return found.sort();
}

/** The expression with its `as`, `satisfies` and parentheses taken off. */
function bare(node: ts.Expression): ts.Expression {
  let at = node;
  while (ts.isAsExpression(at) || ts.isSatisfiesExpression(at) || ts.isParenthesizedExpression(at) || ts.isTypeAssertionExpression(at)) at = at.expression;
  return at;
}

function nameOf(name: ts.PropertyName): string | null {
  return ts.isIdentifier(name) || ts.isStringLiteral(name) ? name.text : null;
}

function literalOf(node: ts.Expression): string | number | boolean | null {
  const at = bare(node);
  if (ts.isStringLiteral(at) || ts.isNoSubstitutionTemplateLiteral(at)) return at.text;
  if (ts.isNumericLiteral(at)) return Number(at.text);
  if (ts.isPrefixUnaryExpression(at) && at.operator === ts.SyntaxKind.MinusToken && ts.isNumericLiteral(at.operand)) return -Number(at.operand.text);
  if (at.kind === ts.SyntaxKind.TrueKeyword) return true;
  if (at.kind === ts.SyntaxKind.FalseKeyword) return false;
  return null;
}

function propertyOf(object: ts.ObjectLiteralExpression, key: string): ts.Expression | null {
  for (const property of object.properties) {
    if (ts.isPropertyAssignment(property) && nameOf(property.name) === key) return property.initializer;
  }
  return null;
}

function argsOf(story: ts.ObjectLiteralExpression): StoryArgs {
  const args: StoryArgs = {};
  const written = propertyOf(story, "args");
  const object = written === null ? null : bare(written);
  if (object === null || !ts.isObjectLiteralExpression(object)) return args;
  for (const property of object.properties) {
    if (!ts.isPropertyAssignment(property)) continue;
    const key = nameOf(property.name);
    const value = literalOf(property.initializer);
    if (key !== null && value !== null) args[key] = value;
  }
  return args;
}

/** The file a relative import names, or null where it is a package or nothing is there. */
function resolveImport(from: string, specifier: string): string | null {
  if (!specifier.startsWith(".")) return null;
  const base = resolve(dirname(from), specifier);
  for (const candidate of [base, `${base}.tsx`, `${base}.ts`, join(base, "index.tsx"), join(base, "index.ts")]) {
    if (existsSync(candidate) && statSync(candidate).isFile()) return candidate;
  }
  return null;
}

/** The meta object the default export is: written in place, or a variable of this file. */
function metaOf(file: ts.SourceFile): ts.ObjectLiteralExpression | null {
  for (const statement of file.statements) {
    if (!ts.isExportAssignment(statement) || statement.isExportEquals === true) continue;
    const exported = bare(statement.expression);
    if (ts.isObjectLiteralExpression(exported)) return exported;
    if (!ts.isIdentifier(exported)) return null;
    for (const other of file.statements) {
      if (!ts.isVariableStatement(other)) continue;
      for (const declaration of other.declarationList.declarations) {
        if (!ts.isIdentifier(declaration.name) || declaration.name.text !== exported.text || declaration.initializer === undefined) continue;
        const initial = bare(declaration.initializer);
        return ts.isObjectLiteralExpression(initial) ? initial : null;
      }
    }
  }
  return null;
}

/** What the component identifier was imported as, and the file it came from. */
function importOf(file: ts.SourceFile, from: string, local: string): { name: string; file: string } | null {
  for (const statement of file.statements) {
    if (!ts.isImportDeclaration(statement) || !ts.isStringLiteral(statement.moduleSpecifier)) continue;
    const bindings = statement.importClause?.namedBindings;
    const resolved = (): string | null => resolveImport(from, (statement.moduleSpecifier as ts.StringLiteral).text);
    if (statement.importClause?.name?.text === local) {
      const path = resolved();
      return path === null ? null : { name: local, file: path };
    }
    if (bindings === undefined || !ts.isNamedImports(bindings)) continue;
    for (const element of bindings.elements) {
      if (element.name.text !== local) continue;
      const path = resolved();
      return path === null ? null : { name: (element.propertyName ?? element.name).text, file: path };
    }
  }
  return null;
}

/**
 * What one story file says: the component it is about and the stories it
 * exports. Null where it names no component this can find a file for. Throws
 * where the file cannot be read, which `storyIndex` reports and carries on past.
 */
export function readStoryFile(path: string, root: string, text: string = readFileSync(path, "utf8")): { name: string; file: string; title: string; stories: IndexedStory[] } | null {
  const source = ts.createSourceFile(path, text, ts.ScriptTarget.Latest, true, ts.ScriptKind.TSX);
  const meta = metaOf(source);
  if (meta === null) return null;
  const component = propertyOf(meta, "component");
  const identifier = component === null ? null : bare(component);
  if (identifier === null || !ts.isIdentifier(identifier)) return null;
  const imported = importOf(source, path, identifier.text);
  if (imported === null) return null;
  const titled = propertyOf(meta, "title");
  const title = titled === null ? null : literalOf(titled);
  const stories: IndexedStory[] = [];
  const storyFile = relative(root, path).split(sep).join("/");
  for (const statement of source.statements) {
    if (!ts.isVariableStatement(statement) || !statement.modifiers?.some((m) => m.kind === ts.SyntaxKind.ExportKeyword)) continue;
    for (const declaration of statement.declarationList.declarations) {
      if (!ts.isIdentifier(declaration.name) || declaration.initializer === undefined) continue;
      const story = bare(declaration.initializer);
      if (ts.isObjectLiteralExpression(story)) stories.push({ name: declaration.name.text, args: argsOf(story), file: storyFile });
    }
  }
  return { name: imported.name, file: relative(root, imported.file).split(sep).join("/"), title: typeof title === "string" ? title : imported.name, stories };
}

/** Each file's reading, kept by its modification time so a request re-parses only what changed. */
export type StoryCache = Map<string, { mtime: number; read: ReturnType<typeof readStoryFile> }>;

/**
 * The index of every `*.stories.tsx` under `packages/components/src`. A file that cannot
 * be read costs one `warn` line and nothing else; two files about one component
 * merge, the first one's title winning.
 */
export function storyIndex(root: string, warn: (line: string) => void = () => {}, cache: StoryCache = new Map()): StoryIndex {
  const dir = join(root, ...STORIES_DIR);
  const index: StoryIndex = {};
  if (!existsSync(dir)) return index;
  for (const path of storyFiles(dir)) {
    const short = relative(root, path).split(sep).join("/");
    let read: ReturnType<typeof readStoryFile>;
    try {
      const mtime = statSync(path).mtimeMs;
      const kept = cache.get(path);
      if (kept !== undefined && kept.mtime === mtime) read = kept.read;
      else {
        read = readStoryFile(path, root);
        cache.set(path, { mtime, read });
      }
    } catch (error) {
      warn(`armada-stories: cannot read ${short}: ${error instanceof Error ? error.message : String(error)}`);
      continue;
    }
    if (read === null) continue;
    const entry = (index[read.name] ??= { file: read.file, title: read.title, stories: [] });
    for (const story of read.stories) {
      if (!entry.stories.some((known) => known.name === story.name)) entry.stories.push(story);
    }
  }
  return index;
}

export type AnnotationStoriesOptions = {
  /** Where to look for the repository from. Defaults to the dev server's root. */
  from?: string;
};

export function annotationStories(options: AnnotationStoriesOptions = {}): Plugin {
  return {
    name: "armada-annotation-stories",
    apply: "serve",
    configureServer(server) {
      const root = repositoryRoot(options.from ?? server.config.root);
      if (root === null) {
        server.config.logger.warn(`armada-stories: no repository above ${server.config.root}`);
        return;
      }
      const cache: StoryCache = new Map();
      // Warned once per file, not once per request.
      const warned = new Set<string>();
      const warn = (line: string): void => {
        if (warned.has(line)) return;
        warned.add(line);
        server.config.logger.warn(line);
      };
      server.middlewares.use(STORIES_PATH, (req: IncomingMessage, res: ServerResponse) => {
        if (req.method !== "GET") {
          res.statusCode = 405;
          res.end();
          return;
        }
        try {
          const body = JSON.stringify(storyIndex(root, warn, cache));
          res.statusCode = 200;
          res.setHeader("Content-Type", "application/json");
          res.end(body);
        } catch (error) {
          res.statusCode = 500;
          res.setHeader("Content-Type", "application/json");
          res.end(JSON.stringify({ error: error instanceof Error ? error.message : String(error) }));
        }
      });
    },
  };
}
