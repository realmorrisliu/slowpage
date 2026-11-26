import { createMemo, createSignal, For } from 'solid-js'
import { GlassTabs } from 'solid-liquid-glass'

import { BookCover, type CoverId } from '../components/BookCover'
import { IconBook, IconCheck, IconFolder, IconLibrary } from '../components/Icons'
import { t } from '../i18n'
import { prefersReducedMotion, useReveal } from '../utils/motion'

type SourceKey = 'apple-books' | 'calibre' | 'local-files'

// The card mirrors the app's Chinese library UI, like the other product demos.
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
    <section id="library" class="bg-[#F5F5F7] px-5 py-24 sm:px-8 sm:py-32">
      <div class="mx-auto grid max-w-[1120px] items-center gap-12 lg:grid-cols-12 lg:gap-16">
        {/* Library card on the left */}
        <div class="order-2 rounded-[28px] bg-white p-6 shadow-[var(--sp-card-shadow)] sm:p-9 lg:order-1 lg:col-span-7">
          <GlassTabs
            aria-label={t.libEyebrow}
            size="sm"
            value={source()}
            onValueChange={handleSourceChange}
            options={[
              { value: 'apple-books', label: 'Apple Books' },
              { value: 'calibre', label: 'Calibre' },
              { value: 'local-files', label: 'EPUB & PDF' },
            ]}
          />

          <div
            ref={(el) => {
              sourceCardRef = el
            }}
            class="mt-6 rounded-[18px] bg-[#F5F5F7] p-5 sm:p-6"
          >
            <div class="flex flex-wrap items-center justify-between gap-3 border-b border-black/[0.06] pb-4">
              <div class="flex items-center gap-3">
                <div class="flex size-9 items-center justify-center rounded-[10px] bg-white text-[#1D1D1F]">
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
                  <div class="text-[12px] text-[#6E6E73]">{activeSource().locationLabel}</div>
                </div>
              </div>

              <span class="sp-tabular rounded-full bg-white px-3 py-1 text-[11.5px] font-medium text-[#55555B]">
                {activeSource().countLabel}
              </span>
            </div>

            <div class="mt-5 flex flex-col gap-5 sm:flex-row sm:items-center">
              <div class="flex shrink-0 items-center gap-2.5">
                <For each={activeSource().covers}>{(coverId) => <BookCover id={coverId} size="mini" />}</For>
              </div>

              <ul class="space-y-2 text-[13.5px] leading-[1.55] text-[#3A3A3F]">
                <For each={activeSource().assurances}>
                  {(line) => (
                    <li class="flex items-start gap-2">
                      <IconCheck class="mt-0.5 size-4 shrink-0 text-[#B4531F]" />
                      <span>{line}</span>
                    </li>
                  )}
                </For>
              </ul>
            </div>
          </div>
        </div>

        {/* Copy on the right */}
        <div
          ref={(el) => {
            headerRef = el
          }}
          class="order-1 text-left lg:order-2 lg:col-span-5"
        >
          <p data-shown={headerShown() ? 'true' : 'false'} style={{ '--sp-delay': '0ms' }} class="sp-reveal sp-eyebrow">
            {t.libEyebrow}
          </p>
          <h2
            data-shown={headerShown() ? 'true' : 'false'}
            style={{ '--sp-delay': '60ms' }}
            class="sp-reveal mt-3 text-[32px] leading-[1.12] font-semibold tracking-[-0.03em] text-[#1D1D1F] [text-wrap:balance] sm:text-[44px]"
          >
            {t.libTitle}
          </h2>
          <p
            data-shown={headerShown() ? 'true' : 'false'}
            style={{ '--sp-delay': '120ms' }}
            class="sp-reveal mt-4 text-[16px] leading-[1.65] text-[#6E6E73] [text-wrap:pretty] sm:text-[17px]"
          >
            {t.libLead}
          </p>
        </div>
      </div>
    </section>
  )
}
