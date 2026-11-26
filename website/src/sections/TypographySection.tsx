import { createMemo, createSignal, Show } from 'solid-js'
import { GlassButtonGroup, GlassButtonGroupItem, GlassButtonGroupSeparator, GlassTabs } from 'solid-liquid-glass'

import { createSpringSignal, createSwipePager, prefersReducedMotion, useReveal } from '../utils/motion'

type TrimKey = '32kai' | 'paperback' | 'trade'
type LangKey = 'en' | 'zh'

const TRIMS: Record<TrimKey, { label: string; mm: string; widthPx: number }> = {
  '32kai': { label: '32 开', mm: '130 × 184 mm · 口袋开本', widthPx: 460 },
  paperback: { label: '平装', mm: '129 × 198 mm · 经典平装', widthPx: 540 },
  trade: { label: '大开本', mm: '140 × 216 mm · 宽裕版心', widthPx: 620 },
}

const EN_PAGES = [
  {
    heading: 'Chapter I',
    paragraphs: [
      'It is a truth universally acknowledged, that a single man in possession of a good fortune, must be in want of a wife.',
      'However little known the feelings or views of such a man may be on his first entering a neighbourhood, this truth is so well fixed in the minds of the surrounding families, that he is considered the rightful property of some one or other of their daughters.',
    ],
  },
  {
    heading: 'Chapter I (cont.)',
    paragraphs: [
      '“My dear Mr. Bennet,” said his lady to him one day, “have you heard that Netherfield Park is let at last?”',
      'Mr. Bennet replied that he had not. “But it is,” returned she; “for Mrs. Long has just been here, and she told me all about it.” Mr. Bennet made no answer.',
    ],
  },
]

const ZH_PAGES = [
  {
    heading: '范爱农',
    paragraphs: [
      '在东京的客店里，我们大抵一起来就看报。学生少，看报的也不多，所以报来得早，往往在十点钟左右。',
      '一天早晨，我正坐在楼下廊檐前看报，忽然听见楼梯上响起一阵沉重的脚步声，抬头一看，却是一个高大身材、长头发、眼球白多黑少的人，看人总像在渺视。',
    ],
  },
  {
    heading: '范爱农（续）',
    paragraphs: [
      '他便是范爱农，徐锡麟的学生。那时我们因为发电报的事情，彼此很不相容，直到后来回到故乡绍兴，才在酒楼上慢慢谈起旧事。',
      '此后他便常来找我，有时一起喝几杯黄酒，谈起报上的新闻，彼此都不再像从前那样剑拔弩张，只觉得世事流转得实在太快。',
    ],
  },
]

function applyBoldWordStarts(text: string): string {
  return text.replace(/([A-Za-zÀ-ÖØ-öø-ÿ]{2,})/g, (word) => {
    const split = word.length <= 3 ? 1 : word.length <= 6 ? 2 : 3
    return `<b>${word.slice(0, split)}</b>${word.slice(split)}`
  })
}

export function TypographySection() {
  const [trim, setTrim] = createSignal<TrimKey>('paperback')
  const [lang, setLang] = createSignal<LangKey>('en')
  const [fineJustify, setFineJustify] = createSignal(true)
  const [bws, setBws] = createSignal(false)
  const [pageIdx, setPageIdx] = createSignal(0)

  const [sheetWidth, setSheetWidthTarget] = createSpringSignal(TRIMS.paperback.widthPx, {
    damping: 1.0,
    response: 0.38,
    precision: 0.3,
  })

  let headerRef: HTMLDivElement | undefined
  let sheetRef: HTMLDivElement | undefined
  let articleRef: HTMLElement | undefined

  const headerShown = useReveal(() => headerRef)

  const currentPages = createMemo(() => (lang() === 'en' ? EN_PAGES : ZH_PAGES))
  const currentPage = createMemo(() => currentPages()[pageIdx()] ?? currentPages()[0])
  const trimSpec = createMemo(() => TRIMS[trim()])

  const pager = createSwipePager({
    pageIndex: pageIdx,
    pageCount: () => currentPages().length,
    onTurn: (next) => setPageIdx(next),
    width: () => sheetRef?.clientWidth || 500,
  })

  const pulseOptical = () => {
    if (prefersReducedMotion() || !articleRef?.animate) return
    articleRef.animate(
      [
        { opacity: 0.7, filter: 'blur(1.6px)' },
        { opacity: 1, filter: 'blur(0px)' },
      ],
      { duration: 200, easing: 'cubic-bezier(0.22, 1, 0.36, 1)' },
    )
  }

  const handleTrimChange = (next: TrimKey) => {
    setTrim(next)
    setSheetWidthTarget(TRIMS[next].widthPx)
  }

  const handleLangChange = (next: LangKey) => {
    if (lang() === next) return
    setLang(next)
    setPageIdx(0)
    pulseOptical()
  }

  return (
    <section id="typography" class="bg-white px-5 py-24 sm:px-8 sm:py-32">
      <div class="mx-auto max-w-[1120px]">
        {/* Centered Apple Display Header */}
        <div
          ref={(el) => {
            headerRef = el
          }}
          class="mx-auto max-w-2xl text-center"
        >
          <p
            data-shown={headerShown() ? 'true' : 'false'}
            style={{ '--sp-delay': '0ms' }}
            class="sp-reveal text-[15px] font-semibold tracking-[-0.01em] text-[#D2693C]"
          >
            排印
          </p>
          <h2
            data-shown={headerShown() ? 'true' : 'false'}
            style={{ '--sp-delay': '60ms' }}
            class="sp-reveal mt-3 text-[32px] leading-[1.1] font-semibold tracking-[-0.03em] text-[#1D1D1F] [text-wrap:balance] sm:text-[46px]"
          >
            好排印，让你忘记屏幕的存在。
          </h2>
          <p
            data-shown={headerShown() ? 'true' : 'false'}
            style={{ '--sp-delay': '120ms' }}
            class="sp-reveal mt-4 text-[16px] leading-[1.6] text-[#6E6E73] sm:text-[18px]"
          >
            忠于书籍原本的版心比例。英文细心调匀词距与断行，中文严格守住标点挤压与行首避头，长文久读也从容不迫。
          </p>
        </div>

        {/* Studio Desk Stage — Physical sheet width springs smoothly between real book trims */}
        <div class="mt-14 rounded-[28px] bg-[#F5F5F7] px-4 pt-8 pb-7 sm:px-12 sm:pt-14 sm:pb-10">
          <div
            ref={(el) => {
              sheetRef = el
            }}
            {...pager.handlers}
            class={`sp-sheet relative mx-auto w-full overflow-hidden rounded-[10px] px-6 py-10 sm:px-14 sm:py-14 ${
              pager.isDragging() ? 'cursor-grabbing' : 'cursor-grab'
            }`}
            style={{
              'max-width': `${sheetWidth().toFixed(1)}px`,
              'touch-action': 'pan-y',
            }}
          >
            <button
              type="button"
              aria-label="上一页"
              onClick={() => pager.triggerTurn(-1)}
              class="absolute inset-y-0 left-0 z-10 w-1/5 cursor-w-resize bg-transparent"
            />
            <button
              type="button"
              aria-label="下一页"
              onClick={() => pager.triggerTurn(1)}
              class="absolute inset-y-0 right-0 z-10 w-1/5 cursor-e-resize bg-transparent"
            />

            <div
              class="flex min-h-[300px] flex-col justify-between will-change-transform"
              style={{
                transform: `translate3d(${pager.offsetX().toFixed(1)}px, 0, 0)`,
              }}
            >
              <article
                ref={(el) => {
                  articleRef = el
                }}
                class={`text-left ${
                  lang() === 'en'
                    ? 'sp-reading-en text-[19px] leading-[1.52] sm:text-[20px]'
                    : 'sp-reading-zh text-[18px] leading-[1.74] sm:text-[19px]'
                } ${bws() ? 'sp-bws' : ''}`}
              >
                <div class="mb-5 text-[12px] tracking-[0.06em] text-[#86868B]">
                  {currentPage().heading}
                </div>
                {currentPage().paragraphs.map((para) => (
                  <Show
                    when={lang() === 'en' && bws()}
                    fallback={
                      <p
                        class={`mb-4 indent-6 transition-[word-spacing] duration-200 ${
                          fineJustify()
                            ? 'text-justify [word-spacing:-0.02em] hyphens-auto'
                            : 'text-justify [word-spacing:0.24em]'
                        }`}
                      >
                        {para}
                      </p>
                    }
                  >
                    {/* Safe static showcase copy transformed by local prefix rule */}
                    <p
                      class={`mb-4 indent-6 transition-[word-spacing] duration-200 ${
                        fineJustify()
                          ? 'text-justify [word-spacing:-0.02em] hyphens-auto'
                          : 'text-justify [word-spacing:0.24em]'
                      }`}
                      innerHTML={applyBoldWordStarts(para)}
                    />
                  </Show>
                ))}
              </article>

              {/* Divider-less tabular footer */}
              <div class="sp-tabular relative z-20 mt-8 flex items-center justify-between text-[11.5px] text-[#3C3C43]/72">
                <div class="flex items-center gap-3">
                  <span>
                    {pageIdx() + 1}/{currentPages().length} 页
                  </span>
                  <span class="hidden text-[#86868B] sm:inline">· 拖拽书页或轻点边缘翻页</span>
                </div>
                <div class="flex items-center gap-3">
                  <button
                    type="button"
                    data-no-drag=""
                    onClick={() => pager.triggerTurn(-1)}
                    disabled={pageIdx() === 0}
                    class="cursor-pointer rounded px-1.5 py-0.5 text-[#55555B] hover:bg-black/[0.05] disabled:cursor-default disabled:opacity-35"
                  >
                    上一页
                  </button>
                  <button
                    type="button"
                    data-no-drag=""
                    onClick={() => pager.triggerTurn(1)}
                    disabled={pageIdx() >= currentPages().length - 1}
                    class="cursor-pointer rounded px-1.5 py-0.5 text-[#55555B] hover:bg-black/[0.05] disabled:cursor-default disabled:opacity-35"
                  >
                    下一页
                  </button>
                  <span>本章还剩 {currentPages().length - 1 - pageIdx()} 页</span>
                </div>
              </div>
            </div>
          </div>

          {/* Unified Floating Liquid Glass Control Dock */}
          <div class="mt-8 flex flex-col items-center gap-3">
            <div class="flex flex-wrap items-center justify-center gap-3">
              <GlassTabs
                aria-label="开本规格"
                size="sm"
                value={trim()}
                onValueChange={handleTrimChange}
                options={[
                  { value: '32kai', label: '32 开' },
                  { value: 'paperback', label: '平装' },
                  { value: 'trade', label: '大开本' },
                ]}
              />

              <GlassButtonGroup aria-label="语种与排印" material="clear" size="sm" swell="subtle" backdrop="#typography">
                <GlassButtonGroupItem
                  class={lang() === 'en' ? 'bg-black/[0.08] font-semibold text-[#1D1D1F]' : 'text-[#55555B]'}
                  onClick={() => handleLangChange('en')}
                >
                  英文
                </GlassButtonGroupItem>
                <GlassButtonGroupItem
                  class={lang() === 'zh' ? 'bg-black/[0.08] font-semibold text-[#1D1D1F]' : 'text-[#55555B]'}
                  onClick={() => handleLangChange('zh')}
                >
                  中文
                </GlassButtonGroupItem>
                <GlassButtonGroupSeparator />
                <GlassButtonGroupItem
                  class={fineJustify() ? 'bg-black/[0.08] font-semibold text-[#1D1D1F]' : 'text-[#55555B]'}
                  onClick={() => {
                    setFineJustify((v) => !v)
                    pulseOptical()
                  }}
                >
                  {fineJustify() ? '精细齐行：开' : '精细齐行：关'}
                </GlassButtonGroupItem>
                <GlassButtonGroupSeparator />
                <GlassButtonGroupItem
                  disabled={lang() === 'zh'}
                  class={
                    lang() === 'zh'
                      ? 'text-[#A1A1A6]'
                      : bws()
                        ? 'bg-black/[0.08] font-semibold text-[#1D1D1F]'
                        : 'text-[#55555B]'
                  }
                  onClick={() => {
                    if (lang() === 'en') {
                      setBws((v) => !v)
                      pulseOptical()
                    }
                  }}
                >
                  {lang() === 'zh' ? '词首引导（英文）' : bws() ? '词首引导：开' : '词首引导：关'}
                </GlassButtonGroupItem>
              </GlassButtonGroup>
            </div>

            <p class="sp-tabular text-[12px] text-[#86868B]">
              {trimSpec().mm} · {fineJustify() ? '精细齐行已开启，词距匀称' : '普通对齐模式，可见松散词距'}
            </p>
          </div>
        </div>
      </div>
    </section>
  )
}
