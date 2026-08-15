# DSH Deck Design

## Source of truth

- Status: Active — WebView MVP and integrated Windows chrome selected, brand identity open
- Last refreshed: 2026-08-15
- Primary product surface: the existing dsh Web UI hosted in a native system WebView
- Evidence reviewed: dsh Web bundle and browser plugin architecture and Wry 0.55 system-WebView support
- Current visual reference: the dsh Web UI is authoritative inside the WebView

This document records settled product and UI decisions. Unknowns stay explicit so implementation does not silently turn temporary choices into a design system. Entries are marked **Decision** (settled until this document changes), **Assumption** (a working choice awaiting validation), or listed under Open questions.

## MVP renderer decision

Decision — the first usable DSH Deck release reuses the complete dsh Web UI in the operating system's WebView through Wry. DSH Deck starts or connects to the loopback-only `dsh web` composition and navigates the WebView to its announced URL. The Web UI's Cordis client roster, components, tokens, accessibility behavior, localization, and extension points remain authoritative.

A future native surface must justify itself with a desktop-only job that the Web UI and host capability plugins cannot satisfy.

Desktop-owned presentation is deliberately narrow: native window creation, a local startup state, a local runtime-unavailable state, application icon and metadata, and future platform dialogs that belong outside browser content. These surfaces should visually defer to the Web UI instead of introducing a competing design system.

Decision — on Windows, DSH Deck removes the operating system title bar and reserves a 36px WebView title strip for window drag, minimize, maximize/restore, and close. The strip consumes the Web UI's own color, typography, border, hover, and focus tokens, with neutral fallbacks for the pre-runtime loading page. It contains no application navigation or duplicated runtime/session state. Release builds use the Windows GUI subsystem and start child processes without console windows; development builds retain their console for diagnostics.

## Brand

- Personality: focused, capable, calm, and developer-oriented.
- Trust signals: visible system state, exact action labels, reversible operations, and honest progress/error reporting.
- Avoid: ornamental dashboard density, imitation terminal aesthetics everywhere, anthropomorphic agent decoration, and hidden automation.

The name is **DSH Deck**. Logo, icon family, and public-facing relationship to the DeepSeek Harness brand remain open. Until that is resolved, the working visual identity is deliberately brand-neutral: graphite surfaces, one restrained interactive blue, and no logotype lockups baked into components (Assumption).

## Product goals

- Provide an installable desktop window over the existing dsh runtime and Web UI.
- Preserve Cordis composition, lifecycle, and plugin extensibility.
- Reuse the Web UI's conversation, tool, permission, session, configuration, keyboard, accessibility, and localization behavior without forking it.
- Add desktop-native capabilities through dsh Host/Cordis plugins rather than renderer-specific rewrites.

Non-goals for the foundation phase:

- Reimplementing dsh core behavior in the UI repository.
- Maintaining a parallel native implementation of browser-visible product flows.
- Defining a separate desktop UI plugin ABI while the browser plugin model remains sufficient.
- Supporting remote multi-user deployment.

Success initially means DSH Deck can start the real loopback dsh Web composition, display its complete Web UI, preserve the browser plugin roster, report startup/runtime failure visibly, and stop the owned runtime when the window closes.

## Personas and jobs

- Primary assumption: developers using dsh for coding and repository work on their own workstation.
- Core jobs: start or resume a session, understand current agent activity, inspect tool results, answer questions and permissions, navigate artifacts, and manage models/settings.
- Key context: long-running desktop work with frequent keyboard use, intermittent background activity, and multiple sessions.

Broader personas and non-coding workflows require validation before they influence navigation.

## Information architecture

Decision — the shipping information architecture is owned by the dsh Web UI. DSH Deck must not add an outer navigation rail, application toolbar, status bar, or duplicated session state around it. The Windows title strip is window chrome only. Native startup and error pages disappear as soon as the loopback Web UI is ready.

The provisional native workbench hierarchy below is retained only as historical product-design input.

Provisional hierarchy:

1. Workspaces and sessions.
2. Active conversation and activity timeline.
3. Composer and interaction requests.
4. Contextual inspectors for tool output, files, tasks, and session details.
5. Application settings and plugin management.

The initial shell should reserve space for navigation, the active timeline, and contextual detail without requiring all three regions to remain visible at narrow widths.

### Window regions

Decision — the shell is a three-column workbench under a slim header, with the composer docked inside the center column:

```text
┌────────────────────────────────────────────────────────────┐
│ Header bar — 40px: session title, model, runtime status    │
├──────────┬────────────────────────────────┬────────────────┤
│ Session  │ Conversation timeline          │ Inspector      │
│ rail     │ (primary, fluid width)         │ (contextual)   │
│ 260px    │                                │ 320–420px      │
│          ├────────────────────────────────┤                │
│          │ Composer (docked, grows up)    │                │
└──────────┴────────────────────────────────┴────────────────┘
```

- **Header bar (40px).** Owns whole-window identity: workspace/session name, active model, and the runtime connection state. It is the only place global connection state is shown; timeline entries report per-operation state.
- **Session rail (260px, collapsible).** Workspaces and sessions with status glyphs. Collapses to a toggle before the timeline loses room.
- **Timeline (fluid, minimum 480px).** The primary surface. Message and tool blocks are full-width rows, never chat bubbles.
- **Composer.** Bottom of the center column, grows upward to a capped height; pending questions and permission requests appear immediately above it so answering never requires scrolling away from input.
- **Inspector (320–420px, collapsible).** On-demand detail for tool output, files, diffs, and session metadata. Closed by default until content requests it.
- No status bar. Its usual jobs are covered by the header (connection), timeline (activity), and inspector (detail); adding one would duplicate state.

Region separation uses 1px borders on the window background — no gaps, shadows, or floating panels between regions.

## Design principles

- **Show committed truth.** Durable dsh events are authoritative; optimistic UI must be visually distinguishable and reconciled.
- **One activity, one place.** Do not duplicate the same state across transcript cards, status bars, and notifications without distinct user jobs.
- **Progressive detail.** Keep the timeline scannable and reveal raw arguments, logs, or diagnostics on demand.
- **Native without novelty.** Follow platform expectations for windows, focus, selection, menus, dialogs, and keyboard shortcuts.
- **Extensibility through data and registrations.** Plugin contributions declare intent and state; the shell owns consistent rendering and interaction policy.

## Visual language

The dsh Web UI owns the visual language of the shipping product. The remaining token and component decisions in this section apply only if a future desktop-only surface is approved; they do not authorize restyling or wrapping the Web UI for the MVP.

### Visual thesis

Decision: DSH Deck looks like a **calm instrument for long-running work** — closer to a well-kept editor or oscilloscope front panel than to a chat product. Dark graphite surfaces carry the content; color is spent almost entirely on meaning (status, focus, interaction) rather than identity. Structure comes from 1px borders and disciplined spacing, not cards floating on shadows. The agent is presented as machinery with visible state, not a persona.

Explicitly rejected: purple/gradient "AI" styling, glassmorphism and blur, heavily rounded card grids, imitation terminal chrome (scanlines, prompt glyphs, phosphor green) outside real terminal output, ornamental metrics or activity graphs without a user job, and browser-dashboard conventions (breadcrumb bars, hero headers, badge clutter).

### Color

Decision — dark theme first; it is the only shipped theme for the foundation phase. Any future desktop-owned components consume semantic tokens, never raw hex.

Surfaces and lines:

| Token | Hex | Role |
| --- | --- | --- |
| `bg-window` | `#101318` | Window background, region gutters |
| `bg-surface` | `#181D25` | Panels, timeline blocks, session rail |
| `bg-inset` | `#0C0F13` | Composer field, code/terminal wells, inputs |
| `bg-hover` | `#1D232D` | Hover on interactive rows and controls |
| `bg-selected` | `#232B37` | Selected row/tab; persists without focus |
| `border-default` | `#293241` | Region and component borders |
| `border-subtle` | `#1F2630` | Separators inside a surface |
| `overlay-scrim` | `#0C0F13` at 60% | Behind dialogs only |

Text:

| Token | Hex | Role |
| --- | --- | --- |
| `text-primary` | `#F2F4F8` | Message content, titles, values |
| `text-secondary` | `#AEB7C6` | Labels, metadata, timestamps |
| `text-muted` | `#7C8899` | Disabled text, placeholders, deemphasized paths |

Interaction accent — one blue, used only for interactive affordance and focus, never as decoration:

| Token | Hex | Role |
| --- | --- | --- |
| `accent` | `#4C8FD6` | Primary buttons, links, active toggles |
| `accent-hover` | `#66A3E0` | Hovered accent surfaces |
| `accent-pressed` | `#3B79B8` | Pressed accent surfaces |
| `focus-ring` | `#7DB4E8` | Keyboard focus indicator (see Accessibility) |

Status — each role has a foreground and a dim surface tint for block backgrounds; text on tints stays `text-primary`:

| Role | Foreground | Surface tint | Used for |
| --- | --- | --- | --- |
| Running | `#4C8FD6` | `#141B26` | In-flight tool calls, streaming turns |
| Success | `#4FBF8B` | `#13221B` | Committed results, applied changes |
| Warning | `#D9A03F` | `#241E12` | Pending permissions, degraded state |
| Danger | `#E06C60` | `#261513` | Failures, destructive confirmations |

Rules: status is always paired with a glyph or label, never color alone. Optimistic (uncommitted) content uses `text-secondary` with a `border-subtle` left rule and reconciles to primary styling when dsh commits it. A light theme reuses these token names; its values are an open question.

Assumption: the accent blue is a placeholder compatible with DeepSeek's blue family but chosen for contrast, not brand. It changes at most once, when the identity question resolves.

### Typography

Decision — two families, platform-native for UI:

- **UI:** `Segoe UI Variable` / `Segoe UI` on Windows; the system default sans (Noto Sans, Cantarell, Ubuntu) on Linux. No bundled brand font in the foundation phase.
- **Mono:** `Cascadia Mono`, falling back to `JetBrains Mono`, `Consolas`, then platform monospace — for code, paths, diffs, terminal output, IDs, and exact values. Anything the user might copy verbatim renders in mono.

Type scale (px, weight, line-height):

| Role | Size / weight / leading | Used for |
| --- | --- | --- |
| `type-title` | 20 / semibold / 28 | Window-level headers, empty-state headlines |
| `type-section` | 16 / semibold / 24 | Panel and inspector section titles |
| `type-body` | 14 / regular / 22 | Message content, descriptions, settings |
| `type-label` | 13 / medium / 18 | Buttons, row titles, tabs, field labels |
| `type-caption` | 12 / regular / 16 | Timestamps, metadata, counters |
| `type-code` | 13 / regular / 20 (mono) | Code, diffs, terminal, paths |

Weights stop at semibold; bold is reserved for inline emphasis inside rendered message content. No uppercase-tracking label style.

### Spacing

Decision — a 4px-based semantic scale; no one-off margins:

`space-1: 4` · `space-2: 8` · `space-3: 12` · `space-4: 16` · `space-6: 24` · `space-8: 32`

Application rules: control internal padding 8×12 (`space-2`/`space-3`); block internal padding 12–16; region padding 16; gap between timeline blocks 8; related label→value gap 4. Density leans compact — this is a work tool, not a marketing surface.

### Geometry and elevation

Decision:

- Corner radius: `radius-control: 4px` (buttons, inputs, tags), `radius-block: 6px` (timeline blocks, inspector cards, dialogs). Shell regions and the window itself are square. Nothing exceeds 6px; no pill shapes except small status tags.
- Borders: 1px everywhere; hierarchy comes from `border-default` vs `border-subtle`, not thickness.
- Elevation: the base plane is flat — separation by border and background step only. Exactly one shadow token, `shadow-overlay` (`0 4px 16px #000000` at 40%), reserved for true overlays: menus, dialogs, popovers, tooltips. Nothing in the base layout casts a shadow.

### Iconography

- Style: 16px stroke icons, 1.5px stroke, single color inheriting text tokens (Assumption: sourced from Lucide; final family open until the identity question resolves).
- Every icon-only control has an accessible label and a tooltip; status glyphs (`running` spinner-dot, `✓`, `!`, `×` equivalents) always sit next to or under a text label at first use in a view.
- No illustrations or mascots. Empty states are text plus one primary action.

### Motion

Decision — motion only explains state change; nothing loops except progress indication:

- Durations: `motion-fast: 80ms` (hover, press), `motion-base: 120ms` (expand/collapse, panel slide), `motion-slow: 180ms` (overlay enter). Ease-out for entrances, ease-in for exits.
- Animated: tool-block expand/collapse, inspector and rail slide, dialog fade+4px rise, streaming-text is appended not faded.
- Never animated: focus-ring movement (instant), color-only state changes on committed data, layout reflow on window resize, anything behind reduced-motion — when the platform reports reduced motion, all of the above become instant and the running indicator degrades to a static glyph that swaps on state change.

## Components

MVP components are the existing dsh browser components. DSH Deck owns only the native window plus its startup and runtime-unavailable documents. The component families below are product-design guidance, not MVP implementation requirements.

Foundation component families:

- Application shell and navigation.
- Session list and session status.
- Conversation timeline and message blocks.
- Tool presentation for `generic`, `terminal`, `diff`, and `locations` intents.
- Composer, attachments, commands, and skills.
- Question and permission interactions.
- Context inspector and native artifact actions.
- Settings, credentials, model selection, and plugin inventory.
- About and complete third-party notices for binary distribution.

Component tokens belong in the UI layer. Business state and lifecycle do not.

### Core component decisions

**Timeline blocks.** Full-width rows on `bg-surface`, separated by `space-2` gaps on `bg-window` — no left/right chat alignment and no avatars. Each block leads with a 13px label row: role or tool name (`type-label`), then metadata (`type-caption`, `text-secondary`), timestamp right-aligned. User input blocks are distinguished by a 2px `accent` left rule, not by color fill or alignment.

**Tool blocks.** One component per presentation intent (`generic`, `terminal`, `diff`, `locations`) sharing a common frame: a single-line header (status glyph, tool name in mono, one-line summary, duration, expand affordance) that is the collapse toggle. Collapsed is the default for succeeded calls; running and failed calls open expanded. Bodies render in `bg-inset` wells: terminal output in `type-code` with its real colors only, diffs with `#13221B`/`#261513` line tints plus `+`/`-` gutter markers, locations as clickable mono paths. Long output is clipped at ~16 lines with an exact "Show N more lines" action.

**Composer.** A `bg-inset` field with `border-default`, gaining a `focus-ring` when active. Grows from 1 to ~8 lines then scrolls internally. Attachments, command, and skill affordances are `text-secondary` icon buttons inside the field's bottom rail; the send button is the only `accent`-filled control normally visible in the shell. While the agent runs, send is replaced in place by an explicit Stop control — same position, `border-default` outline style with a `Danger`-colored label.

**Questions and permissions.** Rendered as blocks pinned above the composer with a `Warning` tint surface and left rule. They name the exact operation and object ("Allow `rm -rf build/` in `dsh-deck`?"), present verb-labeled actions ("Allow once", "Always allow in this session", "Deny"), and never auto-dismiss. Buttons are outline style; nothing defaults to the permissive choice.

**Session rail rows.** Two lines: session title (`type-label`) and last-activity summary (`type-caption`, truncated), with a leading status glyph (running / attention-needed / idle). Selected row uses `bg-selected` plus a 2px `accent` left rule; attention-needed shows the `Warning` glyph until the pending request is answered.

**Buttons.** Three variants only: `primary` (accent fill, one per view), `outline` (`border-default`, transparent fill), and `ghost` (no border, for icon and inline actions). Destructive actions use outline with `Danger` foreground and confirm with the object named.

**Inputs and selects.** `bg-inset`, 1px `border-default`, `radius-control`; labels above at `type-label`, validation below at `type-caption` in `Danger` foreground with the failing rule spelled out.

## Accessibility

The embedded Web UI retains its DOM semantics and browser accessibility behavior. The native shell must preserve keyboard focus inside the WebView, expose a meaningful window title, honor platform scaling, and avoid intercepting standard browser or editing shortcuts. The startup and error documents use semantic HTML and reduced-motion media queries.

- Target standard: to be selected; keyboard completeness and visible focus are required from the first interactive milestone. All shipped token pairs above meet WCAG AA contrast against their designated backgrounds (Decision: AA is the floor for text even before a formal target is chosen).
- Keyboard: every action must be reachable without a pointer; shortcuts must not trap text editing.
- Contrast and readability: semantic text and status colors must remain distinguishable without color alone.
- Screen readers: WebView and native-shell accessibility support must be verified on each supported OS before claiming conformance.
- Motion: provide a reduced-motion path before adding nonessential animation.

### Keyboard and focus

Decision:

- **Focus indicator:** a 2px `focus-ring` outline at 1px offset, drawn outside the control so layout never shifts. It appears for keyboard navigation and programmatic focus; pointer clicks show pressed state instead. Focus movement is never animated.
- **Region cycling:** `F6` / `Shift+F6` cycles session rail → timeline → composer → inspector (skipping collapsed regions); `Tab` moves within a region. The composer is the default focus target when a session opens.
- **Composer:** `Enter` sends, `Shift+Enter` inserts a newline (Assumption, to validate against dsh Web UI expectations). `Esc` first cancels composition popups, then moves focus to the timeline; a second `Esc` does not stop the agent — stopping is always an explicit control.
- **Timeline:** `↑`/`↓` move between blocks, `Enter`/`Space` toggles tool-block expansion, `Ctrl+C` copies the focused block's text content.
- **Pending requests:** when a question or permission block appears, it is announced and reachable via `F6`, but it never steals focus from active typing.
- Standard editing, selection, and window shortcuts follow the platform; DSH Deck does not override `Ctrl+C/V/X/Z/A` or `Alt+F4`.

## Responsive behavior

The WebView fills the native content area. Responsive breakpoints and compact layouts are inherited from the dsh Web UI; the desktop shell only enforces a usable minimum window size.

- Initial desktop targets: Windows and Linux.
- The window must remain usable at a compact laptop size; secondary regions collapse or become overlays before primary content becomes unreadable.
- Touch is not an initial target, but pointer targets should not depend on pixel-perfect precision; interactive targets keep a minimum 28px hit height even where the visual is smaller.
- macOS support remains an explicit product and CI decision.

### Compact-window behavior

Decision — regions degrade in a fixed order so the timeline and composer are always last to shrink:

- **≥ 1100px wide:** all three columns may be visible; the inspector still opens only on demand.
- **< 1100px:** an open inspector becomes an overlay sliding over the timeline from the right, dismissed by `Esc` or clicking the scrim-free timeline edge; it never squeezes the timeline below 480px.
- **< 880px:** the session rail collapses to a header-bar toggle; opening it overlays the timeline. Only one overlay (rail or inspector) may be open at a time.
- **Always:** the header bar and composer never collapse; the timeline never drops below its 480px readable width before overlaying kicks in; text never truncates in the composer or pending-request blocks (they wrap instead).
- Region visibility is remembered per window size band, so widening the window restores what the user had open.

## Interaction states

- Loading: identify the resource or phase being loaded; preserve usable prior state during refresh when safe.
- Empty: explain the next useful action rather than displaying a decorative blank state.
- Error: keep the failed operation, actionable reason, and retry/recovery path together.
- Success: prefer visible committed state over transient confirmation banners.
- Disabled: explain unmet prerequisites when the reason is not obvious.
- Disconnected: distinguish UI/runtime disconnection from model/provider failure and preserve readable local state.

### Control state treatments

Decision — every interactive control implements this ladder with token values; no bespoke per-component hover colors:

| State | Treatment |
| --- | --- |
| Rest | Token-defined fill and `border-default` |
| Hover | Fill steps to `bg-hover` (or `accent-hover` for primary); cursor change; 80ms transition |
| Pressed | Fill steps to `accent-pressed` / one background step darker; no scale or bounce effects |
| Focused | `focus-ring` outline per Accessibility; combines with hover/selected |
| Selected | `bg-selected` plus 2px `accent` left rule; persists when the region loses focus |
| Disabled | `text-muted` foreground, `border-subtle`, no hover response; tooltip states the unmet prerequisite |
| Busy | Control keeps its label, gains a leading running glyph, and blocks re-trigger; it is never silently swapped for a spinner |

## Content voice

- Tone: direct, compact, and factual.
- Terminology: follow dsh's canonical names for sessions, agents, tools, turns, models, and plugins.
- Microcopy: name the action and affected object; avoid vague labels such as “Something went wrong” or “Continue” without context.

## Implementation constraints

- Framework: Wry 0.55 over the operating system WebView, with Tao owning the native event loop.
- Runtime: the packaged product starts its adjacent Node and dsh runtime; development mode starts `dsh web --port 0` from a source checkout or connects to an explicitly supplied loopback URL.
- Transport: load the exact `http://127.0.0.1:<port>` origin announced by dsh. Do not move assets to a custom protocol because that would split the Web UI and `/api` origins.
- Plugin UI: the host-provided `window.__DSH_BOOT__` graph remains the only browser plugin composition source.
- Security: accept loopback URLs only until dsh has a real remote authentication layer. Do not broaden the server bind or trust fence from the desktop shell.
- Lifecycle: closing the only MVP window stops the complete process group of a runtime started by DSH Deck; a runtime supplied through `--url` remains externally owned.
- Package management: pnpm remains the repository entry point; pnpm scripts invoke Cargo and build the self-contained Node and dsh runtime bundle.
- Compatibility: Windows uses WebView2 and integrated frameless chrome; Linux uses WebKitGTK 4.1 with system decorations. Code signing and updates remain later milestones.
- Verification: Rust formatting, check and unit tests are baseline; the product gate starts the bundled dsh Web composition, confirms that no console window appears, and captures the resulting DSH Deck window including drag and window-control behavior.

## Open questions

- [ ] Who is the first narrow user persona beyond “developer using dsh”?
- [ ] Which visual identity and relationship to DeepSeek Harness branding should the app use? (This may replace the placeholder accent and icon family; surface, text, and status tokens are settled independently of it.)
- [ ] What are the light theme's token values, and is a light theme required before 1.0?
- [ ] Does dsh's Web UI set an Enter-to-send expectation that the composer must match?
- [ ] What accessibility conformance target is realistic across WebView2 and WebKitGTK?
- [ ] Is macOS part of the first supported release?
- [ ] Which desktop-only host capabilities, if any, need new Cordis plugins beyond the current Web bundle?
- [ ] How should window close, background runtime, tray presence, and full application exit relate?
- [ ] Which packaging and update mechanism will ship the native shell, Node, and dsh together?
