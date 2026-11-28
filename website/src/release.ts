import { createSignal } from 'solid-js'

import { RELEASES_URL } from './i18n'

/** Human label for a release tag, e.g. `v0.1.0-alpha.2` → `Alpha 2`, `v1.0.0` → `v1.0.0`. */
export function releaseLabel(tag: string): string {
  const pre = /-(alpha|beta|rc)\.?(\d+)?$/i.exec(tag)
  if (!pre) return tag.startsWith('v') ? tag : `v${tag}`
  const kind = pre[1].toLowerCase()
  const name = kind === 'rc' ? 'RC' : kind[0].toUpperCase() + kind.slice(1)
  return pre[2] ? `${name} ${pre[2]}` : name
}

export type Release = {
  /** Short label such as `Alpha 1`. */
  label: string
  /** Direct link to the macOS disk image, when the latest release has one. */
  dmgUrl: string | null
  /** Disk image size in bytes. */
  dmgSize: number | null
}

type GitHubRelease = {
  tag_name?: unknown
  assets?: Array<{ name?: unknown; size?: unknown; browser_download_url?: unknown }>
}

// Baked fallback matches the latest published release at build time, and
// scripts/prerender.mjs bakes the same values into the static HTML. The GitHub
// lookup replaces them once a newer release ships.
const [release, setRelease] = createSignal<Release>({
  label: 'Alpha 1',
  dmgUrl: 'https://github.com/thinksmart-systems/slowpage/releases/download/v0.1.0-alpha.1/Slowpage-0.1.0-alpha.1.dmg',
  dmgSize: 24_762_289,
})
let requested = false

/** The latest published release, refreshed once from GitHub. */
export function useRelease() {
  if (!requested && typeof fetch !== 'undefined') {
    requested = true
    fetch('https://api.github.com/repos/thinksmart-systems/slowpage/releases/latest', {
      headers: { accept: 'application/vnd.github+json' },
    })
      .then((res) => (res.ok ? res.json() : null))
      .then((data: GitHubRelease | null) => {
        if (!data || typeof data.tag_name !== 'string') return
        const dmg = data.assets?.find(
          (a) => typeof a.name === 'string' && a.name.endsWith('.dmg') && typeof a.browser_download_url === 'string',
        )
        setRelease({
          label: releaseLabel(data.tag_name),
          dmgUrl: dmg && typeof dmg.browser_download_url === 'string' ? dmg.browser_download_url : null,
          dmgSize: dmg && typeof dmg.size === 'number' ? dmg.size : null,
        })
      })
      .catch(() => {})
  }
  return release
}

/** The latest release label, e.g. `Alpha 1`. */
export function useReleaseVersion() {
  const current = useRelease()
  return () => current().label
}

/** Link props for a download button: the disk image itself, or the releases page. */
export function useDownloadLink() {
  const current = useRelease()
  return {
    href: () => current().dmgUrl ?? RELEASES_URL,
    // A direct download stays on this page; the releases page opens a new tab.
    target: () => (current().dmgUrl ? undefined : '_blank'),
    size: () => {
      const bytes = current().dmgSize
      return bytes ? `DMG ${(bytes / 1_000_000).toFixed(1)} MB` : null
    },
  }
}
