import fs from 'node:fs/promises'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const __dirname = path.dirname(fileURLToPath(import.meta.url))
const assetsDir = path.resolve(__dirname, '../.cloudflare/output/v1/workers/default/assets')
const zhHtmlPath = path.join(assetsDir, 'index.html')
const enHtmlPath = path.join(assetsDir, '_i18n/en.html')

const REPO_URL = 'https://github.com/realmorrisliu/slowpage'
// Keep in sync with the baked fallback in src/release.ts.
const FALLBACK_VERSION = 'Alpha 1'
const FALLBACK_DMG_URL =
  'https://github.com/realmorrisliu/slowpage/releases/download/v0.1.0-alpha.1/Slowpage-0.1.0-alpha.1.dmg'
const FALLBACK_DMG_SIZE = 'DMG 24.8 MB'

const LOCALES = {
  zh: {
    navLabel: '主导航',
    navTypography: '排印',
    navMarginalia: '页边批注',
    navLibrary: '书库',
    navOtherLocale: 'English',
    navOtherLocaleLabel: 'Switch to English',
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
    heroFine: ['macOS 14+', 'Apple 芯片', '免费开源', FALLBACK_VERSION, FALLBACK_DMG_SIZE],
  },
  en: {
    navLabel: 'Primary',
    navTypography: 'Typography',
    navMarginalia: 'Marginalia',
    navLibrary: 'Library',
    navOtherLocale: '中文',
    navOtherLocaleLabel: '切换到中文',
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
    heroFine: ['macOS 14+', 'Apple silicon', 'Free and open source', FALLBACK_VERSION, FALLBACK_DMG_SIZE],
  },
}

const EN_HEAD_META = {
  title: 'Slowpage — Read slower. Understand more.',
  description:
    'Slowpage is an EPUB and PDF reader for macOS: a quiet white page, paragraph-at-once typesetting for English and Chinese, notes in the margin and a built-in AI reading companion, @alan.',
  ogDescription:
    'An EPUB and PDF reader for macOS with book-grade typesetting, margin notes and a built-in AI reading companion.',
  ogImage: 'https://slowpage.app/og-en.png',
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
    `<button type="button" lang="${c.navOtherLocaleLang}" aria-label="${escapeHtml(c.navOtherLocaleLabel)}" class="cursor-pointer transition-colors duration-150 hover:text-[#1D1D1F]">${escapeHtml(c.navOtherLocale)}</button>`,
    `<a href="${REPO_URL}" target="_blank" rel="noopener noreferrer" data-track="github_click" class="hidden transition-colors duration-150 hover:text-[#1D1D1F] sm:inline">GitHub</a>`,
    `<a href="${FALLBACK_DMG_URL}" rel="noopener noreferrer" data-track="download_click" class="inline-flex h-7 shrink-0 cursor-pointer items-center justify-center rounded-full bg-[#1D1D1F] px-3.5 text-[12px] font-medium whitespace-nowrap text-white select-none">${escapeHtml(c.navDownload)}</a>`,
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
    `<a href="${FALLBACK_DMG_URL}" rel="noopener noreferrer" data-track="download_click" class="inline-flex h-12 shrink-0 cursor-pointer items-center justify-center rounded-full bg-[#1D1D1F] px-7 text-[15px] font-semibold whitespace-nowrap text-white select-none">${escapeHtml(c.heroDownload)}</a>`,
    `<a href="#preview" class="sp-link text-[17px]">${escapeHtml(c.heroTry)} ›</a>`,
    '</div>',
    `<p class="sp-tabular mt-4 text-[12.5px] text-[#59595E] [text-wrap:balance]">${c.heroFine.map((item) => `<span class="whitespace-nowrap">${escapeHtml(item)}</span>`).join(' · ')}</p>`,
    '</div>',
    // Matches StagePlaceholder in src/sections/HeroSection.tsx.
    '<div id="preview" data-stage-placeholder="" class="mx-auto mt-14 max-w-[1120px]" aria-hidden="true">',
    '<div class="mx-auto h-[620px] rounded-[16px] bg-gradient-to-b from-[#EEEEF0] to-[#E9E9EC] shadow-[var(--sp-window-shadow)] ring-1 ring-black/[0.08]" style="transform: translate3d(0, 5px, 0) scale(0.993)"></div>',
    '<div class="sp-stage-dock-space"></div>',
    '</div>',
    '</section>',
    '</main>',
    '</div>',
  ].join('')
}

function replaceRequired(source, pattern, replacement, label) {
  const next = source.replace(pattern, replacement)
  if (next === source) {
    throw new Error(`[prerender] failed to replace ${label}`)
  }
  return next
}

function localizeHtmlForEn(template) {
  let out = template
  out = replaceRequired(out, '<html lang="zh-CN">', '<html lang="en">', 'html lang')
  out = replaceRequired(
    out,
    /<title>[^<]*<\/title>/,
    `<title>${EN_HEAD_META.title}</title>`,
    'title',
  )
  out = replaceRequired(
    out,
    /(<meta\s+name="description"\s+content=")[^"]*(")/,
    `$1${EN_HEAD_META.description}$2`,
    'meta description',
  )
  out = replaceRequired(
    out,
    /(<meta\s+property="og:locale"\s+content=")zh_CN(")/,
    '$1en_US$2',
    'og:locale',
  )
  out = replaceRequired(
    out,
    /(<meta\s+property="og:title"\s+content=")[^"]*(")/,
    `$1${EN_HEAD_META.title}$2`,
    'og:title',
  )
  out = replaceRequired(
    out,
    /(<meta\s+property="og:description"\s+content=")[^"]*(")/,
    `$1${EN_HEAD_META.ogDescription}$2`,
    'og:description',
  )
  out = replaceRequired(
    out,
    /(<meta\s+property="og:image"\s+content=")https:\/\/slowpage\.app\/og\.png(")/,
    `$1${EN_HEAD_META.ogImage}$2`,
    'og:image',
  )
  out = replaceRequired(
    out,
    /(<meta\s+name="twitter:image"\s+content=")https:\/\/slowpage\.app\/og\.png(")/,
    `$1${EN_HEAD_META.ogImage}$2`,
    'twitter:image',
  )
  out = replaceRequired(
    out,
    /("description":\s*")[^"]*(")/,
    `$1${EN_HEAD_META.description}$2`,
    'json-ld description',
  )
  out = replaceRequired(
    out,
    /("inLanguage":\s*")zh-CN(")/,
    '$1en$2',
    'json-ld inLanguage',
  )
  out = replaceRequired(
    out,
    /("image":\s*")https:\/\/slowpage\.app\/og\.png(")/,
    `$1${EN_HEAD_META.ogImage}$2`,
    'json-ld image',
  )
  return out
}

const ROOT_PLACEHOLDER = '<div id="root"></div>'
const STYLESHEET_RE = /<link\s+rel="stylesheet"[^>]*href="(\/assets\/[^"]+\.css)"[^>]*>/

const rawHtml = await fs.readFile(zhHtmlPath, 'utf8')
if (!rawHtml.includes(ROOT_PLACEHOLDER)) {
  throw new Error(`[prerender] ${zhHtmlPath} does not contain ${ROOT_PLACEHOLDER}`)
}

const cssMatch = STYLESHEET_RE.exec(rawHtml)
if (!cssMatch) {
  throw new Error(`[prerender] ${zhHtmlPath} missing built stylesheet <link>`)
}
const cssPath = path.join(assetsDir, cssMatch[1].replace(/^\//, ''))
const inlineCss = (await fs.readFile(cssPath, 'utf8')).replaceAll('</style>', '<\\/style>')

const template = rawHtml.replace(cssMatch[0], `<style>${inlineCss}</style>`)

const zhHtml = template.replace(ROOT_PLACEHOLDER, `<div id="root">${renderShell(LOCALES.zh)}</div>`)
await fs.writeFile(zhHtmlPath, zhHtml, 'utf8')

const enHtml = localizeHtmlForEn(template).replace(
  ROOT_PLACEHOLDER,
  `<div id="root">${renderShell(LOCALES.en)}</div>`,
)
await fs.mkdir(path.dirname(enHtmlPath), { recursive: true })
await fs.writeFile(enHtmlPath, enHtml, 'utf8')

console.log(
  `[prerender] inlined ${cssMatch[1]} and wrote ${path.relative(process.cwd(), zhHtmlPath)} + ${path.relative(process.cwd(), enHtmlPath)}`,
)
