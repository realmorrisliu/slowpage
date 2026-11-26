import { createMemo, createSignal, For, Show } from 'solid-js'
import { GlassButton, GlassTabs } from 'solid-liquid-glass'

import { BookCover, type CoverId } from '../components/BookCover'
import { IconBook, IconCheck, IconFolder, IconLibrary, IconList, IconMaximize, IconSidebarRight } from '../components/Icons'
import { prefersReducedMotion, useReveal } from '../utils/motion'

type SourceKey = 'apple-books' | 'calibre' | 'local-files'
type HudMode = 'glass' | 'accessible'

const SOURCE_DETAILS: Record<
  SourceKey,
  {
    name: string
    locationLabel: string
    countLabel: string
    covers: CoverId[]
    assurances: string[]
  }
> = {
  'apple-books': {
    name: 'Apple Books 书库',
    locationLabel: '本机 Apple Books 藏书目录',
    countLabel: '已索引 16 本',
    covers: ['pride', 'dawn-blossoms', 'walden'],
    assurances: [
      '自动发现系统里已有的书目、封面与分类',
      '不改写原始电子书文件，保持原书库整洁',
      '可识别尚未下载到本机的 iCloud 书目占位',
    ],
  },
  calibre: {
    name: 'Calibre 书库',
    locationLabel: '本机 Calibre 书库目录',
    countLabel: '已索引 48 本',
    covers: ['three-body', 'taocp', 'nahan'],
    assurances: [
      '直接读取作者、书名与已整理好的个人书单结构',
      '上千本藏书也能就地浏览，不必额外导入拷贝副本',
      '所有高亮、页边笔记与进度稳稳保存在本地',
    ],
  },
  'local-files': {
    name: 'EPUB & PDF',
    locationLabel: '~/Books 及任意本地文件夹',
    countLabel: '随选随读',
    covers: ['moby-dick', 'middlemarch', 'weicheng'],
    assurances: [
      'EPUB 自动整理版心、段首与中英文标点间距',
      'PDF 始终完整保留原本版面、公式与图表比例',
      '没有账号与云端强制同步，断网同样完整可用',
    ],
  },
}

export function LibrarySection() {
  const [source, setSource] = createSignal<SourceKey>('apple-books')
  const [hudMode, setHudMode] = createSignal<HudMode>('glass')

  let headerRef: HTMLDivElement | undefined
  let sourceCardRef: HTMLDivElement | undefined

  const headerShown = useReveal(() => headerRef)
  const activeSource = createMemo(() => SOURCE_DETAILS[source()])

  const handleSourceChange = (next: SourceKey) => {
    setSource(next)
    if (!prefersReducedMotion() && sourceCardRef?.animate) {
      sourceCardRef.animate(
        [
          { opacity: 0.72, filter: 'blur(1.5px)', transform: 'translateY(3px)' },
          { opacity: 1, filter: 'blur(0px)', transform: 'translateY(0)' },
        ],
        { duration: 220, easing: 'cubic-bezier(0.22, 1, 0.36, 1)' },
      )
    }
  }

  return (
    <section id="library" class="bg-white px-5 py-24 sm:px-8 sm:py-32">
      <div class="mx-auto max-w-[1120px]">
        {/* Centered Apple Header */}
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
            本地书库与原生质感
          </p>
          <h2
            data-shown={headerShown() ? 'true' : 'false'}
            style={{ '--sp-delay': '60ms' }}
            class="sp-reveal mt-3 text-[32px] leading-[1.1] font-semibold tracking-[-0.03em] text-[#1D1D1F] [text-wrap:balance] sm:text-[46px]"
          >
            为 macOS 原生打磨，为你的书库而生。
          </h2>
          <p
            data-shown={headerShown() ? 'true' : 'false'}
            style={{ '--sp-delay': '120ms' }}
            class="sp-reveal mt-4 text-[16px] leading-[1.6] text-[#6E6E73] sm:text-[18px]"
          >
            不用把藏书搬去陌生云端，也不用忍受沉重的套壳网页。打开 Slowpage，一切都在你最熟悉的苹果桌面手感里。
          </p>
        </div>

        {/* Asymmetric 2-Panel Apple Studio Grid */}
        <div class="mt-14 grid gap-6 lg:grid-cols-12">
          {/* Left 7-col Panel: Direct Local Library Connection */}
          <div class="flex flex-col justify-between rounded-[28px] bg-[#F5F5F7] p-7 text-left sm:p-10 lg:col-span-7">
            <div>
              <div class="flex flex-col justify-between gap-4 sm:flex-row sm:items-center">
                <div>
                  <h3 class="text-[22px] font-semibold tracking-[-0.02em] text-[#1D1D1F]">
                    直连你 Mac 上已有的书库
                  </h3>
                  <p class="mt-1.5 text-[14.5px] leading-[1.6] text-[#6E6E73]">
                    不需要漫长的导入搬家，书名、作者、封面与文件夹结构即刻归位。
                  </p>
                </div>
              </div>

              <div class="mt-5">
                <GlassTabs
                  aria-label="支持的书库来源"
                  size="sm"
                  value={source()}
                  onValueChange={handleSourceChange}
                  options={[
                    { value: 'apple-books', label: 'Apple Books' },
                    { value: 'calibre', label: 'Calibre' },
                    { value: 'local-files', label: 'EPUB & PDF' },
                  ]}
                />
              </div>
            </div>

            <div
              ref={(el) => {
                sourceCardRef = el
              }}
              class="sp-card mt-7 rounded-[18px] p-5 sm:p-6"
            >
              <div class="flex flex-wrap items-center justify-between gap-3 border-b border-black/[0.06] pb-4">
                <div class="flex items-center gap-3">
                  <div class="flex size-9 items-center justify-center rounded-[10px] bg-[#F5F5F7] text-[#1D1D1F]">
                    {source() === 'apple-books' ? (
                      <IconBook class="size-4.5" />
                    ) : source() === 'calibre' ? (
                      <IconLibrary class="size-4.5" />
                    ) : (
                      <IconFolder class="size-4.5" />
                    )}
                  </div>
                  <div>
                    <div class="text-[14px] font-semibold text-[#1D1D1F]">{activeSource().name}</div>
                    <div class="text-[12px] text-[#86868B]">{activeSource().locationLabel}</div>
                  </div>
                </div>

                <span class="sp-tabular rounded-full bg-[#F5F5F7] px-3 py-1 text-[11.5px] font-medium text-[#55555B]">
                  {activeSource().countLabel}
                </span>
              </div>

              <div class="mt-5 flex flex-col gap-5 sm:flex-row sm:items-center">
                <div class="flex shrink-0 items-center gap-2.5">
                  <For each={activeSource().covers}>
                    {(coverId) => <BookCover id={coverId} size="mini" />}
                  </For>
                </div>

                <ul class="space-y-2 text-[13.5px] leading-[1.55] text-[#3A3A3F]">
                  <For each={activeSource().assurances}>
                    {(line) => (
                      <li class="flex items-start gap-2">
                        <IconCheck class="mt-0.5 size-4 shrink-0 text-[#D2693C]" />
                        <span>{line}</span>
                      </li>
                    )}
                  </For>
                </ul>
              </div>
            </div>
          </div>

          {/* Right 5-col Panel: Zoomed-in Material Boundary (Glass Chrome vs Opaque Paper) */}
          <div class="flex flex-col justify-between rounded-[28px] bg-[#F5F5F7] p-7 text-left sm:p-10 lg:col-span-5">
            <div>
              <h3 class="text-[22px] font-semibold tracking-[-0.02em] text-[#1D1D1F]">
                玻璃属边缘，纯白属书页
              </h3>
              <p class="mt-1.5 text-[14.5px] leading-[1.6] text-[#6E6E73]">
                悬浮工具栏以液态玻璃轻浮于边缘；正中的读书白纸永远保持纯粹不透明，并主动响应系统辅助功能。
              </p>
            </div>

            <div class="mt-6">
              {/* Zoomed Upper Reading Card Stage */}
              <div class="relative overflow-hidden rounded-[18px] bg-gradient-to-b from-[#EEEEF0] to-[#E5E5E9] p-2.5 ring-1 ring-black/[0.06]">
                {/* Floating Top Edge Controls */}
                <div class="pointer-events-none absolute inset-x-4 top-4 z-20 flex items-center justify-between">
                  <Show
                    when={hudMode() === 'glass'}
                    fallback={
                      <button
                        type="button"
                        aria-label="目录（降低透明度示例）"
                        class="sp-pressable pointer-events-auto flex size-7 items-center justify-center rounded-full border border-black/15 bg-white/96 text-[#1D1D1F] shadow-xs"
                      >
                        <IconList class="size-3.5" />
                      </button>
                    }
                  >
                    <GlassButton size="sm" swell="subtle" backdrop="#library" class="pointer-events-auto h-7 w-7 !px-0" aria-label="目录">
                      <IconList class="size-3.5 text-[#1D1D1F]" />
                    </GlassButton>
                  </Show>

                  <div class="pointer-events-auto flex items-center gap-1.5">
                    <Show
                      when={hudMode() === 'glass'}
                      fallback={
                        <>
                          <button
                            type="button"
                            aria-label="全屏沉浸（降低透明度示例）"
                            class="sp-pressable flex size-7 items-center justify-center rounded-full border border-black/15 bg-white/96 text-[#1D1D1F] shadow-xs"
                          >
                            <IconMaximize class="size-3.5" />
                          </button>
                          <button
                            type="button"
                            aria-label="边注（降低透明度示例）"
                            class="sp-pressable flex size-7 items-center justify-center rounded-full border border-black/15 bg-white/96 text-[#1D1D1F] shadow-xs"
                          >
                            <IconSidebarRight class="size-3.5" />
                          </button>
                        </>
                      }
                    >
                      <GlassButton size="sm" swell="subtle" backdrop="#library" class="h-7 w-7 !px-0" aria-label="沉浸">
                        <IconMaximize class="size-3.5 text-[#1D1D1F]" />
                      </GlassButton>
                      <GlassButton size="sm" swell="subtle" backdrop="#library" class="h-7 w-7 !px-0" aria-label="边注">
                        <IconSidebarRight class="size-3.5 text-[#1D1D1F]" />
                      </GlassButton>
                    </Show>
                  </div>
                </div>

                {/* Opaque #FFFFFF Paper Detail */}
                <div class="sp-card rounded-[12px] px-5 pt-12 pb-6">
                  <div class="sp-reading-en text-[15px] leading-[1.6]">
                    <p class="mb-2 text-[11px] tracking-[0.04em] text-[#86868B]">Chapter 1 · The Grand Design</p>
                    <p class="text-justify">
                      Typography exists to honor content. Like oratory, music, dance, calligraphy — anything that lends its grace to language — it rests quietly until read.
                    </p>
                  </div>
                </div>
              </div>

              <div class="mt-4 flex items-center justify-between gap-2">
                <GlassTabs
                  aria-label="控件材质模式对比"
                  size="sm"
                  value={hudMode()}
                  onValueChange={(v) => setHudMode(v)}
                  options={[
                    { value: 'glass', label: '液态玻璃' },
                    { value: 'accessible', label: '降低透明度' },
                  ]}
                />
                <span class="text-[11.5px] text-[#86868B]">
                  {hudMode() === 'glass' ? '边缘轻透折射' : '高对比实体回退'}
                </span>
              </div>
            </div>
          </div>
        </div>
      </div>
    </section>
  )
}
