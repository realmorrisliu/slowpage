import { createEffect, createSignal, For, onSettled, Show } from 'solid-js'
import { GlassTabs } from 'solid-liquid-glass'

import { FactLine } from '../components/FactLine'
import { locale, t } from '../i18n'
import type { LayoutMetrics, SetLine } from '../typeset/knuthPlass'
import { useReveal } from '../utils/motion'

type LangKey = 'en' | 'zh'
type TrimKey = '32kai' | 'paperback' | 'trade'

type TrimSpec = {
  key: TrimKey
  labelZh: string
  labelEn: string
  dimensions: string
  measureClass: string
}

const TRIMS: readonly TrimSpec[] = [
  {
    key: '32kai',
    labelZh: '32 开',
    labelEn: 'Pocket',
    dimensions: '130 × 184 mm',
    measureClass: 'max-w-[340px]',
  },
  {
    key: 'paperback',
    labelZh: '平装',
    labelEn: 'Paperback',
    dimensions: '129 × 198 mm',
    measureClass: 'max-w-[430px]',
  },
  {
    key: 'trade',
    labelZh: '大开本',
    labelEn: 'Trade',
    dimensions: '140 × 216 mm',
    measureClass: 'max-w-[500px]',
  },
]

const EN_TEXT = [
  'It is a truth universally acknowledged, that a single man in possession of a good fortune, must be in want of a wife.',
  'However little known the feelings or views of such a man may be on his first entering a neighbourhood, this truth is so well fixed in the minds of the surrounding families, that he is considered the rightful property of some one or other of their daughters.',
]

// 鲁迅《范爱农》, from 朝花夕拾. The romanized name is in the original.
const ZH_TEXT = [
  '在东京的客店里，我们大抵一起来就看报。学生所看的多是《朝日新闻》和《读卖新闻》，专爱打听社会上琐事的就看《二六新闻》。一天早晨，辟头就看见一条从中国来的电报，大概是：“安徽巡抚恩铭被JoShikiRin刺杀，刺客就擒。”',
  '大家一怔之后，便容光焕发地互相告语，并且研究这刺客是谁，汉字是怎样三个字。但只要是绍兴人，又不专看教科书的，却早已明白了。',
]

function formatWordSpacing(ws: number): string {
  const rounded = Math.abs(ws) < 0.05 ? 0 : ws
  const sign = rounded > 0 ? '+' : rounded < 0 ? '−' : ''
  return `${sign}${Math.abs(rounded).toFixed(1)} px`
}

/** One page of English, set either by the browser or by the reader's line breaker. */
function EnglishPage(props: {
  mode: 'browser' | 'slowpage'
  trim: TrimKey
  onMetrics?: (metrics: LayoutMetrics | null) => void
}) {
  let root: HTMLDivElement | undefined
  // Loaded on demand: the hyphenation patterns are ~40 kB.
  let knuth:
    | {
        setParagraph: (text: string, font: string, width: number, indent: number) => SetLine[] | null
        measureLayoutMetrics: (paragraphs: SetLine[][]) => LayoutMetrics
      }
    | undefined
  let disposed = false
  let rafId = 0

  // Static copy only, so building HTML strings is safe here.
  const layout = () => {
    if (disposed || !root || props.mode !== 'slowpage' || !knuth) return
    const width = root.clientWidth
    if (width <= 0) return
    const style = getComputedStyle(root)
    const font = `${style.fontStyle} ${style.fontWeight} ${style.fontSize} ${style.fontFamily}`
    const indent = parseFloat(style.fontSize) * 1.4
    const html: string[] = []
    const allParagraphs: SetLine[][] = []
    let fallback = false

    for (const para of EN_TEXT) {
      const lines = knuth.setParagraph(para, font, width - 0.5, indent)
      if (!lines) {
        fallback = true
        html.push(`<p class="sp-cmp-p sp-cmp-browser">${para}</p>`)
        continue
      }
      allParagraphs.push(lines)
      const rows = lines.map(
        (line, i) =>
          `<span class="sp-cmp-line" style="word-spacing:${line.wordSpacing.toFixed(2)}px${
            i === 0 ? `;padding-left:${indent}px` : ''
          }">${line.text}</span>`,
      )
      html.push(`<p class="sp-cmp-p">${rows.join('')}</p>`)
    }

    root.innerHTML = html.join('')
    props.onMetrics?.(fallback ? null : knuth.measureLayoutMetrics(allParagraphs))
  }

  onSettled(() => {
    if (props.mode !== 'slowpage' || !root) return
    const ro = new ResizeObserver(layout)
    const onFontsLoaded = () => layout()
    document.fonts?.addEventListener?.('loadingdone', onFontsLoaded)
    // Load and run the line breaker only when the section comes near the viewport.
    const io = new IntersectionObserver(
      (entries) => {
        if (!entries.some((e) => e.isIntersecting)) return
        io.disconnect()
        import('../typeset/knuthPlass').then((mod) => {
          if (disposed || !root) return
          knuth = {
            setParagraph: mod.setParagraph,
            measureLayoutMetrics: mod.measureLayoutMetrics,
          }
          layout()
          document.fonts?.ready.then(() => {
            if (!disposed) layout()
          })
          ro.observe(root)
        })
      },
      { rootMargin: '200px 0px' },
    )
    io.observe(root)
    return () => {
      disposed = true
      if (rafId) cancelAnimationFrame(rafId)
      document.fonts?.removeEventListener?.('loadingdone', onFontsLoaded)
      io.disconnect()
      ro.disconnect()
      props.onMetrics?.(null)
    }
  })

  createEffect(
    () => props.trim,
    () => {
      if (props.mode !== 'slowpage' || !knuth) return
      if (rafId) cancelAnimationFrame(rafId)
      rafId = requestAnimationFrame(() => {
        rafId = 0
        layout()
      })
    },
  )

  return (
    <div
      ref={(el) => {
        root = el
      }}
      lang="en"
      class="sp-reading-en text-[17px] leading-[1.6] sm:text-[19px]"
    >
      <Show when={props.mode === 'browser'}>
        <For each={EN_TEXT}>{(para) => <p class="sp-cmp-p sp-cmp-browser">{para}</p>}</For>
      </Show>
    </div>
  )
}

function ChinesePage(props: { mode: 'browser' | 'slowpage' }) {
  return (
    <div lang="zh-CN" class="sp-reading-zh text-[17px] leading-[1.8] sm:text-[18px]">
      <For each={ZH_TEXT}>
        {(para) => <p class={`sp-cmp-p ${props.mode === 'browser' ? 'sp-cmp-zh-naive' : 'sp-cmp-zh-set'}`}>{para}</p>}
      </For>
    </div>
  )
}

function Page(props: {
  mode: 'browser' | 'slowpage'
  lang: LangKey
  trim: TrimSpec
  onMetrics?: (metrics: LayoutMetrics | null) => void
}) {
  return (
    <div class="px-5 pt-16 pb-10 sm:px-7 sm:pt-[72px] sm:pb-12">
      <div class={`mx-auto w-full ${props.trim.measureClass}`}>
        <Show when={props.lang === 'en'} fallback={<ChinesePage mode={props.mode} />}>
          <EnglishPage mode={props.mode} trim={props.trim.key} onMetrics={props.onMetrics} />
        </Show>
      </div>
    </div>
  )
}

export function TypographySection() {
  const [lang, setLang] = createSignal<LangKey>('en')
  const [trimKey, setTrimKey] = createSignal<TrimKey>('32kai')
  const [metrics, setMetrics] = createSignal<LayoutMetrics | null>(null)

  let headerRef: HTMLDivElement | undefined
  const headerShown = useReveal(() => headerRef)

  const notes = () => (lang() === 'en' ? t.typeNotesEn : t.typeNotesZh)
  const activeTrim = () => TRIMS.find((item) => item.key === trimKey()) ?? TRIMS[0]

  const caption = () => {
    const current = activeTrim()
    const label = locale() === 'en' ? current.labelEn : current.labelZh
    const base = [label, current.dimensions]
    const m = metrics()
    if (lang() !== 'en' || !m) return base
    const ws = formatWordSpacing(m.maxWordSpacing)
    return locale() === 'en'
      ? [...base, `${m.lines} lines`, `max word-spacing ${ws}`]
      : [...base, `${m.lines} 行`, `最大词距微调 ${ws}`]
  }

  return (
    <section id="typography" class="bg-[#F5F5F7] px-5 py-24 sm:px-8 sm:py-32">
      <div class="mx-auto max-w-[1120px]">
        <div
          ref={(el) => {
            headerRef = el
          }}
          class="mx-auto max-w-2xl text-center"
        >
          <p data-shown={headerShown() ? 'true' : 'false'} style={{ '--sp-delay': '0ms' }} class="sp-reveal sp-eyebrow">
            {t.typeEyebrow}
          </p>
          <h2
            data-shown={headerShown() ? 'true' : 'false'}
            style={{ '--sp-delay': '60ms' }}
            class="sp-reveal sp-h2 mt-3"
          >
            {t.typeTitle}
          </h2>
          <p
            data-shown={headerShown() ? 'true' : 'false'}
            style={{ '--sp-delay': '120ms' }}
            class="sp-reveal sp-lead mt-4"
          >
            {t.typeLead}
          </p>
        </div>

        <div class="mt-12 flex flex-wrap items-center justify-center gap-2.5 sm:gap-3">
          <GlassTabs
            aria-label={locale() === 'en' ? 'Language' : '语言'}
            size="sm"
            value={lang()}
            onValueChange={(v) => setLang(v as LangKey)}
            options={[
              { value: 'en', label: t.typeLangEn },
              { value: 'zh', label: t.typeLangZh },
            ]}
          />
          <GlassTabs
            aria-label={locale() === 'en' ? 'Book trim' : '书本开本'}
            size="sm"
            value={trimKey()}
            onValueChange={(v) => setTrimKey(v as TrimKey)}
            options={TRIMS.map((item) => ({
              value: item.key,
              label: locale() === 'en' ? item.labelEn : item.labelZh,
            }))}
          />
        </div>

        {/* Side by side: the same text, set by the browser and by the reader. */}
        <div class="mx-auto mt-8 grid max-w-[1120px] overflow-hidden rounded-[24px] bg-white shadow-[var(--sp-sheet-shadow)] md:grid-cols-2">
          <figure class="relative m-0">
            <figcaption class="sp-compare-tag left-4 bg-[#F0F0F2] text-[#4A4A50]">{t.typeLeft}</figcaption>
            <Page mode="browser" lang={lang()} trim={activeTrim()} />
          </figure>
          <figure class="relative m-0 border-t border-black/[0.06] md:border-t-0 md:border-l">
            <figcaption class="sp-compare-tag left-4 bg-[#FBEBE2] text-[#9A4317]">{t.typeRight}</figcaption>
            <Page mode="slowpage" lang={lang()} trim={activeTrim()} onMetrics={setMetrics} />
          </figure>
        </div>

        <p class="sp-tabular mx-auto mt-3.5 max-w-[1120px] text-center text-[13px] leading-[1.5] text-[#59595E] [text-wrap:balance]">
          <FactLine items={caption()} />
        </p>

        <ul class="mx-auto mt-8 grid max-w-[880px] gap-4 text-left sm:grid-cols-3 sm:gap-8 sm:text-center">
          <For each={notes()}>
            {(note) => (
              <li class="text-[14px] leading-[1.55] text-[#59595E] [text-wrap:balance]">
                <span class="block font-semibold text-[#1D1D1F]">{note[0]}</span>
                {note[1]}
              </li>
            )}
          </For>
        </ul>
      </div>
    </section>
  )
}
