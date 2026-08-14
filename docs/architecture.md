# Architecture

DSH Deck is a native system-WebView shell over DeepSeek Harness, not a fork of its runtime or Web UI.

## Process model

The MVP consists of a native owner process and the ordinary dsh Web composition:

```text
DSH Deck (Rust, Tao + Wry)
  ├─ starts `dsh web --port 0`
  ├─ waits for `dsh web: http://127.0.0.1:<port>`
  └─ loads that exact origin in the system WebView

dsh Web composition (Node.js + Cordis)
  ├─ dsh core and capability plugins
  ├─ Web Host, API and persistence plugins
  └─ browser plugin roster injected through `window.__DSH_BOOT__`
```

The packaged product starts `runtime/node` and the installed `runtime/dsh` entry beside the native executable. Development mode accepts `--dsh-root <checkout>`, which starts the source launcher through pnpm, or `--url <loopback-url>`, which connects to an externally owned Web composition. All modes preserve the same readiness and origin rules.

## Ownership

- Cordis owns agent, session, tool, persistence, settings, providers, browser plugins, and Web Host lifetime.
- Tao owns the native event loop and window.
- Wry owns the system WebView.
- DSH Deck owns a dsh process group it starts and stops/reaps the complete tree when the window closes. Windows uses a Job Object so pnpm's Node descendants cannot escape cleanup.
- DSH Deck does not own a runtime supplied through `--url`.

The MVP uses a hard process-group stop on window close. Graceful cross-platform dsh disposal is required before a packaged release so Cordis receives its normal shutdown signal and persistence finishes within dsh's bounded teardown period.

## Data access

The WebView loads the exact loopback origin served by `dsh-web-app`. Browser HTTP, WebSocket, boot-manifest, trust-fence, cancellation, and validation behavior therefore stay unchanged. DSH Deck must not serve frontend assets through a custom protocol because that would split the page and `/api` origins.

The startup stdout line is a narrow readiness protocol, not an application data channel. After navigation, all product data remains owned by the existing browser client and dsh API.

## Extension model

The existing dsh Host/Core and browser plugin ecosystems are reused without translation. `window.__DSH_BOOT__` remains the sole browser composition source, so an installed Web UI plugin appears in DSH Deck exactly as it does in a browser.

Desktop-only features belong in Cordis Host plugins and should reuse existing browser client contributions where they need UI. There is no separate DSH Deck renderer registry in the MVP.

## Platform boundary

Wry uses WebView2 on Windows and WebKitGTK 4.1 on Linux. Tao owns window-system integration. Platform dependencies, packaging, code signing, and updates stay outside dsh capability APIs.

DSH Deck source is MIT-licensed. Wry and Tao are dual MIT/Apache-2.0. A binary release must aggregate the complete transitive notices of the native and Node distributions.

## Near-term milestones

1. Start a real source-checkout `dsh web` composition and show it in Wry on Windows.
2. Display actionable startup/runtime failure pages and verify child cleanup.
3. Produce a bundle containing the native shell, a portable Node distribution, and a tarball-installed dsh release family.
4. Replace hard termination with dsh's bounded graceful shutdown on every supported platform.
5. Add application metadata, icon, persisted window geometry, and signed installers.
6. Add desktop-only Cordis capabilities only when a concrete product flow needs them.
