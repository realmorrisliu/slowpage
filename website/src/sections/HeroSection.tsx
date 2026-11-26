import { createSignal, lazy, Loading, onSettled, Show } from 'solid-js'
import { GlassButton } from 'solid-liquid-glass'

import { FactLine } from '../components/FactLine'
import { t } from '../i18n'
import { useDownloadLink, useReleaseVersion } from '../release'

// The interactive window is the heaviest part of the page. It loads as its own
// chunk once it nears the viewport, so first paint and input never wait for it.
const HeroStage = lazy(() => import('./HeroStage'), { export: 'HeroStage' })

/** Same footprint as the loaded stage (window + mode dock), so nothing shifts when it arrives. */
function StagePlaceholder(props: { ref?: (el: HTMLDivElement) => void }) {
  return (
    <div id="preview" ref={props.ref} data-stage-placeholder="" class="mx-auto mt-14 max-w-[1120px]" aria-hidden="true">
      <div
        class="mx-auto h-[620px] rounded-[16px] bg-gradient-to-b from-[#EEEEF0] to-[#E9E9EC] shadow-[var(--sp-window-shadow)] ring-1 ring-black/[0.08]"
        style={{ transform: 'translate3d(0, 5px, 0) scale(0.993)' }}
      />
      <div class="sp-stage-dock-space" />
    </div>
  )
}

export function HeroSection() {
  const version = useReleaseVersion()
  const download = useDownloadLink()
  const [loadStage, setLoadStage] = createSignal(
    typeof window !== 'undefined' && window.location.hash === '#preview',
  )
  let placeholder: HTMLDivElement | undefined

  onSettled(() => {
    if (loadStage()) return
    if (!placeholder || typeof IntersectionObserver === 'undefined') {
      setLoadStage(true)
      return
    }
    const io = new IntersectionObserver(
      (entries) => {
        if (entries.some((entry) => entry.isIntersecting)) {
          setLoadStage(true)
          io.disconnect()
        }
      },
      { rootMargin: '300px 0px' },
    )
    io.observe(placeholder)
    const onHash = () => {
      if (window.location.hash === '#preview') setLoadStage(true)
    }
    window.addEventListener('hashchange', onHash)
    return () => {
      io.disconnect()
      window.removeEventListener('hashchange', onHash)
    }
  })

  return (
    <section id="top" class="bg-white px-5 pt-14 pb-20 sm:px-8 sm:pt-20 sm:pb-28">
      {/* Headline renders immediately, with no opacity gating, for an instant LCP. */}
      <div class="mx-auto flex max-w-4xl flex-col items-center text-center">
        <p class="sp-eyebrow">{t.heroEyebrow}</p>

        <h1 class="mt-3 text-[44px] leading-[1.06] font-semibold tracking-[-0.035em] text-[#1D1D1F] [text-wrap:balance] sm:text-[64px] lg:text-[76px]">
          {t.heroTitle}
        </h1>

        <p class="relative mt-6 max-w-2xl text-[17px] leading-[1.6] text-[#59595E] [text-wrap:pretty] sm:text-[20px]">
          {t.heroLeadBefore}{' '}
          <span tabindex="0" aria-describedby="alan-tip" class="sp-alan-term font-medium text-[#1D1D1F]">
            {t.heroAlan}
            <span id="alan-tip" role="tooltip" class="sp-alan-tip">
              {t.heroAlanTip}
            </span>
          </span>
          {t.heroLeadAfter}
        </p>

        <div class="mt-9 flex flex-wrap items-center justify-center gap-x-7 gap-y-4">
          <GlassButton
            material="tinted"
            size="lg"
            swell="subtle"
            backdrop="#top"
            class="bg-[#1D1D1F] px-7 text-white"
            href={download.href()}
            target={download.target()}
            rel="noopener noreferrer"
            data-track="download_click"
          >
            {t.heroDownload}
          </GlassButton>

          <a href="#preview" class="sp-link text-[17px]">
            {t.heroTry} ›
          </a>
        </div>

        <p class="sp-tabular mt-4 text-[12.5px] text-[#59595E] [text-wrap:balance]">
          <FactLine items={t.heroFine(version(), download.size())} />
        </p>
      </div>

      <Show when={loadStage()} fallback={<StagePlaceholder ref={(el) => (placeholder = el)} />}>
        <Loading fallback={<StagePlaceholder />}>
          <HeroStage />
        </Loading>
      </Show>
    </section>
  )
}
