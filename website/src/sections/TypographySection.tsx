import { createSignal, For, onSettled, Show } from 'solid-js'
import { GlassTabs } from 'solid-liquid-glass'

import { t } from '../i18n'
import type { SetLine } from '../typeset/knuthPlass'
import { useReveal } from '../utils/motion'

type LangKey = 'en' | 'zh'

const EN_TEXT = [
  'It is a truth universally acknowledged, that a single man in possession of a good fortune, must be in want of a wife.',
  'However little known the feelings or views of such a man may be on his first entering a neighbourhood, this truth is so well fixed in the minds of the surrounding families, that he is considered the rightful property of some one or other of their daughters.',
]

// 鲁迅《范爱农》, from 朝花夕拾. The romanized name is in the original.
const ZH_TEXT = [
  '在东京的客店里，我们大抵一起来就看报。学生所看的多是《朝日新闻》和《读卖新闻》，专爱打听社会上琐事的就看《二六新闻》。一天早晨，辟头就看见一条从中国来的电报，大概是：“安徽巡抚恩铭被JoShikiRin刺杀，刺客就擒。”',
  '大家一怔之后，便容光焕发地互相告语，并且研究这刺客是谁，汉字是怎样三个字。但只要是绍兴人，又不专看教科书的，却早已明白了。',
]

/** One page of English, set either by the browser or by the reader's line breaker. */
function EnglishPage(props: { mode: 'browser' | 'slowpage' }) {
  let root: HTMLDivElement | undefined
  // Loaded on demand: the hyphenation patterns are ~40 kB.
  let setParagraph: ((text: string, font: string, width: number, indent: number) => SetLine[] | null) | undefined

  // Static copy only, so building HTML strings is safe here.
  const layout = () => {
    if (!root || props.mode !== 'slowpage' || !setParagraph) return
    const width = root.clientWidth
    if (width <= 0) return
    const style = getComputedStyle(root)
    const font = `${style.fontStyle} ${style.fontWeight} ${style.fontSize} ${style.fontFamily}`
    const indent = parseFloat(style.fontSize) * 1.4
    const html: string[] = []
    for (const para of EN_TEXT) {
      const lines = setParagraph(para, font, width - 0.5, indent)
      if (!lines) {
        html.push(`<p class="sp-cmp-p sp-cmp-browser">${para}</p>`)
        continue
      }
      const rows = lines.map(
        (line, i) =>
          `<span class="sp-cmp-line" style="word-spacing:${line.wordSpacing.toFixed(2)}px${
            i === 0 ? `;padding-left:${indent}px` : ''
          }">${line.text}</span>`,
      )
      html.push(`<p class="sp-cmp-p">${rows.join('')}</p>`)
    }
    root.innerHTML = html.join('')
  }

  onSettled(() => {
    if (props.mode !== 'slowpage' || !root) return
    const ro = new ResizeObserver(layout)
    // Load and run the line breaker only when the section comes near the viewport.
    const io = new IntersectionObserver(
      (entries) => {
        if (!entries.some((e) => e.isIntersecting)) return
        io.disconnect()
        import('../typeset/knuthPlass').then((mod) => {
          setParagraph = mod.setParagraph
          layout()
          document.fonts?.ready.then(layout)
          if (root) ro.observe(root)
        })
      },
      { rootMargin: '200px 0px' },
    )
    io.observe(root)
    return () => {
      io.disconnect()
      ro.disconnect()
    }
  })

  return (
    <div
      ref={(el) => {
        root = el
      }}
      lang="en"
      class="sp-reading-en text-[18px] leading-[1.6] sm:text-[19px]"
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

function Page(props: { mode: 'browser' | 'slowpage'; lang: LangKey }) {
  return (
    <div class="mx-auto w-full max-w-[520px] px-7 pt-16 pb-10 sm:px-11 sm:pt-[72px] sm:pb-12">
      <Show when={props.lang === 'en'} fallback={<ChinesePage mode={props.mode} />}>
        <EnglishPage mode={props.mode} />
      </Show>
    </div>
  )
}

export function TypographySection() {
  const [lang, setLang] = createSignal<LangKey>('en')
  
  let headerRef: HTMLDivElement | undefined
  const headerShown = useReveal(() => headerRef)

  const notes = () => (lang() === 'en' ? t.typeNotesEn : t.typeNotesZh)

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
            class="sp-reveal mt-3 text-[34px] leading-[1.1] font-semibold tracking-[-0.03em] text-[#1D1D1F] [text-wrap:balance] sm:text-[52px]"
          >
            {t.typeTitle}
          </h2>
          <p
            data-shown={headerShown() ? 'true' : 'false'}
            style={{ '--sp-delay': '120ms' }}
            class="sp-reveal mt-4 text-[16px] leading-[1.6] text-[#6E6E73] [text-wrap:pretty] sm:text-[18px]"
          >
            {t.typeLead}
          </p>
        </div>

        <div class="mt-12 flex justify-center">
          <GlassTabs
            aria-label={t.typeEyebrow}
            size="sm"
            value={lang()}
            onValueChange={(v) => setLang(v as LangKey)}
            options={[
              { value: 'en', label: t.typeLangEn },
              { value: 'zh', label: t.typeLangZh },
            ]}
          />
        </div>

        {/* Side by side: the same text, set by the browser and by the reader. */}
        <div class="mx-auto mt-8 grid max-w-[1040px] overflow-hidden rounded-[24px] bg-white shadow-[var(--sp-sheet-shadow)] md:grid-cols-2">
          <figure class="relative m-0">
            <figcaption class="sp-compare-tag left-4 bg-[#F0F0F2] text-[#55555B]">{t.typeLeft}</figcaption>
            <Page mode="browser" lang={lang()} />
          </figure>
          <figure class="relative m-0 border-t border-black/[0.06] md:border-t-0 md:border-l">
            <figcaption class="sp-compare-tag left-4 bg-[#FBEBE2] text-[#9A4317]">{t.typeRight}</figcaption>
            <Page mode="slowpage" lang={lang()} />
          </figure>
        </div>

        <ul class="mx-auto mt-8 grid max-w-[1040px] gap-4 text-left sm:grid-cols-3 sm:gap-8">
          <For each={notes()}>
            {(note) => (
              <li class="text-[14px] leading-[1.55] text-[#6E6E73]">
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
