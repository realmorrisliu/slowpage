import { GlassButton } from 'solid-liquid-glass'

import { RELEASES_URL, REPO_URL, t } from '../i18n'
import { useReleaseVersion } from '../release'
import { useReveal } from '../utils/motion'

export function ClosingSection() {
  let blockRef: HTMLDivElement | undefined
  const shown = useReveal(() => blockRef)
  const version = useReleaseVersion()

  return (
    <section id="download" class="bg-white px-5 pt-24 pb-24 sm:px-8 sm:pt-32 sm:pb-32">
      <div
        ref={(el) => {
          blockRef = el
        }}
        class="mx-auto max-w-2xl text-center"
      >
        <div data-shown={shown() ? 'true' : 'false'} style={{ '--sp-delay': '0ms' }} class="sp-reveal">
          <img
            src="/slowpage-icon.png"
            alt="Slowpage"
            width="80"
            height="80"
            class="mx-auto size-20 drop-shadow-[0_12px_28px_rgba(29,29,31,0.14)]"
          />
        </div>

        <h2
          data-shown={shown() ? 'true' : 'false'}
          style={{ '--sp-delay': '60ms' }}
          class="sp-reveal mt-7 text-[36px] leading-[1.08] font-semibold tracking-[-0.03em] text-[#1D1D1F] [text-wrap:balance] sm:text-[52px]"
        >
          {t.closeTitle}
        </h2>

        <p
          data-shown={shown() ? 'true' : 'false'}
          style={{ '--sp-delay': '120ms' }}
          class="sp-reveal mx-auto mt-4 max-w-lg text-[16px] leading-[1.6] text-[#6E6E73] [text-wrap:pretty] sm:text-[18px]"
        >
          {t.closeLead(version())}
        </p>

        <div
          data-shown={shown() ? 'true' : 'false'}
          style={{ '--sp-delay': '180ms' }}
          class="sp-reveal mt-8 flex flex-wrap items-center justify-center gap-x-7 gap-y-4"
        >
          <GlassButton
            size="lg"
            material="tinted"
            swell="subtle"
            backdrop="section"
            class="bg-[#1D1D1F] px-7 text-white"
            href={RELEASES_URL}
            target="_blank"
            rel="noopener noreferrer"
            data-track="download_click"
          >
            {t.closeDownload}
          </GlassButton>
          <a
            href={REPO_URL}
            target="_blank"
            rel="noopener noreferrer"
            data-track="github_click"
            class="sp-link text-[17px]"
          >
            {t.closeGithub} ›
          </a>
        </div>

        <p
          data-shown={shown() ? 'true' : 'false'}
          style={{ '--sp-delay': '220ms' }}
          class="sp-reveal mt-4 text-[12.5px] text-[#6E6E73]"
        >
          {t.closeFine}
        </p>
      </div>
    </section>
  )
}
