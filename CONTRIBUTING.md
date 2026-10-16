# Contributing

OS Control is a public library. Everything that lands here is public, so keep personal data out of code, tests, fixtures, commit messages, and pull request text: no home paths, email addresses, names, customer data, or machine identifiers.

## Branches

`main` is what people install. It changes only through pull requests, and CI must pass.

| Branch | For |
|---|---|
| `ui/<name>` | Windows people see: the bar, onboarding, the gate panel |
| `core/<name>` | The tools, the gate rules, the MCP server |
| `docs/<name>` | README, WORKLOG, and docs only |
| `ci/<name>` | Workflows and build scripts |

## The pipeline

1. Branch from `main`: `git switch -c ui/<name> origin/main`.
2. Push early and open a **draft** pull request. CI runs on every push.
3. CI builds every crate, test target, and example. It runs the unit tests, the lint that stops OS state being read by parsing program output, and a build of the gate panel. It also checks added lines for home paths and personal email addresses.
4. CI cannot run the live checks, because they need Accessibility and Screen Recording grants. Run `make live` on your Mac and say so in the pull request.
5. A UI change shows a screenshot or a short GIF in the pull request.
6. When the work is ready, mark the pull request ready for review. Merge with **rebase**, so `main` keeps a straight history.
7. Add a `WORKLOG.md` entry for any change worth a session's notes: newest first, using the template at the top.

## UI branches

A UI branch can carry a runnable example under `crates/os-bar/examples/`. Run it with `cargo run -p os-bar --example <name>`. CI builds every example, so a broken one fails the pull request.

## Commits

- One commit per idea. The message says what changed and why.
- Use your GitHub `noreply` address as the commit email (GitHub, Settings, Emails), so your personal address stays private.
- A change to the gates needs a unit test in `gate.rs`. A change to a tool needs a live check.
