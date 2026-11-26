import { GlassButton } from 'solid-liquid-glass'

import { FactLine } from '../components/FactLine'
import { REPO_URL, t } from '../i18n'
import { useDownloadLink, useReleaseVersion } from '../release'
import { useReveal } from '../utils/motion'

export function ClosingSection() {
  let blockRef: HTMLDivElement | undefined
  const shown = useReveal(() => blockRef)
  const version = useReleaseVersion()
  const download = useDownloadLink()

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
          class="sp-reveal sp-h2 mt-7"
        >
          {t.closeTitle}
        </h2>

        <p
          data-shown={shown() ? 'true' : 'false'}
          style={{ '--sp-delay': '120ms' }}
          class="sp-reveal sp-lead mx-auto mt-4 max-w-lg"
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
            href={download.href()}
            target={download.target()}
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
          class="sp-reveal mt-4 text-[12.5px] text-[#59595E] [text-wrap:balance]"
        >
          <FactLine items={t.closeFine} />
        </p>
      </div>
    </section>
  )
}
