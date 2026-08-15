# DSH Deck

Native desktop shell for [DeepSeek Harness](https://github.com/deepseek-ai/DeepSeek-Harness). It uses the operating system WebView to run the complete dsh Web UI and its Cordis browser plugins without Electron.

## Status

The desktop shell starts the bundled Node and dsh runtime beside its executable, waits for the loopback readiness URL, and opens that URL in a Wry window. Windows release builds use WebUI-matched frameless window controls and do not open a console window. Development mode can instead start dsh from a source checkout or connect to an already-running loopback Web UI.

## Requirements

- Node.js `^22.19.0` or `>=24.0.0`
- pnpm `11.7.0`
- Rust `1.94` or newer
- Windows with the WebView2 Runtime, or Linux with WebKitGTK 4.1 development packages

## Development

```sh
pnpm install
pnpm dev -- --dsh-root D:\path\to\deepseek-harness
```

The dsh checkout must already have its dependencies and built Web artifacts. DSH Deck passes `--port 0`, so dsh selects an available loopback port.

Connect to an existing server instead:

```sh
pnpm dev -- --url http://127.0.0.1:3080
```

Quality checks:

```sh
pnpm check
pnpm build
```

`pnpm check` validates the Rust shell and bundle builder. `pnpm build` builds the release native shell.

## Packaging

Provide a built DeepSeek Harness checkout and an unpacked, redistributable Node distribution for the target platform:

```sh
pnpm package -- --dsh-root D:\path\to\deepseek-harness --node-root D:\path\to\node-distribution
```

The command builds dsh and the native shell, packs both dsh npm release families, installs them with the supplied Node distribution, and writes `dist/bundle/`. The resulting executable starts `dist/bundle/runtime/` automatically and does not need pnpm or a source checkout at runtime. Use `--skip-dsh-build` or `--skip-native-build` only when the corresponding release artifacts are already current.

## Architecture

DSH Deck is not a separate agent or UI implementation. The ordinary `dsh-web-app` composition still owns the Web Host, API and browser plugin graph; DSH Deck owns only the native window, WebView, startup state and the dsh process it launches.

See [docs/architecture.md](docs/architecture.md) for the current boundaries and [DESIGN.md](DESIGN.md) for the product and interaction source of truth.

## License

DSH Deck is available under the [MIT License](LICENSE).

Dependencies remain under their own licenses. See [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
