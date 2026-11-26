import { createSignal } from 'solid-js'

/**
 * Marketing copy for the two locales. Interactive product demos also read
 * `locale()` so UI chrome, sample notes, and library descriptions switch in place.
 *
 * The initial locale comes from the `sp_lang` cookie (set when toggling in-page
 * or hitting `/en`), falling back to `<html lang>` set at the Cloudflare edge.
 */
export type Locale = 'zh' | 'en'

export function detectInitialLocale(): Locale {
  if (typeof document === 'undefined') return 'zh'
  const match = document.cookie.match(/(?:^|;\s*)sp_lang=(en|zh)(?:;|$)/)
  if (match) return match[1] as Locale
  return document.documentElement.lang.startsWith('en') ? 'en' : 'zh'
}

const [locale, setLocaleSignal] = createSignal<Locale>(detectInitialLocale())

export { locale }

export function setLocale(next: Locale) {
  if (typeof document !== 'undefined') {
    document.cookie = `sp_lang=${next}; Path=/; Max-Age=31536000; SameSite=Lax`
    document.documentElement.lang = next === 'en' ? 'en' : 'zh-CN'
    document.title =
      next === 'en'
        ? 'Slowpage — Read slower. Understand more.'
        : 'Slowpage — 读得慢一点，懂得多一点'
  }
  if (typeof window !== 'undefined') {
    ;(
      window as unknown as {
        posthog?: { register?: (props: Record<string, unknown>) => void }
      }
    ).posthog?.register?.({ locale: next })
  }
  setLocaleSignal(next)
}

export const REPO_URL = 'https://github.com/realmorrisliu/slowpage'
export const RELEASES_URL = `${REPO_URL}/releases/latest`
export const ISSUES_URL = `${REPO_URL}/issues`
export const ALAN_URL = 'https://alanworks.app'
export const M3_URL = 'https://m3.thinksmart.systems'

const zh = {
  navLabel: '主导航',
  navTypography: '排印',
  navMarginalia: '页边批注',
  navLibrary: '书库',
  navOtherLocale: 'English',
  navDownload: '下载',

  heroEyebrow: '为 macOS 打造的 EPUB 与 PDF 阅读器',
  heroTitle: '读得慢一点，懂得多一点。',
  heroLeadBefore: '一张安静的纯白书页，一位读过这本书的',
  heroAlan: '内置 AI 伴读 @alan',
  heroAlanTip: '@alan 是 Slowpage 内置的 AI 伴读，能解释、翻译、回答你选中的那一句，回复写在页边。',
  heroLeadAfter: '。像读一本印得很好的纸书，把想法记在页边。',
  heroDownload: '下载 macOS 版',
  heroTry: '在网页里试试',
  heroFine: (v: string) => `macOS 14+ · Apple Silicon · 免费开源 · ${v}`,

  typeEyebrow: '排印',
  typeTitle: '好排印，看得见差别。',
  typeLead: '同一段文字。左边是浏览器默认的两端对齐，右边是 Slowpage 阅读器里那套整段一起算的最优断行。',
  typeLeft: '浏览器默认',
  typeRight: 'Slowpage',
  typeLangEn: '英文',
  typeLangZh: '中文',
  typeNotesEn: [
    ['词距匀称', '整段一起断行，没有河流'],
    ['连字符', '按词典断词，最多连续两行'],
    ['首行缩进', '版心和印刷书一致'],
  ],
  typeNotesZh: [
    ['避头尾', '句号、逗号不落在行首'],
    ['标点挤压', '相邻标点不撑开'],
    ['中西间距', '汉字与西文之间留出细空'],
  ],

  marginEyebrow: '页边批注',
  marginTitle: ['想法落在页边，', '伴读就在这一行旁。'],
  marginLead: '不必跳出书页去打开聊天框。轻点一句原文记下眉批，或顺手问 @alan。安静时退到边缘，选中时才连起你正在读的那一行。',
  poweredByAlan: 'Powered by Alan',

  libEyebrow: '本地书库',
  libTitle: '你的书，始终在本地。',
  libLead: '直接接上 Apple Books、Calibre 和本地文件夹。不绑账号，不锁格式，不改写原文件，断网照样能读。',

  closeTitle: '今晚，安静读完一章。',
  closeLead: (v: string) => `Slowpage 处于公开预览阶段（${v}），免费开源。下载试用，或到 GitHub 告诉我们你期待的阅读体验。`,
  closeDownload: '下载 macOS 版',
  closeGithub: '前往 GitHub',
  closeFine: 'macOS 14 Sonoma 及以上 · Apple Silicon · EPUB / PDF · MIT / Apache-2.0',

  footerTagline: '为 macOS 打造的单页阅读器',
  footerDownload: '下载',
  footerFeedback: '反馈建议',
  footerLicense: '开源许可',
  footerFamily: '同一作者',
  footerPrivacy: '本站用 PostHog 统计匿名访问量与下载点击，不录屏，不识别个人。',
  footerCredits: 'MIT OR Apache-2.0 · 图标来自 IconMCP (MIT) · 网页液态玻璃组件开源于',
}

const en: typeof zh = {
  navLabel: 'Primary',
  navTypography: 'Typography',
  navMarginalia: 'Marginalia',
  navLibrary: 'Library',
  navOtherLocale: '中文',
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
  heroFine: (v: string) => `macOS 14+ · Apple Silicon · Free and open source · ${v}`,

  typeEyebrow: 'Typography',
  typeTitle: 'Good typesetting, visibly.',
  typeLead:
    'The same text, twice. On the left, the browser’s default justification. On the right, the paragraph-at-once line breaking the Slowpage reader uses.',
  typeLeft: 'Browser default',
  typeRight: 'Slowpage',
  typeLangEn: 'English',
  typeLangZh: 'Chinese',
  typeNotesEn: [
    ['Even spacing', 'the whole paragraph is broken at once, so no rivers'],
    ['Hyphenation', 'dictionary breaks, never more than two in a row'],
    ['Book measure', 'indents and trim match the printed page'],
  ],
  typeNotesZh: [
    ['Kinsoku', 'no full stop or comma starts a line'],
    ['Punctuation squeeze', 'adjacent marks do not spread apart'],
    ['Mixed-script spacing', 'a hair space between Han and Latin'],
  ],

  marginEyebrow: 'Marginalia',
  marginTitle: ['Your notes in the margin.', 'Your companion beside the line.'],
  marginLead:
    'No chat window to leave the page for. Tap a sentence to write a note, or ask @alan. It stays out of the way until you select the line you are reading.',
  poweredByAlan: 'Powered by Alan',

  libEyebrow: 'Local library',
  libTitle: 'Your books stay on your Mac.',
  libLead:
    'Works with Apple Books, Calibre and plain folders. No account, no format lock-in, no rewriting your files, and it works offline.',

  closeTitle: 'Read one quiet chapter tonight.',
  closeLead: (v: string) =>
    `Slowpage is in public preview (${v}) and free and open source. Try it, or tell us on GitHub what reading should feel like.`,
  closeDownload: 'Download for macOS',
  closeGithub: 'View on GitHub',
  closeFine: 'macOS 14 Sonoma or later · Apple Silicon · EPUB / PDF · MIT / Apache-2.0',

  footerTagline: 'A single-page reader for macOS',
  footerDownload: 'Download',
  footerFeedback: 'Feedback',
  footerLicense: 'License',
  footerFamily: 'Also by the author',
  footerPrivacy: 'This site counts anonymous visits and download clicks with PostHog. No session recording, no personal profiles.',
  footerCredits: 'MIT OR Apache-2.0 · Icons from IconMCP (MIT) · Liquid glass components open-sourced as',
}

export const t: typeof zh = new Proxy({} as typeof zh, {
  get(_target, prop) {
    const dict = locale() === 'en' ? en : zh
    return dict[prop as keyof typeof zh]
  },
})
