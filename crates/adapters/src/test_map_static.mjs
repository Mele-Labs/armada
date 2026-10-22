// The import closure of each test file under the working directory, one JSON
// line per test: {"test": "<abs path>", "files": ["<abs path>", ...]}.
//
// Run as `node --input-type=module -e <this> -- <repository root> [<test file>...]`
// in a Check's directory. With no test file named it finds them the way vitest
// does by default; with some named it reads those and nothing else.
//
// It prints absolute, real paths and leaves the repository's own judgement to
// the Rust side. The one thing it will not do is walk into node_modules or out
// of the repository, which would read the whole dependency tree.
import { createRequire } from "node:module";
import fs from "node:fs";
import path from "node:path";

const [root, ...named] = process.argv.slice(1);
const cwd = process.cwd();

let ts;
try {
  ts = createRequire(path.join(cwd, "noop.js"))("typescript");
} catch {
  console.error(`no typescript package resolves from ${cwd}`);
  process.exit(2);
}

const TEST_FILE = /\.(test|spec)\.[cm]?[jt]sx?$/;
const SKIPPED_DIRS = new Set(["node_modules", "dist", "cypress", ".git", ".idea", ".cache", ".output", ".temp"]);
const SOURCE_FILE = /\.([cm]?[jt]sx?)$/;

function discover(dir, found) {
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    if (entry.isDirectory()) {
      if (!SKIPPED_DIRS.has(entry.name)) discover(path.join(dir, entry.name), found);
    } else if (TEST_FILE.test(entry.name)) {
      found.push(fs.realpathSync(path.join(dir, entry.name)));
    }
  }
  return found;
}

const tests = (named.length
  ? named.filter((file) => fs.existsSync(file)).map((file) => fs.realpathSync(file))
  : discover(cwd, [])
).sort();

// Compiler options by config file, and the options a source file is read with.
// A solution-style tsconfig owns no files and points at the projects that do,
// so the one whose file list holds the source is the one that applies.
const parsed = new Map();
function parse(configPath) {
  if (!parsed.has(configPath)) {
    const read = ts.readConfigFile(configPath, ts.sys.readFile);
    parsed.set(
      configPath,
      read.error ? undefined : ts.parseJsonConfigFileContent(read.config, ts.sys, path.dirname(configPath), undefined, configPath),
    );
  }
  return parsed.get(configPath);
}

function optionsFor(file) {
  const found = ts.findConfigFile(path.dirname(file), ts.sys.fileExists);
  const nearest = found && parse(found);
  if (!nearest) return {};
  if (!nearest.fileNames.includes(file)) {
    for (const reference of nearest.projectReferences ?? []) {
      const target = ts.sys.directoryExists(reference.path) ? path.join(reference.path, "tsconfig.json") : reference.path;
      const project = parse(target);
      if (project?.fileNames.includes(file)) return project.options;
    }
  }
  return nearest.options;
}

const caches = new Map();
function resolve(specifier, from, options) {
  const key = JSON.stringify(options.paths ?? null) + (options.baseUrl ?? "");
  if (!caches.has(key)) caches.set(key, ts.createModuleResolutionCache(cwd, (name) => name, options));
  const hit = ts.resolveModuleName(specifier, from, options, ts.sys, caches.get(key)).resolvedModule;
  return hit && fs.realpathSync(hit.resolvedFileName);
}

const inside = (file) => file.startsWith(root + path.sep) && !file.split(path.sep).includes("node_modules");

function closure(test) {
  const options = optionsFor(test);
  const seen = new Set([test]);
  const queue = [test];
  while (queue.length) {
    const file = queue.pop();
    if (!SOURCE_FILE.test(file)) continue;
    const imports = ts.preProcessFile(fs.readFileSync(file, "utf8"), true, true).importedFiles;
    for (const { fileName } of imports) {
      const target = resolve(fileName, file, options);
      if (!target || seen.has(target)) continue;
      seen.add(target);
      if (inside(target)) queue.push(target);
    }
  }
  return [...seen].sort();
}

for (const test of tests) {
  console.log(JSON.stringify({ test, files: closure(test) }));
}
