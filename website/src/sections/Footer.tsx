import { ALAN_URL, ISSUES_URL, M3_URL, RELEASES_URL, REPO_URL, t } from '../i18n'

const linkClass = 'transition-colors hover:text-[#1D1D1F]'

export function Footer() {
  return (
    <footer class="bg-[#F5F5F7] px-5 pb-12 sm:px-8">
      <div class="mx-auto max-w-[1120px] border-t border-black/[0.08] pt-6 text-left">
        <div class="flex flex-col gap-4 pb-4 sm:flex-row sm:items-center sm:justify-between">
          <div class="flex flex-wrap items-center gap-x-2 gap-y-1 text-[12.5px] text-[#59595E]">
            <span class="flex items-center gap-2 whitespace-nowrap">
              <img src="/slowpage-icon.png" alt="" width="18" height="18" class="size-4.5" />
              <span class="font-semibold text-[#1D1D1F]">Slowpage</span>
            </span>
            <span aria-hidden="true">·</span>
            <span class="whitespace-nowrap">{t.footerTagline}</span>
            <span aria-hidden="true">·</span>
            <a href={ALAN_URL} target="_blank" rel="noopener noreferrer" class={`whitespace-nowrap ${linkClass}`}>
              {t.poweredByAlan}
            </a>
          </div>

          <div class="flex flex-wrap items-center gap-5 text-[12.5px] text-[#55555B]">
            <a href={RELEASES_URL} target="_blank" rel="noopener noreferrer" data-track="download_click" class={linkClass}>
              {t.footerDownload}
            </a>
            <a href={REPO_URL} target="_blank" rel="noopener noreferrer" data-track="github_click" class={linkClass}>
              GitHub
            </a>
            <a href={ISSUES_URL} target="_blank" rel="noopener noreferrer" class={linkClass}>
              {t.footerFeedback}
            </a>
            <a href={`${REPO_URL}/blob/main/LICENSE-MIT`} target="_blank" rel="noopener noreferrer" class={linkClass}>
              {t.footerLicense}
            </a>
          </div>
        </div>

        <div class="flex flex-col gap-2 border-t border-black/[0.05] pt-4 text-[11.5px] leading-relaxed text-[#59595E] sm:flex-row sm:items-start sm:justify-between">
          <div class="max-w-[640px]">
            <p class="m-0">{t.footerPrivacy}</p>
            <p class="m-0 mt-1">
              {t.footerCredits}{' '}
              <a
                href="https://github.com/realmorrisliu/solid-liquid-glass"
                target="_blank"
                rel="noopener noreferrer"
                class="underline decoration-black/20 underline-offset-2 hover:text-[#1D1D1F]"
              >
                solid-liquid-glass
              </a>{' '}
              (
              <a
                href="https://github.com/bilalgursen/liquid-glass"
                target="_blank"
                rel="noopener noreferrer"
                class="underline decoration-black/20 underline-offset-2 hover:text-[#1D1D1F]"
              >
                bilalgursen/liquid-glass
              </a>
              )
            </p>
          </div>
          <nav aria-label={t.footerFamily} class="flex items-center gap-3 whitespace-nowrap">
            <span>{t.footerFamily}</span>
            <a href={ALAN_URL} target="_blank" rel="noopener noreferrer" class={`text-[#55555B] ${linkClass}`}>
              Alan
            </a>
            <a href={M3_URL} target="_blank" rel="noopener noreferrer" class={`text-[#55555B] ${linkClass}`}>
              M3
            </a>
          </nav>
        </div>
      </div>
    </footer>
  )
}
