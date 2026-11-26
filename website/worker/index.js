const P = '\x1b[38;2;248;116;25m'
const B = '\x1b[1m'
const D = '\x1b[2m'
const X = '\x1b[0m'

const BANNER = `
  ${B}${P}SLOWPAGE${X}  —  读得慢一点，懂得多一点 · Read slower. Understand more.
  EPUB & PDF reader for macOS

  ${B}FEATURES${X}
    Paper-first      Opaque #FFFFFF reading sheet, warm serif proportions
    Marginalia       Line-anchored notes & in-process @alan reading companion
    Local library    Apple Books, Calibre, and local EPUB / PDF folders
    Typesetting      Paragraph-at-once (Knuth–Plass) justification, CJK kinsoku

  ${B}INSTALL / DOWNLOAD${X}
    Latest release   https://github.com/realmorrisliu/slowpage/releases/latest
    Source           git clone https://github.com/realmorrisliu/slowpage.git

  ${D}https://slowpage.app · MIT OR Apache-2.0 · Powered by Alan (https://alanworks.app)${X}
`

const EN_COOKIE = 'sp_lang=en; Path=/; Max-Age=31536000; SameSite=Lax'
const ROOT_VARY = 'Accept-Language, Cookie, User-Agent'

function parseCookieLang(cookieHeader) {
  if (!cookieHeader) return null
  for (const part of cookieHeader.split(';')) {
    const eq = part.indexOf('=')
    if (eq === -1) continue
    if (part.slice(0, eq).trim() !== 'sp_lang') continue
    const val = part.slice(eq + 1).trim()
    if (val === 'en' || val === 'zh') return val
  }
  return null
}

function parseAcceptLanguage(header) {
  if (!header) return 'zh'
  let bestEn = 0
  let bestZh = 0
  let firstEnIdx = Infinity
  let firstZhIdx = Infinity
  const parts = header.split(',')
  for (let i = 0; i < parts.length; i++) {
    const segments = parts[i].trim().split(';')
    const tag = (segments[0] || '').trim().toLowerCase()
    if (!tag) continue
    let q = 1
    for (let j = 1; j < segments.length; j++) {
      const param = segments[j].trim().toLowerCase()
      if (param.startsWith('q=')) {
        const parsed = Number.parseFloat(param.slice(2))
        if (!Number.isNaN(parsed)) q = parsed
      }
    }
    if (q <= 0) continue
    if (tag === 'en' || tag.startsWith('en-')) {
      if (q > bestEn) {
        bestEn = q
        firstEnIdx = i
      }
    } else if (tag === 'zh' || tag.startsWith('zh-')) {
      if (q > bestZh) {
        bestZh = q
        firstZhIdx = i
      }
    }
  }
  if (bestEn > bestZh) return 'en'
  if (bestZh > bestEn) return 'zh'
  if (bestEn > 0 && bestEn === bestZh) return firstEnIdx < firstZhIdx ? 'en' : 'zh'
  return 'zh'
}

function negotiateLocale(request) {
  return (
    parseCookieLang(request.headers.get('cookie')) ??
    parseAcceptLanguage(request.headers.get('accept-language'))
  )
}

export default {
  async fetch(request, env) {
    const ua = (request.headers.get('user-agent') || '').toLowerCase()
    const url = new URL(request.url)

    if (url.hostname === 'www.slowpage.app') {
      return Response.redirect(`https://slowpage.app${url.pathname}${url.search}`, 301)
    }

    if (url.pathname === '/_i18n' || url.pathname.startsWith('/_i18n/')) {
      return new Response('Not Found\n', {
        status: 404,
        headers: {
          'content-type': 'text/plain; charset=utf-8',
          'cache-control': 'no-store',
        },
      })
    }

    if (url.pathname === '/en' || url.pathname === '/en/') {
      return new Response(null, {
        status: 302,
        headers: {
          location: `${url.origin}/${url.search}`,
          'set-cookie': EN_COOKIE,
          'cache-control': 'no-store',
        },
      })
    }

    const isRoot = url.pathname === '/' || url.pathname === ''
    if (isRoot && (ua.includes('curl') || ua.includes('wget') || ua.includes('httpie'))) {
      return new Response(request.method === 'HEAD' ? null : BANNER, {
        headers: {
          'content-type': 'text/plain; charset=utf-8',
          'cache-control': 'public, max-age=300',
          vary: ROOT_VARY,
        },
      })
    }

    if (isRoot) {
      const lang = negotiateLocale(request)
      const targetPath = lang === 'en' ? '/_i18n/en' : '/'
      const res = await env.ASSETS.fetch(new Request(new URL(targetPath, url), request))
      const headers = new Headers(res.headers)
      headers.set('vary', ROOT_VARY)
      headers.set('content-language', lang === 'en' ? 'en' : 'zh-CN')
      headers.set('cache-control', 'private, no-cache')
      return new Response(res.body, {
        status: res.status,
        statusText: res.statusText,
        headers,
      })
    }

    return env.ASSETS.fetch(request)
  },
}
