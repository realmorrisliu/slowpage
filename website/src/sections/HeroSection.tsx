import { createMemo, createSignal, For, Show } from 'solid-js'
import { GlassButton, GlassTabs } from 'solid-liquid-glass'

import { BookCover, type CoverId } from '../components/BookCover'
import {
  IconArrowUp,
  IconBook,
  IconBookOpen,
  IconCheck,
  IconFile,
  IconFolder,
  IconGrid,
  IconLibrary,
  IconList,
  IconMaximize,
  IconNote,
  IconPlus,
  IconSearch,
  IconSettings,
  IconSidebarLeft,
  IconSidebarRight,
  IconSortDesc,
  SlowpageAvatar,
} from '../components/Icons'
import { createSwipePager, useScrollProgress } from '../utils/motion'

type ShellForm = 'full' | 'page' | 'immersive'

type BookPage = {
  heading: string
  meta?: string
  authors?: string
  paragraphs: string[]
  highlightIndex?: number
}

type ShelfBook = {
  id: CoverId
  title: string
  author: string
  kind: 'EPUB' | 'PDF'
  lang: 'en' | 'zh'
  progress: string
  percent?: number
  recentNote?: string
  marginUser: { author: string; time: string; text: string }
  marginAlan: string
  pages: BookPage[]
}

const RELEASE_URL = 'https://github.com/realmorrisliu/slowpage/releases/tag/v0.1.0-alpha.1'

const SHELF_BOOKS: ShelfBook[] = [
  {
    id: 'pride',
    title: 'Pride and Prejudice',
    author: 'Jane Austen',
    kind: 'EPUB',
    lang: 'en',
    progress: '第 3 章 · 舞会',
    percent: 37,
    recentNote: '今天 17:42',
    marginUser: {
      author: 'Morris',
      time: '刚才',
      text: '开篇像一句不容置疑的公理，紧接着落回门第与婚姻的盘算，反讽一下立住了。',
    },
    marginAlan:
      '奥斯汀先用 “universally acknowledged” 托举出庄重语气，再在第二页由 Bennet 夫妇的家常拌嘴把镜头拉回客厅。',
    pages: [
      {
        heading: 'Chapter I',
        highlightIndex: 2,
        paragraphs: [
          'It is a truth universally acknowledged, that a single man in possession of a good fortune, must be in want of a wife.',
          'However little known the feelings or views of such a man may be on his first entering a neighbourhood, this truth is so well fixed in the minds of the surrounding families, that he is considered the rightful property of some one or other of their daughters.',
          '“My dear Mr. Bennet,” said his lady to him one day, “have you heard that Netherfield Park is let at last?”',
        ],
      },
      {
        heading: 'Chapter I (cont.)',
        highlightIndex: 0,
        paragraphs: [
          'Mr. Bennet replied that he had not. “But it is,” returned she; “for Mrs. Long has just been here, and she told me all about it.”',
          'Mr. Bennet made no answer. “Do you not want to know who has taken it?” cried his wife impatiently. “You want to tell me, and I have no objection to hearing it.”',
        ],
      },
    ],
  },
  {
    id: 'three-body',
    title: '三体',
    author: '刘慈欣',
    kind: 'EPUB',
    lang: 'zh',
    progress: '第 2 部 · 黑暗森林',
    percent: 62,
    recentNote: '昨天',
    marginUser: {
      author: 'Morris',
      time: '昨天',
      text: '把宇宙文明抽象成两三条简单公理之后，冷峻感反而压过了任何宏大修辞。',
    },
    marginAlan:
      '这里与前文叶文洁在杨冬墓前的那段对话遥相呼应，所有推演都从“生存”与“猜疑链”这两个极简支点展开。',
    pages: [
      {
        heading: '黑暗森林 · 引子',
        highlightIndex: 1,
        paragraphs: [
          '宇宙就是一座黑暗森林，每个文明都是带枪的猎人，像幽灵般潜行于林间，轻轻拨开挡路的树枝，竭力不让脚步发出一点儿声音，连呼吸都必须小心翼翼。',
          '他必须小心，因为林中到处都有与他一样潜行的猎人。如果他发现了别的生命，能做的只有一件事：开枪消灭之。在这片森林中，他人就是地狱。',
        ],
      },
      {
        heading: '第 2 部 · 推演',
        highlightIndex: 0,
        paragraphs: [
          '任何暴露自己存在的生命都将很快被消灭，这就是宇宙文明的图景，这就是对费米悖论的解释。',
          '罗辑站在夜风里抬头望向星空，那些曾经像诗句般温柔的星座，此刻在他眼里变得安静而锋利，如同几万年前就架好的准星。',
        ],
      },
    ],
  },
  {
    id: 'taocp',
    title: 'The Art of Computer Programming',
    author: 'Donald E. Knuth',
    kind: 'PDF',
    lang: 'en',
    progress: '第 124 / 672 页',
    percent: 18,
    recentNote: '周一',
    marginUser: {
      author: 'Morris',
      time: '周一',
      text: '把排版与算法写成同一件严谨的事，页边距和数学推导一样让人安心。',
    },
    marginAlan:
      '第 1 卷第 1 章把“算法”定义为有限步内可终止的清晰过程，紧接着就用欧几里得最大公约数给出了第一个经典分析示例。',
    pages: [
      {
        heading: 'Volume 1 · Fundamental Algorithms',
        meta: 'Section 1.1 · Algorithms',
        authors: 'Donald Ervin Knuth · Stanford University',
        highlightIndex: 0,
        paragraphs: [
          'An algorithm must always terminate after a finite number of steps. A procedure that has all of the characteristics of an algorithm except that it possibly lacks finiteness may be called a computational method.',
          'Finiteness, definiteness, input, output, and effectiveness together form the five essential features that turn an informal recipe into a precise mathematical object.',
        ],
      },
      {
        heading: 'Algorithm E (Euclid’s Algorithm)',
        meta: 'Page 2 · 原版版面整页呈阅',
        highlightIndex: 0,
        paragraphs: [
          'Given two positive integers m and n, find their greatest common divisor, that is, the largest positive integer that evenly divides both m and n.',
          'Divide m by n and let r be the remainder. If r = 0, the algorithm terminates; n is the answer. Otherwise set m ← n, n ← r, and repeat.',
        ],
      },
    ],
  },
  {
    id: 'moby-dick',
    title: 'Moby-Dick',
    author: 'Herman Melville',
    kind: 'EPUB',
    lang: 'en',
    progress: '第 1 章 · Loomings',
    marginUser: {
      author: 'Morris',
      time: '上周',
      text: '“Call me Ishmael.” 一开口就把读者拽进带着海盐味儿的自白里。',
    },
    marginAlan:
      '紧接在简短呼告之后的，是一长串铺陈水手忧郁心境的排比句，节奏就像甲板下缓慢起伏的涌浪。',
    pages: [
      {
        heading: 'Chapter 1 · Loomings',
        highlightIndex: 0,
        paragraphs: [
          'Call me Ishmael. Some years ago—never mind how long precisely—having little or no money in my purse, and nothing particular to interest me on shore, I thought I would sail about a little and see the watery part of the world.',
          'It is a way I have of driving off the spleen and regulating the circulation. Whenever I find myself growing grim about the mouth; whenever it is a damp, drizzly November in my soul, then I account it high time to get to sea as soon as I can.',
        ],
      },
      {
        heading: 'Chapter 1 (cont.)',
        highlightIndex: 1,
        paragraphs: [
          'There now is your insular city of the Manhattoes, belted round by wharves as Indian isles by coral reefs—commerce surrounds it with her surf.',
          'Look at the crowds of water-gazers there. Circumambulate the city of a dreamy Sabbath afternoon, and you will find thousands upon thousands of mortal men fixed in ocean reveries.',
        ],
      },
    ],
  },
  {
    id: 'dawn-blossoms',
    title: '朝花夕拾',
    author: '鲁迅',
    kind: 'EPUB',
    lang: 'zh',
    progress: '范爱农 · 5/18 页',
    marginUser: {
      author: 'Morris',
      time: '刚才',
      text: '“眼球白多黑少，看人总像在渺视”，七八个字就把人物的神态刻在纸上了。',
    },
    marginAlan:
      '东京初见越显得冷峻与隔膜，后半篇回到绍兴酒楼共饮，才更衬出他的真性情与苍凉。',
    pages: [
      {
        heading: '范爱农',
        highlightIndex: 1,
        paragraphs: [
          '在东京的客店里，我们大抵一起来就看报。学生少，看报的也不多，所以报来得早，往往在十点钟左右。',
          '一天早晨，我正坐在楼下廊檐前看报，忽然听见楼梯上响起一阵沉重的脚步声，抬头一看，却是一个高大身材、长头发、眼球白多黑少的人，看人总像在渺视。',
        ],
      },
      {
        heading: '范爱农（续）',
        highlightIndex: 0,
        paragraphs: [
          '他便是范爱农，徐锡麟的学生。那时我们因为发电报的事情，彼此很不相容，直到后来回到故乡绍兴，才在酒楼上慢慢谈起旧事。',
          '此后他便常来找我，有时一起喝几杯黄酒，谈起报上的新闻，彼此都不再像从前那样剑拔弩张，只觉得世事流转得实在太快。',
        ],
      },
    ],
  },
]

const PEEK_BOOKS: Array<{ id: CoverId; title: string; author: string; targetIndex: number }> = [
  { id: 'nahan', title: '呐喊', author: '鲁迅', targetIndex: 4 },
  { id: 'walden', title: 'Walden', author: 'Henry David Thoreau', targetIndex: 3 },
  { id: 'honglou', title: '红楼梦', author: '曹雪芹', targetIndex: 4 },
  { id: 'middlemarch', title: 'Middlemarch', author: 'George Eliot', targetIndex: 0 },
  { id: 'weicheng', title: '围城', author: '钱锺书', targetIndex: 1 },
]

export function HeroSection() {
  const [heroReady] = createSignal(true)
  const [form, setForm] = createSignal<ShellForm>('full')
  const [selectedBook, setSelectedBook] = createSignal(0)
  const [heroPageIdx, setHeroPageIdx] = createSignal(0)
  const [sidebarShown, setSidebarShown] = createSignal(true)
  const [marginShown, setMarginShown] = createSignal(true)

  let windowRef: HTMLDivElement | undefined
  let sheetRef: HTMLDivElement | undefined

  const scrollProgress = useScrollProgress(() => windowRef)

  const activeBook = createMemo(() => SHELF_BOOKS[selectedBook()])
  const currentPage = createMemo(() => activeBook().pages[heroPageIdx()] ?? activeBook().pages[0])

  const pager = createSwipePager({
    pageIndex: heroPageIdx,
    pageCount: () => activeBook().pages.length,
    onTurn: (next) => setHeroPageIdx(next),
    width: () => sheetRef?.clientWidth || 460,
  })

  const openBook = (idx: number) => {
    setSelectedBook(idx)
    setHeroPageIdx(0)
    setForm('page')
  }

  const toggleImmersive = () => {
    setForm((f) => (f === 'immersive' ? 'page' : 'immersive'))
  }

  const isFull = () => form() === 'full'
  const isPage = () => form() === 'page'
  const isImmersive = () => form() === 'immersive'

  const windowScale = () => {
    const p = scrollProgress()
    return 0.976 + p * 0.024
  }

  const windowTranslateY = () => {
    const p = scrollProgress()
    return (1 - p) * 18
  }

  return (
    <section id="top" class="bg-white px-5 pt-14 pb-20 sm:px-8 sm:pt-20 sm:pb-28">
      {/* Studio Headline Block */}
      <div class="mx-auto flex max-w-4xl flex-col items-center text-center">
        <p
          data-shown={heroReady() ? 'true' : 'false'}
          style={{ '--sp-delay': '0ms' }}
          class="sp-reveal text-[15px] font-semibold tracking-[-0.01em] text-[#D2693C]"
        >
          专为 macOS 打造
        </p>

        <h1
          data-shown={heroReady() ? 'true' : 'false'}
          style={{ '--sp-delay': '60ms' }}
          class="sp-reveal mt-3 text-[44px] leading-[1.06] font-semibold tracking-[-0.035em] text-[#1D1D1F] [text-wrap:balance] sm:text-[64px] lg:text-[72px]"
        >
          安静如纸，<span class="text-[#86868B]">灵动如玻璃。</span>
        </h1>

        <p
          data-shown={heroReady() ? 'true' : 'false'}
          style={{ '--sp-delay': '120ms' }}
          class="sp-reveal mt-6 max-w-xl text-[17px] leading-[1.58] text-[#6E6E73] sm:text-[19px]"
        >
          打开书架，摊开纯白书页，把工具轻藏在边缘。像读一本印得很好的纸书，把灵感记在页边，需要时轻唤{' '}
          <span class="font-medium text-[#D2693C]">@alan</span> 陪你读完这一页。
        </p>

        <div
          data-shown={heroReady() ? 'true' : 'false'}
          style={{ '--sp-delay': '180ms' }}
          class="sp-reveal mt-9 flex flex-wrap items-center justify-center gap-3.5"
        >
          <GlassButton
            material="tinted"
            size="lg"
            swell="subtle"
            backdrop="#top"
            class="bg-[#1D1D1F] px-7 text-white"
            href={RELEASE_URL}
            target="_blank"
            rel="noopener noreferrer"
          >
            下载 macOS 版
          </GlassButton>

          <GlassButton size="lg" swell="subtle" backdrop="#top" class="px-7" href="#preview">
            在网页里试试
          </GlassButton>
        </div>

        <p
          data-shown={heroReady() ? 'true' : 'false'}
          style={{ '--sp-delay': '220ms' }}
          class="sp-reveal mt-4 text-[12px] text-[#86868B]"
        >
          macOS 14 Sonoma 及以上 · 支持 Apple Silicon · 免费开源（Alpha 1）
        </p>
      </div>

      {/* Product Stage — Authentic Slowpage Window Shell */}
      <div id="preview" class="mx-auto mt-14 max-w-[1120px]">
        <div
          ref={(el) => {
            windowRef = el
          }}
          style={{
            transform: `translate3d(0, ${windowTranslateY().toFixed(1)}px, 0) scale(${windowScale().toFixed(4)})`,
          }}
          class="will-change-transform"
        >
          <div
            class="relative mx-auto overflow-hidden rounded-[16px] bg-gradient-to-b from-[#EEEEF0] to-[#E9E9EC] shadow-[var(--sp-window-shadow)] ring-1 ring-black/[0.08] transition-[padding,border-radius] duration-200 ease-[cubic-bezier(0.37,0,0.63,1)]"
            style={{
              height: '620px',
              padding: isImmersive() ? '0px' : '6px',
            }}
          >
            {/* Floating Liquid Glass HUD in Reading & Immersive states */}
            <Show when={!isFull()}>
              <div
                class="pointer-events-none absolute inset-x-0 top-0 z-30 flex h-12 items-center justify-between px-4 transition-[opacity,transform,filter] duration-220 ease-[cubic-bezier(0.22,1,0.36,1)]"
              >
                <div class="pointer-events-auto flex items-center gap-2.5">
                  <div class="flex items-center gap-2" aria-hidden="true">
                    <span class="size-3 rounded-full bg-[#FF5F57] shadow-[inset_0_0_0_0.5px_rgba(0,0,0,0.15)]" />
                    <span class="size-3 rounded-full bg-[#FEBC2E] shadow-[inset_0_0_0_0.5px_rgba(0,0,0,0.15)]" />
                    <span class="size-3 rounded-full bg-[#28C840] shadow-[inset_0_0_0_0.5px_rgba(0,0,0,0.15)]" />
                  </div>
                  <GlassButton
                    size="sm"
                    swell="subtle"
                    backdrop="#top"
                    class="h-7 w-7 !px-0"
                    aria-label="返回书架"
                    title="返回书架"
                    onClick={() => setForm('full')}
                  >
                    <IconList class="size-3.5 text-[#55555B]" />
                  </GlassButton>
                  <span class="hidden text-[12px] font-medium text-[#5c544a] sm:inline">
                    {activeBook().title}
                  </span>
                </div>

                <div class="pointer-events-auto flex items-center gap-1.5">
                  <GlassButton
                    size="sm"
                    swell="subtle"
                    backdrop="#top"
                    class="h-7 w-7 !px-0"
                    aria-label={isImmersive() ? '退出沉浸阅读' : '进入沉浸阅读'}
                    title={isImmersive() ? '退出沉浸阅读 (Esc)' : '沉浸阅读 (⌃⌘I)'}
                    onClick={toggleImmersive}
                  >
                    <IconMaximize class="size-3.5 text-[#1D1D1F]" />
                  </GlassButton>
                  <Show when={!isImmersive()}>
                    <GlassButton
                      size="sm"
                      swell="subtle"
                      backdrop="#top"
                      class="h-7 w-7 !px-0"
                      aria-label={marginShown() ? '隐藏页边批注' : '显示页边批注'}
                      title={marginShown() ? '隐藏页边批注' : '显示页边批注'}
                      onClick={() => setMarginShown((v) => !v)}
                    >
                      <IconSidebarRight class="size-3.5 text-[#1D1D1F]" />
                    </GlassButton>
                  </Show>
                </div>
              </div>
            </Show>

            <div class="flex h-full w-full items-stretch">
              {/* Left Sidebar — continuously slides & fades out smoothly instead of popping */}
              <aside
                aria-hidden={!isFull() || !sidebarShown() ? 'true' : undefined}
                class={`hidden shrink-0 overflow-hidden transition-[width,opacity,transform] duration-[280ms] ease-[cubic-bezier(0.22,1,0.36,1)] md:flex ${
                  isFull() && sidebarShown()
                    ? 'w-[208px] translate-x-0 opacity-100'
                    : 'pointer-events-none w-0 -translate-x-4 opacity-0'
                }`}
              >
                <div class="flex h-full w-[208px] shrink-0 flex-col justify-between px-3 py-2.5 text-left">
                  <div>
                    {/* Traffic lights + sidebar toggle */}
                    <div class="mb-3 flex h-8 items-center justify-between px-1">
                      <div class="flex items-center gap-2" aria-hidden="true">
                        <span class="size-3 rounded-full bg-[#FF5F57] shadow-[inset_0_0_0_0.5px_rgba(0,0,0,0.15)]" />
                        <span class="size-3 rounded-full bg-[#FEBC2E] shadow-[inset_0_0_0_0.5px_rgba(0,0,0,0.15)]" />
                        <span class="size-3 rounded-full bg-[#28C840] shadow-[inset_0_0_0_0.5px_rgba(0,0,0,0.15)]" />
                      </div>
                      <button
                        type="button"
                        onClick={() => setSidebarShown(false)}
                        class="sp-pressable flex size-7 cursor-pointer items-center justify-center rounded-[7px] text-[#6E6E73] transition-colors hover:bg-black/[0.06]"
                        aria-label="收起侧栏"
                      >
                        <IconSidebarLeft class="size-4" />
                      </button>
                    </div>

                    {/* Search */}
                    <div class="mb-3.5 flex h-8 items-center gap-2 rounded-[8px] bg-black/[0.05] px-2.5 text-[12px] text-[#8E8E93]">
                      <IconSearch class="size-3.5 shrink-0 text-[#6E6E73]" />
                      <span class="truncate">搜索书名、作者、笔记</span>
                    </div>

                    {/* Groups */}
                    <div class="space-y-3 text-[13px]">
                      <div>
                        <div class="px-2 pb-1 text-[11px] font-semibold text-[#8E8E93]">书库</div>
                        <div class="flex items-center justify-between rounded-[7px] bg-black/[0.07] px-2 py-1.5 font-medium text-[#1D1D1F]">
                          <span class="flex items-center gap-2">
                            <IconLibrary class="size-3.5 text-[#1D1D1F]" />
                            全部书籍
                          </span>
                          <span class="sp-tabular text-[11px] text-[#6E6E73]">12</span>
                        </div>
                        <div class="mt-0.5 flex items-center justify-between rounded-[7px] px-2 py-1 text-[#3A3A3F]">
                          <span class="flex items-center gap-2">
                            <IconBookOpen class="size-3.5 text-[#6E6E73]" />
                            正在读
                          </span>
                          <span class="sp-tabular text-[11px] text-[#8E8E93]">3</span>
                        </div>
                        <div class="flex items-center justify-between rounded-[7px] px-2 py-1 text-[#3A3A3F]">
                          <span class="flex items-center gap-2">
                            <IconBook class="size-3.5 text-[#6E6E73]" />
                            想读
                          </span>
                          <span class="sp-tabular text-[11px] text-[#8E8E93]">4</span>
                        </div>
                        <div class="flex items-center justify-between rounded-[7px] px-2 py-1 text-[#3A3A3F]">
                          <span class="flex items-center gap-2">
                            <IconCheck class="size-3.5 text-[#6E6E73]" />
                            已读完
                          </span>
                          <span class="sp-tabular text-[11px] text-[#8E8E93]">5</span>
                        </div>
                        <div class="flex items-center justify-between rounded-[7px] px-2 py-1 text-[#3A3A3F]">
                          <span class="flex items-center gap-2">
                            <IconFile class="size-3.5 text-[#6E6E73]" />
                            PDF
                          </span>
                          <span class="sp-tabular text-[11px] text-[#8E8E93]">1</span>
                        </div>
                      </div>

                      <div>
                        <div class="px-2 pb-1 text-[11px] font-semibold text-[#8E8E93]">笔记</div>
                        <div class="flex items-center justify-between rounded-[7px] px-2 py-1 text-[#3A3A3F]">
                          <span class="flex items-center gap-2">
                            <IconNote class="size-3.5 text-[#6E6E73]" />
                            高亮与笔记
                          </span>
                          <span class="sp-tabular text-[11px] text-[#8E8E93]">86</span>
                        </div>
                      </div>

                      <div>
                        <div class="px-2 pb-1 text-[11px] font-semibold text-[#8E8E93]">书单</div>
                        <div class="flex items-center justify-between px-2 py-1 text-[#3A3A3F]">
                          <span class="flex items-center gap-2">
                            <span class="size-2 rounded-full bg-[#E0773A]" />
                            2026 年想读完
                          </span>
                          <span class="sp-tabular text-[11px] text-[#8E8E93]">4</span>
                        </div>
                        <div class="flex items-center justify-between px-2 py-1 text-[#3A3A3F]">
                          <span class="flex items-center gap-2">
                            <span class="size-2 rounded-full bg-[#5B8DEF]" />
                            鲁迅全集
                          </span>
                          <span class="sp-tabular text-[11px] text-[#8E8E93]">2</span>
                        </div>
                        <div class="flex items-center justify-between px-2 py-1 text-[#3A3A3F]">
                          <span class="flex items-center gap-2">
                            <span class="size-2 rounded-full bg-[#7D8C6A]" />
                            英文经典
                          </span>
                          <span class="sp-tabular text-[11px] text-[#8E8E93]">5</span>
                        </div>
                      </div>

                      <div>
                        <div class="px-2 pb-1 text-[11px] font-semibold text-[#8E8E93]">来源</div>
                        <div class="flex items-center justify-between px-2 py-1 text-[#3A3A3F]">
                          <span class="flex items-center gap-2">
                            <span class="inline-flex size-3.5 items-center justify-center rounded-[3px] bg-gradient-to-b from-[#FFA53D] to-[#F27A1A]" />
                            Apple Books
                          </span>
                          <span class="sp-tabular text-[11px] text-[#8E8E93]">5</span>
                        </div>
                        <div class="flex items-center justify-between px-2 py-1 text-[#3A3A3F]">
                          <span class="flex items-center gap-2">
                            <IconFolder class="size-3.5 text-[#6B5A4A]" />
                            calibre
                          </span>
                          <span class="sp-tabular text-[11px] text-[#8E8E93]">6</span>
                        </div>
                        <div class="flex items-center gap-2 px-2 py-1 text-[#6E6E73]">
                          <IconPlus class="size-3.5" />
                          <span>添加来源</span>
                        </div>
                      </div>
                    </div>
                  </div>

                  <div class="flex items-center gap-2 px-2 py-1 text-[13px] text-[#55555B]">
                    <IconSettings class="size-3.5 text-[#6E6E73]" />
                    <span>设置</span>
                  </div>
                </div>
              </aside>

              {/* Center Opaque #FFFFFF Sheet */}
              <div
                ref={(el) => {
                  sheetRef = el
                }}
                class={`sp-card relative flex min-w-0 flex-1 flex-col overflow-hidden transition-[border-radius,box-shadow] duration-200 ease-[cubic-bezier(0.37,0,0.63,1)] ${
                  isImmersive() ? 'rounded-none shadow-none' : 'rounded-[12px]'
                }`}
              >
                <Show
                  when={isFull()}
                  fallback={
                    /* Reading Page inside the card with 1:1 Pointer Drag + Edge-Click */
                    <div
                      {...pager.handlers}
                      class={`relative flex h-full flex-col items-center justify-between px-6 pt-12 pb-6 text-left select-none ${
                        pager.isDragging() ? 'cursor-grabbing' : 'cursor-grab'
                      }`}
                      style={{ 'touch-action': 'pan-y' }}
                    >
                      <button
                        type="button"
                        aria-label="上一页"
                        onClick={() => pager.triggerTurn(-1)}
                        class="absolute inset-y-0 left-0 z-10 w-1/5 cursor-w-resize bg-transparent"
                      />
                      <button
                        type="button"
                        aria-label="下一页"
                        onClick={() => pager.triggerTurn(1)}
                        class="absolute inset-y-0 right-0 z-10 w-1/5 cursor-e-resize bg-transparent"
                      />

                      <div
                        class="mx-auto flex w-full max-w-[460px] flex-1 flex-col justify-center will-change-transform"
                        style={{
                          transform: `translate3d(${pager.offsetX().toFixed(1)}px, 0, 0)`,
                        }}
                      >
                        <Show
                          when={activeBook().kind === 'EPUB'}
                          fallback={
                            /* Realistic Archival PDF Sheet Facsimile */
                            <div class="mx-auto flex h-[430px] w-full max-w-[340px] flex-col justify-between rounded-[6px] border border-black/8 bg-white px-7 py-6 shadow-[0_3px_14px_rgba(0,0,0,0.05)]">
                              <div>
                                <div class="flex items-center justify-between text-[10px] text-[#8E8E93]">
                                  <span>{currentPage().meta}</span>
                                  <span class="sp-tabular">p. {heroPageIdx() + 1}</span>
                                </div>
                                <h3 class="sp-reading-en mt-2.5 text-[15px] font-semibold leading-snug text-[#1D1D1F]">
                                  {currentPage().heading}
                                </h3>
                                <Show when={currentPage().authors}>
                                  <p class="mt-1 text-[10px] leading-relaxed text-[#6E6E73]">
                                    {currentPage().authors}
                                  </p>
                                </Show>
                                <div class="mt-3.5 space-y-2.5 border-t border-black/[0.06] pt-3.5">
                                  <For each={currentPage().paragraphs}>
                                    {(para, idx) => (
                                      <p
                                        class={`sp-reading-en text-justify text-[12px] leading-[1.55] ${
                                          idx() === currentPage().highlightIndex
                                            ? 'sp-highlight-amber bg-[#F0B267]/26'
                                            : ''
                                        }`}
                                      >
                                        {para}
                                      </p>
                                    )}
                                  </For>
                                </div>
                              </div>
                              <div class="sp-tabular flex items-center justify-between text-[10px] text-[#8E8E93]">
                                <span>{activeBook().author}</span>
                                <span>原版面整页呈阅</span>
                              </div>
                            </div>
                          }
                        >
                          <article
                            class={
                              activeBook().lang === 'zh'
                                ? 'sp-reading-zh text-[18px] leading-[1.72] sm:text-[19px]'
                                : 'sp-reading-en text-[18px] leading-[1.52] sm:text-[19px]'
                            }
                          >
                            <p class="mb-4 text-[12px] tracking-[0.04em] text-[#8E8E93]">{currentPage().heading}</p>
                            <For each={currentPage().paragraphs}>
                              {(para, idx) => (
                                <p class={`${idx() > 0 ? 'mt-3.5' : ''} text-justify indent-6`}>
                                  <Show when={idx() === currentPage().highlightIndex} fallback={para}>
                                    <span class="sp-highlight-amber bg-[#F0B267]/28">{para}</span>
                                  </Show>
                                </p>
                              )}
                            </For>
                          </article>
                        </Show>
                      </div>

                      {/* Divider-less tabular footer */}
                      <div class="sp-tabular relative z-20 flex w-full max-w-[460px] items-center justify-between text-[11px] text-[#3C3C43]/72">
                        <span>
                          {heroPageIdx() + 1}/{activeBook().pages.length} 页
                        </span>
                        <div class="flex items-center gap-3">
                          <button
                            type="button"
                            data-no-drag=""
                            onClick={() => pager.triggerTurn(-1)}
                            disabled={heroPageIdx() === 0}
                            class="cursor-pointer rounded px-1.5 py-0.5 text-[#55555B] hover:bg-black/[0.05] disabled:cursor-default disabled:opacity-35"
                          >
                            上一页
                          </button>
                          <button
                            type="button"
                            data-no-drag=""
                            onClick={() => pager.triggerTurn(1)}
                            disabled={heroPageIdx() >= activeBook().pages.length - 1}
                            class="cursor-pointer rounded px-1.5 py-0.5 text-[#55555B] hover:bg-black/[0.05] disabled:cursor-default disabled:opacity-35"
                          >
                            下一页
                          </button>
                          <span>本章还剩 {activeBook().pages.length - 1 - heroPageIdx()} 页</span>
                        </div>
                      </div>
                    </div>
                  }
                >
                  {/* Full-Form Shelf matching approved-full-form.png */}
                  <div class="flex h-full flex-col overflow-hidden px-6 pt-5 pb-0 text-left">
                    <div class="flex items-center justify-between">
                      <div class="flex items-center gap-2.5">
                        <Show when={!sidebarShown()}>
                          <button
                            type="button"
                            onClick={() => setSidebarShown(true)}
                            class="sp-pressable flex size-7 cursor-pointer items-center justify-center rounded-[7px] text-[#6E6E73] hover:bg-black/[0.05]"
                            aria-label="展开侧栏"
                          >
                            <IconSidebarLeft class="size-4" />
                          </button>
                        </Show>
                        <h3 class="text-[22px] font-bold tracking-[-0.015em] text-[#1D1D1F]">全部书籍</h3>
                        <span class="sp-tabular text-[13px] font-medium text-[#8E8E93]">12 本</span>
                      </div>
                      <div class="flex items-center gap-2 text-[12px] text-[#55555B]">
                        <span class="inline-flex items-center gap-1 rounded-[7px] bg-black/[0.04] px-2.5 py-1">
                          <IconSortDesc class="size-3.5" />
                          最近阅读
                        </span>
                        <span class="inline-flex size-7 items-center justify-center rounded-[7px] bg-black/[0.04]">
                          <IconGrid class="size-3.5" />
                        </span>
                      </div>
                    </div>

                    {/* 继续阅读 — with bespoke BookCover thumbs + progress bars */}
                    <div class="mt-4">
                      <div class="mb-2.5 text-[13px] font-semibold text-[#1D1D1F]">继续阅读</div>
                      <div class="grid gap-2.5 sm:grid-cols-3">
                        <For each={SHELF_BOOKS.slice(0, 3)}>
                          {(b, i) => (
                            <button
                              type="button"
                              onClick={() => openBook(i())}
                              class="sp-pressable flex cursor-pointer items-center gap-3 rounded-[10px] bg-[#F7F7F9] p-2.5 text-left transition-colors hover:bg-[#EFEFF2]"
                            >
                              <BookCover id={b.id} size="thumb" />
                              <div class="min-w-0 flex-1">
                                <div class="truncate text-[12px] font-semibold text-[#1D1D1F]">{b.title}</div>
                                <div class="mt-0.5 truncate text-[10.5px] text-[#8E8E93]">{b.progress}</div>
                                <div class="mt-1.5 flex items-center gap-1.5">
                                  <div class="h-[2.5px] flex-1 overflow-hidden rounded-full bg-black/[0.08]">
                                    <div
                                      class="h-full rounded-full bg-[#3A3A3F]"
                                      style={{ width: `${b.percent ?? 24}%` }}
                                    />
                                  </div>
                                  <span class="sp-tabular text-[10px] text-[#8E8E93]">{b.percent}%</span>
                                </div>
                              </div>
                            </button>
                          )}
                        </For>
                      </div>
                    </div>

                    {/* Row 1 Bespoke Editorial Covers */}
                    <div class="mt-5 flex-1">
                      <div class="mb-3 flex items-baseline gap-2">
                        <span class="text-[13px] font-semibold text-[#1D1D1F]">书架</span>
                        <span class="text-[11px] text-[#8E8E93]">按最近阅读</span>
                      </div>

                      <div class="grid grid-cols-2 gap-4 sm:grid-cols-3 lg:grid-cols-5">
                        <For each={SHELF_BOOKS}>
                          {(b, i) => (
                            <button
                              type="button"
                              onClick={() => openBook(i())}
                              class="sp-pressable group cursor-pointer text-left"
                            >
                              <BookCover id={b.id} size="grid" interactive />
                              <div class="mt-2 truncate text-[12px] font-medium text-[#1D1D1F]">{b.title}</div>
                              <div class="flex items-center justify-between text-[11px] text-[#8E8E93]">
                                <span class="truncate">{b.author}</span>
                                <Show when={b.percent}>
                                  <span class="sp-tabular ml-1 shrink-0">{b.percent}%</span>
                                </Show>
                              </div>
                            </button>
                          )}
                        </For>
                      </div>

                      {/* Subtle Row 2 shelf covers peeking cleanly at bottom edge like approved-full-form.png */}
                      <div class="mt-5 hidden grid-cols-5 gap-4 lg:grid" aria-hidden="true">
                        <For each={PEEK_BOOKS}>
                          {(p) => (
                            <button
                              type="button"
                              onClick={() => openBook(p.targetIndex)}
                              class="cursor-pointer opacity-90 transition-opacity hover:opacity-100"
                              tabindex={-1}
                            >
                              <BookCover id={p.id} size="grid" interactive />
                            </button>
                          )}
                        </For>
                      </div>
                    </div>
                  </div>
                </Show>
              </div>

              {/* Right Assistant Panel (Full form) — smooth width/opacity morph */}
              <aside
                aria-hidden={!isFull() ? 'true' : undefined}
                class={`hidden shrink-0 overflow-hidden transition-[width,opacity,transform] duration-[280ms] ease-[cubic-bezier(0.22,1,0.36,1)] lg:flex ${
                  isFull()
                    ? 'w-[268px] translate-x-0 opacity-100'
                    : 'pointer-events-none w-0 translate-x-4 opacity-0'
                }`}
              >
                <div class="flex h-full w-[268px] shrink-0 flex-col justify-between px-3 py-3 text-left">
                  <div class="flex items-center justify-between px-1">
                    <div class="flex items-center gap-2.5">
                      <SlowpageAvatar class="size-7" />
                      <div>
                        <div class="text-[13px] font-semibold text-[#1D1D1F]">Slowpage</div>
                        <div class="text-[11px] text-[#8E8E93]">你的阅读助手</div>
                      </div>
                    </div>
                  </div>

                  <div class="space-y-3 px-1 text-[11.5px] leading-[1.55]">
                    <div class="flex justify-end">
                      <div class="rounded-[12px] bg-white/85 px-3 py-1.5 text-[#1D1D1F] shadow-[0_0_0_0.5px_rgba(0,0,0,0.05)]">
                        帮我总结一下《傲慢与偏见》第三章
                      </div>
                    </div>

                    <div class="space-y-1.5 text-[#3A3A3F]">
                      <p class="font-medium text-[#1D1D1F]">第三章是梅里顿的舞会，也是全书矛盾的起点：</p>
                      <ul class="list-disc space-y-1 pl-4">
                        <li>宾利开朗随和，和简跳了两支舞，在场的人都喜欢他。</li>
                        <li>达西冷淡傲慢，当众说伊丽莎白“还过得去”，打动不了他。</li>
                        <li>伊丽莎白听见了，一笑置之，对达西的偏见就此埋下。</li>
                      </ul>
                    </div>

                    <div class="flex flex-wrap gap-1.5 pt-1 text-[10.5px] text-[#55555B]">
                      <button
                        type="button"
                        onClick={() => openBook(0)}
                        class="sp-pressable cursor-pointer rounded-full bg-white/75 px-2.5 py-1 shadow-[0_0_0_0.5px_rgba(0,0,0,0.06)] hover:bg-white"
                      >
                        跳到第三章
                      </button>
                      <button
                        type="button"
                        onClick={() => openBook(0)}
                        class="sp-pressable cursor-pointer rounded-full bg-white/75 px-2.5 py-1 shadow-[0_0_0_0.5px_rgba(0,0,0,0.06)] hover:bg-white"
                      >
                        存为笔记
                      </button>
                    </div>
                  </div>

                  <div class="flex flex-col gap-2 rounded-[14px] bg-white p-2.5 shadow-[0_0_0_0.5px_rgba(0,0,0,0.08),0_2px_10px_rgba(0,0,0,0.04)]">
                    <span class="px-0.5 text-[12px] text-[#A1A1A6]">问问 Slowpage…</span>
                    <div class="flex items-center justify-between">
                      <span class="inline-flex items-center gap-1 rounded-[6px] bg-black/[0.04] px-2 py-0.5 text-[10.5px] text-[#55555B]">
                        <IconBookOpen class="size-3" /> 傲慢与偏见
                      </span>
                      <span class="flex size-6 items-center justify-center rounded-full bg-[#D8D8DD] text-[#6E6E73]">
                        <IconArrowUp class="size-3.5" />
                      </span>
                    </div>
                  </div>
                </div>
              </aside>

              {/* Right Margin Notes Panel (Page form) — smooth width/opacity morph */}
              <aside
                aria-hidden={!isPage() || !marginShown() ? 'true' : undefined}
                class={`hidden shrink-0 overflow-hidden transition-[width,opacity,transform] duration-[280ms] ease-[cubic-bezier(0.22,1,0.36,1)] md:flex ${
                  isPage() && marginShown()
                    ? 'w-[228px] translate-x-0 opacity-100'
                    : 'pointer-events-none w-0 translate-x-4 opacity-0'
                }`}
              >
                <div class="flex h-full w-[228px] shrink-0 flex-col justify-between px-2.5 pt-12 pb-3 text-left">
                  <div class="space-y-2.5">
                    <div class="rounded-[12px] bg-white p-3 shadow-[0_0_0_0.5px_rgba(0,0,0,0.07)]">
                      <div class="flex items-center justify-between text-[11px] text-[#6E6E73]">
                        <span class="font-medium text-[#1D1D1F]">{activeBook().marginUser.author}</span>
                        <span>{activeBook().marginUser.time}</span>
                      </div>
                      <p class="mt-1 text-[12px] leading-snug text-[#3A3A3F]">
                        {activeBook().marginUser.text}
                      </p>
                    </div>

                    <div class="rounded-[12px] bg-white p-3 shadow-[0_0_0_0.5px_rgba(0,0,0,0.07)]">
                      <div class="flex items-center gap-1.5 text-[11px] font-semibold text-[#D2693C]">
                        <span>@alan</span>
                        <span class="font-normal text-[#8E8E93]">· 伴读回复</span>
                      </div>
                      <p class="mt-1 text-[12px] leading-snug text-[#3A3A3F]">
                        {activeBook().marginAlan}
                      </p>
                    </div>
                  </div>

                  <div class="flex h-[72px] flex-col justify-between rounded-[14px] bg-white p-2.5 shadow-[0_0_0_0.5px_rgba(0,0,0,0.08),0_2px_10px_rgba(0,0,0,0.05)]">
                    <span class="text-[12px] text-[#A1A1A6]">写下页边笔记，或 @alan…</span>
                    <div class="flex justify-end">
                      <span class="text-[11px] font-medium text-[#8E8E93]">按回车保存</span>
                    </div>
                  </div>
                </div>
              </aside>
            </div>
          </div>
        </div>

        {/* Floating Liquid Glass Mode Dock placed cleanly beneath the product window */}
        <div class="mt-6 flex flex-col items-center gap-2.5">
          <GlassTabs
            aria-label="Slowpage 视图模式"
            value={form()}
            onValueChange={(next) => setForm(next)}
            options={[
              { value: 'full', label: '书架' },
              { value: 'page', label: '阅读与批注' },
              { value: 'immersive', label: '沉浸阅读' },
            ]}
          />
          <p class="text-[12px] text-[#86868B]">
            点击书架上的封面即可翻开试读；阅读时可直接左右拖拽或轻点页缘翻页。
          </p>
        </div>
      </div>
    </section>
  )
}
