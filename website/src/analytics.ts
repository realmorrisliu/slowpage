/**
 * Minimal anonymous analytics, shared by name with alanworks.app and M3:
 * `$pageview` (automatic), `download_click`, `github_click` and `scroll_75`.
 * The PostHog loader (`/ph.js`) is injected after load, when the main thread
 * is idle, so analytics never competes with first paint or input.
 */
type PostHog = { capture: (event: string, props?: Record<string, unknown>) => void }

function ph(): PostHog | undefined {
  return (window as unknown as { posthog?: PostHog }).posthog
}

function loadPostHog() {
  const s = document.createElement('script')
  s.src = '/ph.js'
  s.async = true
  document.head.appendChild(s)
}

export function installAnalytics() {
  const idle = (cb: () => void) =>
    'requestIdleCallback' in window ? requestIdleCallback(cb, { timeout: 4000 }) : setTimeout(cb, 1500)
  if (document.readyState === 'complete') idle(loadPostHog)
  else window.addEventListener('load', () => idle(loadPostHog), { once: true })

  document.addEventListener(
    'click',
    (event) => {
      const el = (event.target as Element | null)?.closest<HTMLElement>('[data-track]')
      if (!el) return
      ph()?.capture(el.dataset.track!, { href: el.getAttribute('href') ?? undefined })
    },
    { capture: true },
  )

  let sent = false
  const onScroll = () => {
    if (sent) return
    const doc = document.documentElement
    const depth = (window.scrollY + window.innerHeight) / doc.scrollHeight
    if (depth >= 0.75) {
      sent = true
      ph()?.capture('scroll_75')
      window.removeEventListener('scroll', onScroll)
    }
  }
  window.addEventListener('scroll', onScroll, { passive: true })
}
