import { createSignal } from 'solid-js'

/** Human label for a release tag, e.g. `v0.1.0-alpha.2` → `Alpha 2`, `v1.0.0` → `v1.0.0`. */
export function releaseLabel(tag: string): string {
  const pre = /-(alpha|beta|rc)\.?(\d+)?$/i.exec(tag)
  if (!pre) return tag.startsWith('v') ? tag : `v${tag}`
  const kind = pre[1].toLowerCase()
  const name = kind === 'rc' ? 'RC' : kind[0].toUpperCase() + kind.slice(1)
  return pre[2] ? `${name} ${pre[2]}` : name
}

// Baked fallback matches the latest published release at build time.
const [version, setVersion] = createSignal('Alpha 1')
let requested = false

/** The latest published release label, refreshed once from GitHub. */
export function useReleaseVersion() {
  if (!requested && typeof fetch !== 'undefined') {
    requested = true
    fetch('https://api.github.com/repos/realmorrisliu/slowpage/releases/latest', {
      headers: { accept: 'application/vnd.github+json' },
    })
      .then((res) => (res.ok ? res.json() : null))
      .then((data: { tag_name?: unknown } | null) => {
        if (data && typeof data.tag_name === 'string') setVersion(releaseLabel(data.tag_name))
      })
      .catch(() => {})
  }
  return version
}
