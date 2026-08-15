(() => {
  if (window.top !== window) return

  const TITLEBAR_ID = 'dsh-deck-titlebar'
  const post = action => window.ipc?.postMessage(`dsh-deck:${action}`)

  window.__DSH_DECK_SET_MAXIMIZED__ = maximized => {
    const titlebar = document.getElementById(TITLEBAR_ID)
    titlebar?.toggleAttribute('data-maximized', maximized)
    const button = titlebar?.querySelector("button[data-action='toggle-maximize']")
    const label = maximized ? 'Restore' : 'Maximize'
    button?.setAttribute('title', label)
    button?.setAttribute('aria-label', label)
  }

  const install = () => {
    if (document.getElementById(TITLEBAR_ID)) return

    const style = document.createElement('style')
    style.textContent = `
      html[data-dsh-deck-desktop] body {
        box-sizing: border-box !important;
        height: 100% !important;
        padding-top: 36px !important;
      }

      #${TITLEBAR_ID} {
        position: fixed;
        inset: 0 0 auto;
        z-index: 2147483647;
        box-sizing: border-box;
        height: 36px;
        display: flex;
        align-items: center;
        color: var(--dsw-alias-label-secondary, #aeb7c6);
        background: var(--dsw-specific-sidebar-fill, var(--dsw-alias-bg-base, #101318));
        border-bottom: 1px solid var(--dsw-alias-border-l1, #293241);
        font: 12px/1 var(--dsw-font-family, 'Segoe UI Variable', 'Segoe UI', sans-serif);
        user-select: none;
      }

      #${TITLEBAR_ID} .dsh-deck-title {
        min-width: 0;
        flex: 1;
        padding-inline: 12px;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
        pointer-events: none;
      }

      #${TITLEBAR_ID} .dsh-deck-controls {
        align-self: stretch;
        display: flex;
      }

      #${TITLEBAR_ID} button {
        width: 46px;
        height: 35px;
        margin: 0;
        padding: 0;
        border: 0;
        border-radius: 0;
        color: inherit;
        background: transparent;
        font: 16px/1 'Segoe UI Symbol', 'Segoe UI', sans-serif;
      }

      #${TITLEBAR_ID} button:hover {
        color: var(--dsw-alias-label-primary, #f2f4f8);
        background: var(--dsw-alias-interactive-bg-hover, #1d232d);
      }

      #${TITLEBAR_ID} button:focus-visible {
        outline: 2px solid var(--dsw-alias-brand-primary, #4c8fd6);
        outline-offset: -2px;
      }

      #${TITLEBAR_ID} [data-action='close']:hover {
        color: #fff;
        background: #c42b1c;
      }

      #${TITLEBAR_ID} .dsh-deck-restore,
      #${TITLEBAR_ID}[data-maximized] .dsh-deck-maximize {
        display: none;
      }

      #${TITLEBAR_ID}[data-maximized] .dsh-deck-restore {
        display: inline;
      }
    `
    document.head.append(style)
    document.documentElement.setAttribute('data-dsh-deck-desktop', '')

    const titlebar = document.createElement('header')
    titlebar.id = TITLEBAR_ID
    titlebar.setAttribute('aria-label', 'Window controls')
    titlebar.innerHTML = `
      <span class="dsh-deck-title">DSH Deck</span>
      <span class="dsh-deck-controls">
        <button type="button" data-action="minimize" title="Minimize" aria-label="Minimize">−</button>
        <button type="button" data-action="toggle-maximize" title="Maximize" aria-label="Maximize"><span class="dsh-deck-maximize">□</span><span class="dsh-deck-restore">❐</span></button>
        <button type="button" data-action="close" title="Close" aria-label="Close">×</button>
      </span>
    `

    titlebar.addEventListener('pointerdown', event => {
      if (event.button === 0 && !(event.target instanceof Element && event.target.closest('button'))) {
        post('drag')
      }
    })
    titlebar.addEventListener('dblclick', event => {
      if (!(event.target instanceof Element && event.target.closest('button'))) post('toggle-maximize')
    })
    titlebar.addEventListener('click', event => {
      const button = event.target instanceof Element ? event.target.closest('button[data-action]') : null
      if (button instanceof HTMLElement) post(button.dataset.action)
    })

    document.body.prepend(titlebar)
  }

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', install, { once: true })
  } else {
    install()
  }
})()
