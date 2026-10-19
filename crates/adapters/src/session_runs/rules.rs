//! What a command is for, by plain rules on its words.
//!
//! No command is run and nothing is learned from a repository: a script is judged by its name and a
//! tool by its own. A command no rule names is nobody's business here and is left out.

use super::RunKind;

/// What a command was recognised as.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Verdict {
    pub kind: RunKind,
    /// A server whose name does not promise one (`start`, `serve`, `cargo run`) counts only if it
    /// stayed running or printed where it listens.
    pub needs_proof: bool,
}

const fn is(kind: RunKind) -> Option<Verdict> {
    Some(Verdict { kind, needs_proof: false })
}

const fn server_if_proven() -> Option<Verdict> {
    Some(Verdict { kind: RunKind::DevServer, needs_proof: true })
}

/// A command's words, after wrappers, judged.
pub(super) fn classify(words: &[String]) -> Option<Verdict> {
    let words = unwrapped(words);
    let first = base(words.first()?);
    let rest: Vec<&str> = words[1..].iter().map(String::as_str).collect();
    match first {
        "npm" | "pnpm" | "yarn" | "bun" | "bunx" | "turbo" | "nx" | "lerna" | "deno" => runner(first, &rest),
        "cargo" => cargo(&rest),
        "go" => go(&rest),
        "make" | "gmake" => make(&rest),
        "vitest" | "jest" | "mocha" | "pytest" | "py.test" | "rspec" | "phpunit" | "ava" | "tox" | "nosetests" => is(RunKind::Tests),
        "playwright" | "cypress" => rest.first().filter(|sub| matches!(**sub, "test" | "run")).and(is(RunKind::Tests)),
        "eslint" | "ruff" | "flake8" | "pylint" | "rubocop" | "golangci-lint" | "stylelint" | "oxlint" | "shellcheck" | "clippy-driver" => {
            match (first, rest.first().copied()) {
                ("ruff", Some("format")) => is(RunKind::Format),
                _ => is(RunKind::Lint),
            }
        }
        "biome" => match rest.first().copied() {
            Some("format") => is(RunKind::Format),
            Some("check" | "lint" | "ci") => is(RunKind::Lint),
            _ => None,
        },
        "prettier" | "black" | "gofmt" | "rustfmt" | "isort" | "swiftformat" | "dprint" => is(RunKind::Format),
        "tsc" => match rest.iter().any(|word| *word == "--noEmit") {
            true => is(RunKind::Typecheck),
            false => is(RunKind::Build),
        },
        "mypy" | "pyright" | "svelte-check" | "vue-tsc" | "tsgo" => is(RunKind::Typecheck),
        "vite" => match rest.first().copied() {
            Some("build") => is(RunKind::Build),
            Some("preview") => server_if_proven(),
            None | Some("dev" | "serve") => is(RunKind::DevServer),
            Some(flag) if flag.starts_with('-') => is(RunKind::DevServer),
            _ => None,
        },
        "next" | "nuxt" | "nuxi" | "astro" | "remix" | "ng" | "expo" | "gatsby" | "svelte-kit" | "webpack" | "parcel" | "rollup" => {
            framework(&rest)
        }
        "storybook" | "webpack-dev-server" | "browser-sync" | "wrangler" => {
            match rest.first().copied() {
                Some("build") => is(RunKind::Build),
                _ => is(RunKind::DevServer),
            }
        }
        "http-server" | "live-server" | "json-server" | "serve" | "uvicorn" | "gunicorn" | "hypercorn" | "flask" | "caddy" => server_if_proven(),
        "rails" => match rest.first().copied() {
            Some("s" | "server") => is(RunKind::DevServer),
            Some("test" | "t") => is(RunKind::Tests),
            _ => None,
        },
        "rake" => script(rest.first().copied().unwrap_or("")),
        "python" | "python3" | "py" => python(&rest),
        "swift" => match rest.first().copied() {
            Some("test") => is(RunKind::Tests),
            Some("build") => is(RunKind::Build),
            Some("run") => server_if_proven(),
            _ => None,
        },
        "dotnet" => match rest.first().copied() {
            Some("test") => is(RunKind::Tests),
            Some("build" | "publish") => is(RunKind::Build),
            Some("format") => is(RunKind::Format),
            Some("run" | "watch") => server_if_proven(),
            _ => None,
        },
        "gradle" | "gradlew" | "mvn" | "mvnw" => jvm(&rest),
        "mix" => match rest.first().copied() {
            Some("test") => is(RunKind::Tests),
            Some("compile") => is(RunKind::Build),
            Some("format") => is(RunKind::Format),
            Some("phx.server") => is(RunKind::DevServer),
            _ => None,
        },
        "php" => rest.first().filter(|flag| **flag == "-S").and(server_if_proven()),
        _ => None,
    }
}

/// A word without the folders in front of it: `./gradlew`, `node_modules/.bin/vitest`.
fn base(word: &str) -> &str {
    word.rsplit('/').next().unwrap_or(word)
}

/// The command under the wrappers that only start another: `time`, `timeout 60`, `npx -y`,
/// `uv run`, `bundle exec`.
fn unwrapped(words: &[String]) -> &[String] {
    let mut words = words;
    loop {
        let Some(first) = words.first().map(|word| base(word)) else {
            return words;
        };
        let second = words.get(1).map(String::as_str);
        let skip = match (first, second) {
            ("time" | "nohup" | "exec" | "command" | "env" | "caffeinate", _) => 1,
            ("timeout", _) => 2,
            ("npx" | "bunx" | "pnpx", _) => {
                1 + words[1..].iter().take_while(|word| word.starts_with('-')).count()
            }
            ("npm" | "pnpm" | "yarn" | "bun", Some("exec" | "dlx" | "x")) => {
                2 + words[2..].iter().take_while(|word| word.starts_with('-')).count()
            }
            ("uv" | "poetry" | "pipenv" | "rye" | "pdm" | "hatch", Some("run")) => 2,
            ("bundle", Some("exec")) => 2,
            _ => return words,
        };
        words = &words[skip.min(words.len())..];
    }
}

/// `npm run <script>`, `pnpm -r <script>`, `yarn <script>`, `turbo run <script>`, and the tools that
/// run a script by name.
fn runner(tool: &str, rest: &[&str]) -> Option<Verdict> {
    let mut rest = rest.iter().copied();
    let mut word = rest.next()?;
    // Options in front of the script; the ones that take a value take the next word too.
    while word.starts_with('-') {
        if matches!(word, "--filter" | "-F" | "-C" | "--prefix" | "--cwd" | "--dir" | "-w" | "--workspace") {
            rest.next();
        }
        word = rest.next()?;
    }
    if matches!(word, "run" | "run-script" | "task" | "run-many" | "exec") {
        word = rest.find(|next| !next.starts_with('-'))?;
    }
    match (tool, word) {
        ("deno", "test") | ("bun", "test") => is(RunKind::Tests),
        ("deno", "lint") => is(RunKind::Lint),
        ("deno", "fmt") => is(RunKind::Format),
        ("deno", "check") => is(RunKind::Typecheck),
        _ => script(word),
    }
}

/// A package script judged by its name, ahead of any colon: `test:unit` is a test.
fn script(name: &str) -> Option<Verdict> {
    match name.split(':').next().unwrap_or(name) {
        "test" | "tests" | "t" | "vitest" | "jest" | "e2e" | "spec" | "unit" => is(RunKind::Tests),
        "lint" | "eslint" => is(RunKind::Lint),
        "typecheck" | "type-check" | "check-types" | "tsc" | "types" => is(RunKind::Typecheck),
        "format" | "fmt" | "prettier" => is(RunKind::Format),
        "build" | "compile" | "bundle" | "package" => is(RunKind::Build),
        "dev" | "develop" | "storybook" => is(RunKind::DevServer),
        "start" | "serve" | "preview" => server_if_proven(),
        _ => None,
    }
}

fn cargo(rest: &[&str]) -> Option<Verdict> {
    let sub = rest.iter().find(|word| !word.starts_with('+') && !word.starts_with('-'))?;
    match *sub {
        "test" | "nextest" | "t" => is(RunKind::Tests),
        "clippy" => is(RunKind::Lint),
        "check" | "c" => is(RunKind::Typecheck),
        "build" | "b" | "install" => is(RunKind::Build),
        "fmt" => is(RunKind::Format),
        "run" | "r" | "watch" => server_if_proven(),
        _ => None,
    }
}

fn go(rest: &[&str]) -> Option<Verdict> {
    match rest.first().copied()? {
        "test" => is(RunKind::Tests),
        "vet" => is(RunKind::Lint),
        "build" | "install" => is(RunKind::Build),
        "fmt" => is(RunKind::Format),
        "run" => server_if_proven(),
        _ => None,
    }
}

fn make(rest: &[&str]) -> Option<Verdict> {
    // Only the target counts; `-C dir` and `-j4` come before it.
    let mut words = rest.iter().copied();
    let mut target = None;
    while let Some(word) = words.next() {
        match word {
            "-C" | "-f" | "-j" => {
                words.next();
            }
            word if word.starts_with('-') || word.contains('=') => {}
            word => {
                target = Some(word);
                break;
            }
        }
    }
    match target {
        None => is(RunKind::Build),
        Some("check") => is(RunKind::Tests),
        Some("all") => is(RunKind::Build),
        Some("run") => server_if_proven(),
        Some(name) => script(name),
    }
}

fn framework(rest: &[&str]) -> Option<Verdict> {
    match rest.first().copied()? {
        "build" | "export" => is(RunKind::Build),
        "dev" | "develop" | "serve" | "start:dev" => is(RunKind::DevServer),
        "start" | "preview" => server_if_proven(),
        "test" => is(RunKind::Tests),
        "lint" => is(RunKind::Lint),
        _ => None,
    }
}

fn python(rest: &[&str]) -> Option<Verdict> {
    match rest {
        ["-m", "pytest" | "unittest" | "nose2", ..] => is(RunKind::Tests),
        ["-m", "http.server", ..] | ["-m", "uvicorn" | "gunicorn" | "flask", ..] => is(RunKind::DevServer),
        ["-m", "ruff", "format", ..] => is(RunKind::Format),
        ["-m", "ruff" | "flake8" | "pylint", ..] => is(RunKind::Lint),
        ["-m", "mypy" | "pyright", ..] => is(RunKind::Typecheck),
        ["-m", "black" | "isort", ..] => is(RunKind::Format),
        [script, "runserver", ..] if script.ends_with("manage.py") => is(RunKind::DevServer),
        [script, "test", ..] if script.ends_with("manage.py") => is(RunKind::Tests),
        _ => None,
    }
}

fn jvm(rest: &[&str]) -> Option<Verdict> {
    rest.iter().find_map(|word| match *word {
        "test" | "check" | "verify" => is(RunKind::Tests),
        "build" | "assemble" | "package" | "compile" | "install" | "compileJava" => is(RunKind::Build),
        "spotlessApply" | "spotless:apply" => is(RunKind::Format),
        "bootRun" | "spring-boot:run" => is(RunKind::DevServer),
        _ => None,
    })
}
