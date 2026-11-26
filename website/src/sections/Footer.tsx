const RELEASE_URL = 'https://github.com/realmorrisliu/slowpage/releases/tag/v0.1.0-alpha.1'
const REPO_URL = 'https://github.com/realmorrisliu/slowpage'
const ISSUES_URL = 'https://github.com/realmorrisliu/slowpage/issues'

export function Footer() {
  return (
    <footer class="bg-[#F5F5F7] px-5 pb-12 sm:px-8">
      <div class="mx-auto max-w-[1120px] border-t border-black/[0.08] pt-6 text-left">
        <div class="flex flex-col gap-4 pb-4 sm:flex-row sm:items-center sm:justify-between">
          <div class="flex items-center gap-2 text-[12.5px]">
            <img src="/slowpage-icon.png" alt="" class="size-4.5" />
            <span class="font-semibold text-[#1D1D1F]">Slowpage</span>
            <span class="text-[#86868B]">· 为 macOS 打造的极简单页阅读器</span>
          </div>

          <div class="flex flex-wrap items-center gap-5 text-[12.5px] text-[#55555B]">
            <a
              href={RELEASE_URL}
              target="_blank"
              rel="noopener noreferrer"
              class="transition-colors hover:text-[#1D1D1F]"
            >
              下载 Alpha 1
            </a>
            <a
              href={REPO_URL}
              target="_blank"
              rel="noopener noreferrer"
              class="transition-colors hover:text-[#1D1D1F]"
            >
              GitHub
            </a>
            <a
              href={ISSUES_URL}
              target="_blank"
              rel="noopener noreferrer"
              class="transition-colors hover:text-[#1D1D1F]"
            >
              反馈建议
            </a>
            <a
              href={`${REPO_URL}/blob/main/LICENSE-MIT`}
              target="_blank"
              rel="noopener noreferrer"
              class="transition-colors hover:text-[#1D1D1F]"
            >
              开源许可
            </a>
          </div>
        </div>

        <div class="text-[11px] leading-relaxed text-[#86868B]">
          <span>MIT OR Apache-2.0 · 图标来自 IconMCP (MIT) · 网页液态玻璃组件开源于 </span>
          <a
            href="https://github.com/realmorrisliu/solid-liquid-glass"
            target="_blank"
            rel="noopener noreferrer"
            class="underline decoration-black/20 underline-offset-2 hover:text-[#1D1D1F]"
          >
            solid-liquid-glass
          </a>
          <span>（致谢 </span>
          <a
            href="https://github.com/bilalgursen/liquid-glass"
            target="_blank"
            rel="noopener noreferrer"
            class="underline decoration-black/20 underline-offset-2 hover:text-[#1D1D1F]"
          >
            bilalgursen/liquid-glass
          </a>
          <span>）</span>
        </div>
      </div>
    </footer>
  )
}
