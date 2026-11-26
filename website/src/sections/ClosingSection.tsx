import { GlassButton } from 'solid-liquid-glass'

import { useReveal } from '../utils/motion'

const RELEASE_URL = 'https://github.com/realmorrisliu/slowpage/releases/tag/v0.1.0-alpha.1'
const REPO_URL = 'https://github.com/realmorrisliu/slowpage'

export function ClosingSection() {
  let blockRef: HTMLDivElement | undefined
  const shown = useReveal(() => blockRef)

  return (
    <section
      id="download"
      class="border-t border-black/[0.05] bg-[#F5F5F7] px-5 pt-24 pb-20 sm:px-8 sm:pt-32 sm:pb-24"
    >
      <div
        ref={(el) => {
          blockRef = el
        }}
        class="mx-auto max-w-2xl text-center"
      >
        <div
          data-shown={shown() ? 'true' : 'false'}
          style={{ '--sp-delay': '0ms' }}
          class="sp-reveal"
        >
          <img
            src="/slowpage-icon.png"
            alt="Slowpage"
            class="mx-auto size-20 drop-shadow-[0_12px_28px_rgba(29,29,31,0.14)]"
          />
        </div>

        <h2
          data-shown={shown() ? 'true' : 'false'}
          style={{ '--sp-delay': '60ms' }}
          class="sp-reveal mt-7 text-[36px] leading-[1.08] font-semibold tracking-[-0.03em] text-[#1D1D1F] [text-wrap:balance] sm:text-[52px]"
        >
          今晚，安静读完一章。
        </h2>

        <p
          data-shown={shown() ? 'true' : 'false'}
          style={{ '--sp-delay': '120ms' }}
          class="sp-reveal mx-auto mt-4 max-w-lg text-[16px] leading-[1.6] text-[#6E6E73] sm:text-[18px]"
        >
          Slowpage 目前处于早期公开预览阶段（Alpha 1），免费开源。下载试用，或到 GitHub 告诉我们你期待的阅读体验。
        </p>

        <div
          data-shown={shown() ? 'true' : 'false'}
          style={{ '--sp-delay': '180ms' }}
          class="sp-reveal mt-8 flex flex-wrap items-center justify-center gap-3.5"
        >
          <GlassButton
            size="lg"
            material="tinted"
            swell="subtle"
            backdrop="section"
            class="bg-[#1D1D1F] px-7 text-white"
            href={RELEASE_URL}
            target="_blank"
            rel="noopener noreferrer"
          >
            下载 macOS Alpha 1
          </GlassButton>
          <GlassButton
            size="lg"
            swell="subtle"
            backdrop="section"
            class="px-7"
            href={REPO_URL}
            target="_blank"
            rel="noopener noreferrer"
          >
            前往 GitHub
          </GlassButton>
        </div>

        <p
          data-shown={shown() ? 'true' : 'false'}
          style={{ '--sp-delay': '220ms' }}
          class="sp-reveal mt-4 text-[12px] text-[#86868B]"
        >
          macOS 14 Sonoma 及以上 · 支持 Apple Silicon · EPUB / PDF · MIT / Apache-2.0
        </p>
      </div>
    </section>
  )
}
