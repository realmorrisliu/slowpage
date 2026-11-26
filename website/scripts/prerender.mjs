import fs from 'node:fs/promises'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const __dirname = path.dirname(fileURLToPath(import.meta.url))
const assetsDir = path.resolve(__dirname, '../.cloudflare/output/v1/workers/default/assets')

const REPO_URL = 'https://github.com/realmorrisliu/slowpage'
const RELEASES_URL = `${REPO_URL}/releases/latest`
const FALLBACK_VERSION = 'Alpha 1'

const LOCALES = {
  zh: {
    file: path.join(assetsDir, 'index.html'),
    navLabel: '主导航',
    navTypography: '排印',
    navMarginalia: '页边批注',
    navLibrary: '书库',
    navOtherLocale: 'English',
    navOtherLocaleHref: '/en/',
    navOtherLocaleLang: 'en',
    navDownload: '下载',
    heroEyebrow: '为 macOS 打造的 EPUB 与 PDF 阅读器',
    heroTitle: '读得慢一点，懂得多一点。',
    heroLeadBefore: '一张安静的纯白书页，一位读过这本书的',
    heroAlan: '内置 AI 伴读 @alan',
    heroAlanTip:
      '@alan 是 Slowpage 内置的 AI 伴读，能解释、翻译、回答你选中的那一句，回复写在页边。',
    heroLeadAfter: '。像读一本印得很好的纸书，把想法记在页边。',
    heroDownload: '下载 macOS 版',
    heroTry: '在网页里试试',
    heroFine: `macOS 14+ · Apple Silicon · 免费开源 · ${FALLBACK_VERSION}`,
  },
  en: {
    file: path.join(assetsDir, 'en/index.html'),
    navLabel: 'Primary',
    navTypography: 'Typography',
    navMarginalia: 'Marginalia',
    navLibrary: 'Library',
    navOtherLocale: '中文',
    navOtherLocaleHref: '/',
    navOtherLocaleLang: 'zh-CN',
    navDownload: 'Download',
    heroEyebrow: 'An EPUB and PDF reader for macOS',
    heroTitle: 'Read slower. Understand more.',
    heroLeadBefore: 'A quiet white page, and a companion who has read the book —',
    heroAlan: 'the built-in AI reader @alan',
    heroAlanTip:
      '@alan is the AI reading companion built into Slowpage. It explains, translates and answers the line you select, and writes its reply in the margin.',
    heroLeadAfter: '. Read it like a well-printed book, and keep your thoughts in the margin.',
    heroDownload: 'Download for macOS',
    heroTry: 'Try it in the browser',
    heroFine: `macOS 14+ · Apple Silicon · Free and open source · ${FALLBACK_VERSION}`,
  },
}

function escapeHtml(text) {
  return text
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
    .replaceAll('"', '&quot;')
}

function renderShell(c) {
  return [
    '<div class="min-h-screen bg-white">',
    '<header class="sticky top-0 z-40 h-[52px] border-b transition-[background-color,border-color,backdrop-filter] duration-200 ease-out border-transparent bg-white/0">',
    `<nav aria-label="${escapeHtml(c.navLabel)}" class="mx-auto flex h-full max-w-[1120px] items-center justify-between px-5 sm:px-8">`,
    '<a href="#top" class="sp-pressable flex items-center gap-2 text-[15px] font-semibold tracking-[-0.015em] text-[#1D1D1F]">',
    '<img src="/slowpage-icon.png" alt="" width="24" height="24" class="size-6 drop-shadow-[0_1px_2px_rgba(0,0,0,0.1)]" />',
    '<span>Slowpage</span>',
    '</a>',
    '<div class="hidden items-center gap-8 text-[12.5px] font-normal text-[#424245] md:flex">',
    `<a href="#typography" class="transition-colors duration-150 hover:text-[#1D1D1F]">${escapeHtml(c.navTypography)}</a>`,
    `<a href="#marginalia" class="transition-colors duration-150 hover:text-[#1D1D1F]">${escapeHtml(c.navMarginalia)}</a>`,
    `<a href="#library" class="transition-colors duration-150 hover:text-[#1D1D1F]">${escapeHtml(c.navLibrary)}</a>`,
    '</div>',
    '<div class="flex items-center gap-5 text-[12.5px] text-[#424245]">',
    `<a href="${c.navOtherLocaleHref}" hreflang="${c.navOtherLocaleLang}" class="transition-colors duration-150 hover:text-[#1D1D1F]">${escapeHtml(c.navOtherLocale)}</a>`,
    `<a href="${REPO_URL}" target="_blank" rel="noopener noreferrer" data-track="github_click" class="hidden transition-colors duration-150 hover:text-[#1D1D1F] sm:inline">GitHub</a>`,
    `<a href="${RELEASES_URL}" target="_blank" rel="noopener noreferrer" data-track="download_click" class="inline-flex h-7 shrink-0 cursor-pointer items-center justify-center rounded-full bg-[#1D1D1F] px-3.5 text-[12px] font-medium whitespace-nowrap text-white select-none">${escapeHtml(c.navDownload)}</a>`,
    '</div>',
    '</nav>',
    '</header>',
    '<main>',
    '<section id="top" class="bg-white px-5 pt-14 pb-20 sm:px-8 sm:pt-20 sm:pb-28">',
    '<div class="mx-auto flex max-w-4xl flex-col items-center text-center">',
    `<p class="sp-eyebrow">${escapeHtml(c.heroEyebrow)}</p>`,
    `<h1 class="mt-3 text-[44px] leading-[1.06] font-semibold tracking-[-0.035em] text-[#1D1D1F] [text-wrap:balance] sm:text-[64px] lg:text-[76px]">${escapeHtml(c.heroTitle)}</h1>`,
    `<p class="relative mt-6 max-w-2xl text-[17px] leading-[1.6] text-[#59595E] [text-wrap:pretty] sm:text-[20px]">${escapeHtml(c.heroLeadBefore)} <span tabindex="0" aria-describedby="alan-tip" class="sp-alan-term font-medium text-[#1D1D1F]">${escapeHtml(c.heroAlan)}<span id="alan-tip" role="tooltip" class="sp-alan-tip">${escapeHtml(c.heroAlanTip)}</span></span>${escapeHtml(c.heroLeadAfter)}</p>`,
    '<div class="mt-9 flex flex-wrap items-center justify-center gap-x-7 gap-y-4">',
    `<a href="${RELEASES_URL}" target="_blank" rel="noopener noreferrer" data-track="download_click" class="inline-flex h-12 shrink-0 cursor-pointer items-center justify-center rounded-full bg-[#1D1D1F] px-7 text-[15px] font-semibold whitespace-nowrap text-white select-none">${escapeHtml(c.heroDownload)}</a>`,
    `<a href="#preview" class="sp-link text-[17px]">${escapeHtml(c.heroTry)} ›</a>`,
    '</div>',
    `<p class="sp-tabular mt-4 text-[12.5px] text-[#59595E]">${escapeHtml(c.heroFine)}</p>`,
    '</div>',
    '</section>',
    '</main>',
    '</div>',
  ].join('')
}

const ROOT_PLACEHOLDER = '<div id="root"></div>'
const STYLESHEET_RE = /<link\s+rel="stylesheet"[^>]*href="(\/assets\/[^"]+\.css)"[^>]*>/

const cssCache = new Map()
async function loadCss(href) {
  if (!cssCache.has(href)) {
    const cssPath = path.join(assetsDir, href.replace(/^\//, ''))
    const raw = await fs.readFile(cssPath, 'utf8')
    cssCache.set(href, raw.replaceAll('</style>', '<\\/style>'))
  }
  return cssCache.get(href)
}

for (const [locale, copy] of Object.entries(LOCALES)) {
  const html = await fs.readFile(copy.file, 'utf8')
  if (!html.includes(ROOT_PLACEHOLDER)) {
    throw new Error(`[prerender] ${copy.file} does not contain ${ROOT_PLACEHOLDER}`)
  }
  let updated = html.replace(ROOT_PLACEHOLDER, `<div id="root">${renderShell(copy)}</div>`)
  const cssMatch = STYLESHEET_RE.exec(updated)
  if (!cssMatch) {
    throw new Error(`[prerender] ${copy.file} missing built stylesheet <link>`)
  }
  const inlineCss = await loadCss(cssMatch[1])
  updated = updated.replace(cssMatch[0], `<style>${inlineCss}</style>`)
  await fs.writeFile(copy.file, updated, 'utf8')
  console.log(
    `[prerender] injected ${locale} shell + inlined ${cssMatch[1]} into ${path.relative(process.cwd(), copy.file)}`,
  )
}
