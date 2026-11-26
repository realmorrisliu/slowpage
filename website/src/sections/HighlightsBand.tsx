import { For } from 'solid-js'

import { IconBookOpen, IconLibrary, IconNote } from '../components/Icons'
import { useReveal } from '../utils/motion'

const HIGHLIGHTS = [
  {
    icon: IconBookOpen,
    title: '纯白如纸，零干扰',
    body: '拒绝花哨主题与视觉噪声。无论窗外如何变化，摊在你面前的永远是一张安静、锐利的纯白书页。',
  },
  {
    icon: IconNote,
    title: '页边批注，顺手伴读',
    body: '想法与 @alan 回复都挂在正在读的那一行旁，平时安静收起，需要时轻轻展开。',
  },
  {
    icon: IconLibrary,
    title: '你的书，始终在本地',
    body: '直接衔接 Apple Books、Calibre 与本地文件夹。不绑账号，不锁格式，断网同样完整可用。',
  },
] as const

export function HighlightsBand() {
  let bandRef: HTMLElement | undefined
  const shown = useReveal(() => bandRef)

  return (
    <section
      ref={(el) => {
        bandRef = el
      }}
      aria-label="核心特点"
      class="border-y border-black/[0.04] bg-[#F5F5F7] px-5 py-20 sm:px-8 sm:py-24"
    >
      <div class="mx-auto grid max-w-[1120px] gap-10 md:grid-cols-3 md:gap-12">
        <For each={HIGHLIGHTS}>
          {(item, i) => {
            const Icon = item.icon
            return (
              <div
                data-shown={shown() ? 'true' : 'false'}
                style={{ '--sp-delay': `${i() * 70}ms` }}
                class="sp-reveal text-left"
              >
                <div class="flex size-9 items-center justify-center rounded-[10px] bg-white text-[#1D1D1F] shadow-[0_0_0_0.5px_rgba(0,0,0,0.06),0_2px_8px_rgba(0,0,0,0.04)]">
                  <Icon class="size-4.5" />
                </div>
                <h2 class="mt-4 text-[18px] font-semibold tracking-[-0.015em] text-[#1D1D1F]">
                  {item.title}
                </h2>
                <p class="mt-2 text-[14.5px] leading-[1.65] text-[#6E6E73]">{item.body}</p>
              </div>
            )
          }}
        </For>
      </div>
    </section>
  )
}
