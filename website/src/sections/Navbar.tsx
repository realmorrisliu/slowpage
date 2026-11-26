import { GlassButton } from 'solid-liquid-glass'

import { RELEASES_URL, REPO_URL, t } from '../i18n'
import { useScrolled } from '../utils/motion'

export function Navbar() {
  const scrolled = useScrolled(10)

  return (
    <header
      class={`sticky top-0 z-40 h-[52px] border-b transition-[background-color,border-color,backdrop-filter] duration-200 ease-out ${
        scrolled()
          ? 'border-black/[0.06] bg-white/78 backdrop-blur-2xl backdrop-saturate-180'
          : 'border-transparent bg-white/0'
      }`}
    >
      <nav
        aria-label={t.navLabel}
        class="mx-auto flex h-full max-w-[1120px] items-center justify-between px-5 sm:px-8"
      >
        <a
          href="#top"
          class="sp-pressable flex items-center gap-2 text-[15px] font-semibold tracking-[-0.015em] text-[#1D1D1F]"
        >
          <img
            src="/slowpage-icon.png"
            alt=""
            width="24"
            height="24"
            class="size-6 drop-shadow-[0_1px_2px_rgba(0,0,0,0.1)]"
          />
          <span>Slowpage</span>
        </a>

        <div class="hidden items-center gap-8 text-[12.5px] font-normal text-[#424245] md:flex">
          <a href="#typography" class="transition-colors duration-150 hover:text-[#1D1D1F]">
            {t.navTypography}
          </a>
          <a href="#marginalia" class="transition-colors duration-150 hover:text-[#1D1D1F]">
            {t.navMarginalia}
          </a>
          <a href="#library" class="transition-colors duration-150 hover:text-[#1D1D1F]">
            {t.navLibrary}
          </a>
        </div>

        <div class="flex items-center gap-5 text-[12.5px] text-[#424245]">
          <a
            href={t.navOtherLocaleHref}
            hreflang={t.navOtherLocaleHref === '/' ? 'zh-CN' : 'en'}
            class="transition-colors duration-150 hover:text-[#1D1D1F]"
          >
            {t.navOtherLocale}
          </a>
          <a
            href={REPO_URL}
            target="_blank"
            rel="noopener noreferrer"
            data-track="github_click"
            class="hidden transition-colors duration-150 hover:text-[#1D1D1F] sm:inline"
          >
            GitHub
          </a>
          <GlassButton
            size="sm"
            material="tinted"
            swell="subtle"
            class="h-7 bg-[#1D1D1F] px-3.5 text-[12px] font-medium text-white"
            href={RELEASES_URL}
            target="_blank"
            rel="noopener noreferrer"
            data-track="download_click"
          >
            {t.navDownload}
          </GlassButton>
        </div>
      </nav>
    </header>
  )
}
