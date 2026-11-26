import { Match, Switch } from 'solid-js'

export type CoverId =
  | 'pride'
  | 'three-body'
  | 'taocp'
  | 'moby-dick'
  | 'dawn-blossoms'
  | 'nahan'
  | 'walden'
  | 'honglou'
  | 'middlemarch'
  | 'weicheng'

type BookCoverProps = {
  id: CoverId
  size?: 'grid' | 'thumb' | 'mini'
  interactive?: boolean
  class?: string
}

export function BookCover(props: BookCoverProps) {
  const sizeClass = () => {
    if (props.size === 'thumb') return 'h-12 w-8 rounded-[3px]'
    if (props.size === 'mini') return 'h-16 w-11 rounded-[3.5px]'
    return 'aspect-[2/3] w-full rounded-[4.5px]'
  }

  const hoverClass = () => (props.interactive ? 'sp-cover-hover' : '')
  const isCompact = () => props.size === 'thumb' || props.size === 'mini'

  return (
    <div
      aria-hidden="true"
      class={`sp-cover relative shrink-0 overflow-hidden select-none ${sizeClass()} ${hoverClass()} ${props.class ?? ''}`}
    >
      <Switch>
        {/* 1. Pride and Prejudice — muted sage with cream frame & leaf ornament */}
        <Match when={props.id === 'pride'}>
          <div class="flex h-full w-full flex-col items-center justify-between bg-[#6D7C62] p-[8%] text-[#F4F0E6]">
            <div class="flex h-full w-full flex-col items-center justify-between border border-[#F4F0E6]/35 px-[8%] py-[12%] text-center">
              <div class={isCompact() ? 'text-[5.5px] leading-tight italic' : 'font-serif text-[12px] leading-[1.15] italic'}>
                Pride and
                <br />
                Prejudice
              </div>
              <svg viewBox="0 0 16 16" class={isCompact() ? 'size-2' : 'size-3.5'} fill="none" stroke="currentColor" stroke-width="1">
                <path d="M8 2.5C5.5 5 5.5 10.5 8 13.5C10.5 10.5 10.5 5 8 2.5Z" />
                <line x1="8" y1="3" x2="8" y2="13.5" />
              </svg>
              <div class={isCompact() ? 'text-[4px] tracking-[0.12em] opacity-80' : 'text-[7px] tracking-[0.16em] opacity-80'}>
                JANE AUSTEN
              </div>
            </div>
          </div>
        </Match>

        {/* 2. 三体 — matte charcoal sky + warm terracotta sun */}
        <Match when={props.id === 'three-body'}>
          <div class="flex h-full w-full flex-col items-center justify-between bg-[#1B232B] px-[12%] py-[14%] text-center text-[#F3EDE2]">
            <div class="mt-[10%] flex items-center justify-center">
              <div
                class={isCompact() ? 'size-3.5 rounded-full' : 'size-9 rounded-full'}
                style={{
                  background: 'radial-gradient(circle at 38% 38%, #EA843B 0%, #D45F24 70%, #8A3714 100%)',
                }}
              />
            </div>
            <div>
              <div class={`sp-reading-zh font-medium tracking-[0.12em] text-[#F3EDE2] ${isCompact() ? 'text-[7px]' : 'text-[17px]'}`}>
                三体
              </div>
              <div class={`mt-0.5 text-[#F3EDE2]/65 ${isCompact() ? 'text-[4px]' : 'text-[7.5px] tracking-[0.18em]'}`}>
                刘慈欣
              </div>
            </div>
          </div>
        </Match>

        {/* 3. TAOCP — archival ivory with burgundy hairlines */}
        <Match when={props.id === 'taocp'}>
          <div class="flex h-full w-full flex-col justify-between bg-[#EFE8DB] px-[10%] py-[10%] text-[#2C2621]">
            <div>
              <div class="h-[1.5px] w-full bg-[#8E3B3B]" />
              <div class={`mt-1 text-center font-serif tracking-[0.14em] text-[#8E3B3B] ${isCompact() ? 'text-[3.5px]' : 'text-[6px]'}`}>
                VOLUME 1
              </div>
              <div class={`mt-[10%] text-center font-serif leading-[1.15] font-semibold ${isCompact() ? 'text-[4.5px]' : 'text-[10.5px]'}`}>
                The Art of Computer Programming
              </div>
              {!isCompact() && (
                <div class="mt-1.5 text-center font-serif text-[7px] italic text-[#6E6359]">
                  Fundamental Algorithms
                </div>
              )}
            </div>
            <div>
              <div class="mb-1 h-[1px] w-full bg-[#8E3B3B]/60" />
              <div class={`text-center font-serif tracking-[0.08em] text-[#5C5147] ${isCompact() ? 'text-[3.5px]' : 'text-[6.5px]'}`}>
                DONALD E. KNUTH
              </div>
            </div>
          </div>
        </Match>

        {/* 4. Moby-Dick — deep ocean navy with SVG waves */}
        <Match when={props.id === 'moby-dick'}>
          <div class="relative flex h-full w-full flex-col justify-between bg-[#1D324A] px-[10%] py-[14%] text-center text-[#E8EEF5]">
            <div>
              <div class={`font-serif font-semibold ${isCompact() ? 'text-[5.5px]' : 'text-[13px]'}`}>
                Moby-Dick
              </div>
              {!isCompact() && (
                <div class="mt-0.5 font-serif text-[7.5px] italic text-[#E8EEF5]/70">
                  or, The Whale
                </div>
              )}
            </div>

            <svg viewBox="0 0 80 28" class="w-full text-[#E8EEF5]/30" fill="none" stroke="currentColor" stroke-width="1.2">
              <path d="M0 6 Q 10 2, 20 6 T 40 6 T 60 6 T 80 6" />
              <path d="M0 14 Q 10 10, 20 14 T 40 14 T 60 14 T 80 14" />
              <path d="M0 22 Q 10 18, 20 22 T 40 22 T 60 22 T 80 22" />
            </svg>

            <div class={isCompact() ? 'text-[3.5px] opacity-75' : 'text-[6.5px] tracking-[0.14em] opacity-75'}>
              HERMAN MELVILLE
            </div>
          </div>
        </Match>

        {/* 5. 朝花夕拾 — terracotta vermilion + cream seal + vertical Songti */}
        <Match when={props.id === 'dawn-blossoms'}>
          <div class="relative flex h-full w-full flex-col justify-between bg-[#B25436] p-[11%] text-[#F9F2E8]">
            <div class="flex items-start justify-between">
              <div class={`mt-[18%] sp-reading-zh leading-none text-[#F9F2E8]/80 [writing-mode:vertical-rl] ${isCompact() ? 'text-[4px]' : 'text-[7px] tracking-[0.2em]'}`}>
                鲁迅 著
              </div>
              <div class="flex flex-col items-end gap-[6px]">
                <span class={`rounded-[1px] bg-[#E8C69A] ${isCompact() ? 'size-1.5' : 'size-3'}`} />
                <div class={`sp-reading-zh font-medium text-[#F9F2E8] [writing-mode:vertical-rl] ${isCompact() ? 'text-[6.5px]' : 'text-[15px] tracking-[0.16em]'}`}>
                  朝花夕拾
                </div>
              </div>
            </div>
          </div>
        </Match>

        {/* 6. 呐喊 — deep black + vermilion vertical Songti */}
        <Match when={props.id === 'nahan'}>
          <div class="flex h-full w-full flex-col items-center justify-center bg-[#18181A] px-[10%] text-center text-[#DC4C38]">
            <div class={`sp-reading-zh font-semibold tracking-[0.16em] text-[#DC4C38] ${isCompact() ? 'text-[7px]' : 'text-[18px]'}`}>
              呐喊
            </div>
            <div class={`mt-2 text-[#E5E5EA]/65 ${isCompact() ? 'text-[4px]' : 'text-[7.5px] tracking-[0.15em]'}`}>
              鲁迅
            </div>
          </div>
        </Match>

        {/* 7. Walden — deep pine green */}
        <Match when={props.id === 'walden'}>
          <div class="flex h-full w-full flex-col items-center justify-between bg-[#2D4838] px-[10%] py-[14%] text-center text-[#E8EFE7]">
            <div class={`font-serif italic ${isCompact() ? 'text-[6px]' : 'text-[13px]'}`}>Walden</div>
            <div class={isCompact() ? 'text-[3.5px] opacity-75' : 'text-[6.5px] tracking-[0.14em] opacity-75'}>
              THOREAU
            </div>
          </div>
        </Match>

        {/* 8. 红楼梦 — deep crimson + cream square */}
        <Match when={props.id === 'honglou'}>
          <div class="flex h-full w-full flex-col justify-between bg-[#7C2727] p-[12%] text-[#F7ECE3]">
            <div class="flex items-start justify-end gap-1.5">
              <div class={`sp-reading-zh text-[#F7ECE3] [writing-mode:vertical-rl] ${isCompact() ? 'text-[6px]' : 'text-[13px] tracking-[0.15em]'}`}>
                红楼梦
              </div>
              <span class="size-2.5 rounded-[1px] bg-[#E8C69A]" />
            </div>
          </div>
        </Match>

        {/* 9. Middlemarch — warm ochre gold */}
        <Match when={props.id === 'middlemarch'}>
          <div class="flex h-full w-full flex-col items-center justify-between bg-[#C69C45] px-[10%] py-[14%] text-center text-[#241C0C]">
            <div class="w-full border-t border-black/30 pt-1 text-[6px] tracking-[0.12em]">GEORGE ELIOT</div>
            <div class={`font-serif font-semibold ${isCompact() ? 'text-[5px]' : 'text-[11px]'}`}>Middlemarch</div>
            <div class="w-full border-b border-black/30 pb-1 text-[6px] opacity-75">SLOWPAGE</div>
          </div>
        </Match>

        {/* 10. 围城 — cool architectural slate frame */}
        <Match when={props.id === 'weicheng'}>
          <div class="flex h-full w-full items-center justify-center bg-[#D7DEE4] p-[14%] text-[#2F3E4E]">
            <div class="flex h-full w-full flex-col items-center justify-center border border-[#2F3E4E]/35">
              <div class={`sp-reading-zh font-medium text-[#2F3E4E] ${isCompact() ? 'text-[6px]' : 'text-[14px] tracking-[0.2em]'}`}>
                围城
              </div>
            </div>
          </div>
        </Match>
      </Switch>
    </div>
  )
}
