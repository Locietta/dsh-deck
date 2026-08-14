# AGENTS.md

DSH Deck is a native desktop shell for DeepSeek Harness. It uses Wry to host the existing dsh Web UI in the operating system WebView.

## Product direction

- Preserve the dsh plugin model. New behavior belongs in Cordis plugins and reversible effects.
- Reuse the complete dsh Web UI and its browser plugin roster. Do not reimplement the agent loop, session model, tool registry, settings, persistence, or browser-visible flows.
- Add desktop-native capabilities through dsh Host/Cordis plugins. Do not create a second desktop UI extension registry while the browser plugin model is sufficient.
- Keep the native shell narrow: window lifecycle, local-runtime startup, WebView navigation, and visible local failure states.

The project is pre-release and has no compatibility promise. Prefer a clear foundation over shims, aliases, or obsolete paths.

## Architecture rules

- Load only a dsh-announced loopback HTTP(S) URL. Do not weaken dsh's Host/Origin trust fence or add remote exposure before dsh has authentication.
- When DSH Deck starts the dsh process, closing the shell must stop and reap it. A runtime supplied through `--url` is externally owned and must not be stopped.
- Parse the readiness line as a narrow startup protocol and fail visibly when startup ends without it.
- Wry/Tao objects stay on the native event-loop thread. Background threads communicate through Tao user events.
- Keep platform-specific WebView code behind narrow functions and `cfg` blocks.
- Configuration that deployments may change belongs in CLI options or environment variables, not hidden constants.
- The future packaged runtime should mount a desktop bundle over `dsh-web-app`; the MVP source-checkout launcher must not fork that composition.

## Repository layout

```text
native/           Shipping Rust/Wry desktop shell
scripts/          Runtime bundle builder and narrow project checks
docs/             Architecture and maintained documentation
docs/working/     Ignored drafts for developer and agent discussion
```

## Commands

```sh
pnpm install
pnpm dev -- --dsh-root <deepseek-harness-checkout>
pnpm build
pnpm check
pnpm package -- --dsh-root <checkout> --node-root <portable-node-directory>
```

Use pnpm as the only package manager. Keep `packageManager` and the lockfile authoritative.

## Change expectations

- Update `README.md`, `DESIGN.md`, or `docs/architecture.md` when their stated behavior or decisions change.
- Add focused tests for startup parsing, URL policy, bundle layout, lifecycle, and visible failure behavior.
- Run the smallest checks that cover the change; `pnpm check` is the baseline before publishing a branch.
- Never commit credentials, generated `dist/`, `target/`, `node_modules/`, local absolute paths, or package-manager files from another tool.
- Preserve `LICENSE` and third-party notices. Generate a complete dependency notice before distributing binaries.
- Files end with exactly one trailing newline.
