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

  ${D}https://slowpage.app · https://slowpage.app/en/ · MIT OR Apache-2.0 · Powered by Alan (https://alanworks.app)${X}
`

export default {
  async fetch(request, env) {
    const ua = (request.headers.get('user-agent') || '').toLowerCase()
    const url = new URL(request.url)

    if (url.hostname === 'www.slowpage.app') {
      return Response.redirect(`https://slowpage.app${url.pathname}${url.search}`, 301)
    }

    const isRoot = url.pathname === '/' || url.pathname === ''
    if (isRoot && (ua.includes('curl') || ua.includes('wget') || ua.includes('httpie'))) {
      return new Response(BANNER, {
        headers: {
          'content-type': 'text/plain; charset=utf-8',
          'cache-control': 'public, max-age=300',
          vary: 'User-Agent',
        },
      })
    }

    const res = await env.ASSETS.fetch(request)
    if (isRoot) {
      const headers = new Headers(res.headers)
      headers.set('vary', 'User-Agent')
      return new Response(res.body, {
        status: res.status,
        statusText: res.statusText,
        headers,
      })
    }
    return res
  },
}
