import { createEffect, createMemo, createSignal, For, onSettled, Show } from 'solid-js'

import { ALAN_URL, locale, t } from '../i18n'
import {
  GlassMenu,
  GlassMenuContent,
  GlassMenuGroup,
  GlassMenuItem,
  GlassMenuLabel,
  GlassMenuRadioGroup,
  GlassMenuRadioItem,
  GlassMenuSeparator,
  GlassMenuTrigger,
  GlassPopover,
  GlassPopoverClose,
  GlassPopoverContent,
  GlassPopoverDescription,
  GlassPopoverTitle,
  GlassPopoverTrigger,
} from 'solid-liquid-glass'

import { IconArrowUp, IconBookmark, IconEdit, IconNote } from '../components/Icons'
import { prefersReducedMotion, useReveal } from '../utils/motion'

type AnchorId = 'n1' | 'n2'
type FilterMode = 'all' | 'mine' | 'alan'

type NoteItem = {
  id: string
  anchorId: AnchorId
  author: string
  time: string
  body: string
  reply?: string
}

type ConnectorCoords = {
  x1: number
  y1: number
  elbowX: number
  x2: number
  y2: number
  length: number
}

const isEn = locale === 'en'

const INITIAL_NOTES: NoteItem[] = isEn
  ? [
      {
        id: 'n1',
        anchorId: 'n1',
        author: 'Morris',
        time: 'Line 1',
        body: 'The contrast between a solemn universal axiom and plain matchmaking math sets the ironic tone in a single sentence.',
        reply:
          '@alan: “Universally acknowledged” mirrors “rightful property,” dressing gentry marriage economics up as natural law.',
      },
      {
        id: 'n2',
        anchorId: 'n2',
        author: '@alan',
        time: 'Inline note',
        body: 'The dialogue jumps straight to letting Netherfield Park — pair this introduction with the Chapter 3 assembly.',
      },
    ]
  : [
      {
        id: 'n1',
        anchorId: 'n1',
        author: 'Morris',
        time: '第 1 句',
        body: '全称真理的庄重句式与后半句婚姻算盘的反差，一开篇就定下了整本书的反讽基调。',
        reply: '@alan：句首 “universally acknowledged” 与结尾 “rightful property” 呼应，把乡绅阶层的世俗规则写成仿佛自然法则。',
      },
      {
        id: 'n2',
        anchorId: 'n2',
        author: '@alan',
        time: '行内伴读',
        body: '对话直接切入 Netherfield Park 出租，建议把人物出场后的财产线和第 3 章舞会并读。',
      },
    ]

const QUICK_PROMPTS = isEn
  ? [
      {
        question: 'Where does the irony in this line come from?',
        anchorId: 'n1' as AnchorId,
        answer:
          '@alan: It opens in the voice of a philosophical maxim (“universally acknowledged”) only to pivot into families treating a bachelor as “rightful property” — the drop in register creates the wit.',
      },
      {
        question: 'Who appears in this chapter?',
        anchorId: 'n2' as AnchorId,
        answer:
          '@alan: Only Mr. and Mrs. Bennet appear in person; their five daughters and the newly arrived Mr. Bingley are all introduced through their banter.',
      },
    ]
  : [
      {
        question: '这句话的反讽妙在哪里？',
        anchorId: 'n1' as AnchorId,
        answer:
          '@alan：句首模仿哲学公理的口吻（universally acknowledged），结尾却突然转向乡绅家庭把单身贵族当作女儿“应得财产”的精打细算，这种落差构成了轻巧的讽刺。',
      },
      {
        question: '这一章主要出场了谁？',
        anchorId: 'n2' as AnchorId,
        answer:
          '@alan：这一章只有 Bennet 夫妇二人在客厅对话，五个女儿和邻镇新搬来的 Bingley 先生都在对话里先声夺人地被提起。',
      },
    ]

export function MarginaliaSection() {
  const [notes, setNotes] = createSignal<NoteItem[]>(INITIAL_NOTES)
  const [filter, setFilter] = createSignal<FilterMode>('all')
  const [selectedId, setSelectedId] = createSignal<string>('n1')
  const [hoveredId, setHoveredId] = createSignal<string | null>(null)
  const [editingId, setEditingId] = createSignal<string | null>(null)
  const [editDraft, setEditDraft] = createSignal('')
  const [draftNote, setDraftNote] = createSignal('')
  const [savedNotice, setSavedNotice] = createSignal('')
  const [connectors, setConnectors] = createSignal<Record<string, ConnectorCoords>>({})

  let headerRef: HTMLDivElement | undefined
  let gridEl: HTMLDivElement | undefined
  let paperEl: HTMLDivElement | undefined
  const anchorRefs: Partial<Record<AnchorId, HTMLSpanElement | undefined>> = {}
  const cardRefs: Record<string, HTMLDivElement | undefined> = {}
  const streamCancelFns: Array<() => void> = []

  const headerShown = useReveal(() => headerRef)

  const visibleNotes = createMemo(() => {
    const mode = filter()
    return notes().filter((note) => {
      if (mode === 'mine') return note.author !== '@alan'
      if (mode === 'alan') return note.author === '@alan' || Boolean(note.reply)
      return true
    })
  })

  const selectedAnchor = createMemo<AnchorId>(() => {
    const found = notes().find((n) => n.id === selectedId())
    return found?.anchorId ?? 'n1'
  })

  const streamReplyIntoNote = (noteId: string, fullReply: string) => {
    if (prefersReducedMotion()) {
      setNotes((prev) => prev.map((n) => (n.id === noteId ? { ...n, reply: fullReply } : n)))
      return
    }
    let raf = 0
    let start = 0
    const charsPerSec = 58
    let cancelled = false

    const cancel = () => {
      cancelled = true
      if (raf) cancelAnimationFrame(raf)
    }
    streamCancelFns.push(cancel)

    const tick = (now: number) => {
      if (cancelled) return
      if (!start) start = now
      const elapsedSec = (now - start) / 1000
      const count = Math.min(fullReply.length, Math.max(4, Math.floor(elapsedSec * charsPerSec)))
      const slice = fullReply.slice(0, count)
      setNotes((prev) => prev.map((n) => (n.id === noteId ? { ...n, reply: slice } : n)))
      if (count < fullReply.length) {
        raf = requestAnimationFrame(tick)
      } else {
        setSavedNotice(isEn ? '@alan replied in the margin' : '@alan 已回复到页边')
      }
    }
    raf = requestAnimationFrame(tick)
  }

  const measureConnectors = () => {
    if (!gridEl || !paperEl) return
    const gridRect = gridEl.getBoundingClientRect()
    const paperRect = paperEl.getBoundingClientRect()
    const next: Record<string, ConnectorCoords> = {}
    for (const note of visibleNotes()) {
      const anchor = anchorRefs[note.anchorId]
      const card = cardRefs[note.id]
      if (!anchor || !card) continue
      const firstLine = anchor.getClientRects()[0] ?? anchor.getBoundingClientRect()
      const cardRect = card.getBoundingClientRect()
      const x1 = firstLine.right - gridRect.left + 6
      const y1 = firstLine.top + firstLine.height / 2 - gridRect.top
      const x2 = cardRect.left - gridRect.left
      const y2 = cardRect.top + 22 - gridRect.top
      const elbowX = paperRect.right - gridRect.left + 8
      const length = Math.abs(elbowX - x1) + Math.abs(y2 - y1) + Math.abs(x2 - elbowX)
      next[note.id] = { x1, y1, elbowX, x2, y2, length }
    }
    setConnectors(next)
  }

  onSettled(() => {
    measureConnectors()
    const ro = new ResizeObserver(measureConnectors)
    if (gridEl) ro.observe(gridEl)
    window.addEventListener('resize', measureConnectors, { passive: true })
    return () => {
      ro.disconnect()
      window.removeEventListener('resize', measureConnectors)
      for (const stop of streamCancelFns) stop()
    }
  })

  createEffect(
    () => ({
      sel: selectedId(),
      ed: editingId(),
      f: filter(),
      len: notes().length,
      textSum: notes()
        .map((n) => `${n.body}|${n.reply ?? ''}`)
        .join('||'),
    }),
    () => {
      requestAnimationFrame(measureConnectors)
    },
  )

  const connectorAlpha = (id: string) => {
    if (selectedId() === id) return 0.72
    if (hoveredId() === id) return 0.28
    return 0
  }

  const selectAnchorSentence = (anchorId: AnchorId) => {
    const match = visibleNotes().find((n) => n.anchorId === anchorId)
    if (match) {
      setSelectedId(match.id)
      return
    }
    setFilter('all')
    const anyMatch = notes().find((n) => n.anchorId === anchorId)
    if (anyMatch) setSelectedId(anyMatch.id)
  }

  const onHighlightKeyDown = (e: KeyboardEvent, anchorId: AnchorId) => {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault()
      selectAnchorSentence(anchorId)
    }
  }

  const askQuickAlan = (item: (typeof QUICK_PROMPTS)[number]) => {
    const id = `alan-${Date.now()}`
    const newNote: NoteItem = {
      id,
      anchorId: item.anchorId,
      author: isEn ? 'Me' : '我',
      time: isEn ? 'Just now' : '刚才',
      body: `@alan ${item.question}`,
      reply: isEn ? '@alan: …' : '@alan：…',
    }
    setFilter('all')
    setNotes((prev) => [...prev, newNote])
    setSelectedId(id)
    setSavedNotice(isEn ? '@alan is writing in the margin…' : '@alan 正在页边书写…')
    streamReplyIntoNote(id, item.answer)
  }

  const startEditing = (note: NoteItem) => {
    setSelectedId(note.id)
    if (editingId() === note.id) {
      setEditingId(null)
      return
    }
    setEditDraft(note.body)
    setEditingId(note.id)
  }

  const saveEdit = (noteId: string) => {
    const trimmed = editDraft().trim()
    if (!trimmed) return
    setNotes((prev) =>
      prev.map((n) => (n.id === noteId ? { ...n, body: trimmed, time: isEn ? 'Edited' : '已编辑' } : n)),
    )
    setEditingId(null)
    setSavedNotice(isEn ? 'Margin note updated' : '已更新页边笔记')
  }

  const submitNewNote = (e: SubmitEvent) => {
    e.preventDefault()
    const trimmed = draftNote().trim()
    if (!trimmed) {
      setSavedNotice(isEn ? 'Select a line or write a thought first' : '先选中一句原文，或写下你的想法')
      return
    }
    const anchor = selectedAnchor()
    const id = `note-${Date.now()}`
    const mentionsAlan = trimmed.includes('@alan')
    const newItem: NoteItem = {
      id,
      anchorId: anchor,
      author: isEn ? 'Me' : '我',
      time: isEn ? 'Just now' : '刚才',
      body: trimmed,
      reply: mentionsAlan ? (isEn ? '@alan: …' : '@alan：…') : undefined,
    }
    setFilter('all')
    setNotes((prev) => [...prev, newItem])
    setSelectedId(id)
    setDraftNote('')
    if (mentionsAlan) {
      setSavedNotice(isEn ? '@alan is writing in the margin…' : '@alan 正在页边书写…')
      const reply = isEn
        ? anchor === 'n1'
          ? '@alan: The deadpan rational tone applied to worldly marriage calculations gives the opening its quiet humor.'
          : '@alan: Mrs. Bennet rushes ahead while Mr. Bennet hangs back — the contrast in tempo sketches both characters at once.'
        : anchor === 'n1'
          ? '@alan：第一句用最克制的理性口吻写最世俗的婚姻算计，整段话都不露声色，幽默全在对比里。'
          : '@alan：Mrs. Bennet 急切推进话题，Mr. Bennet 却故意慢半拍回应，一急一缓立刻勾勒出两人的性格。'
      streamReplyIntoNote(id, reply)
    } else {
      setSavedNotice(isEn ? 'Saved in margin' : '已添加到页边')
    }
  }

  return (
    <section id="marginalia" class="bg-white px-5 py-24 sm:px-8 sm:py-32">
      <div class="mx-auto max-w-[1120px]">
        {/* Left-aligned header: title on the left, lead and Alan credit on the right */}
        <div
          ref={(el) => {
            headerRef = el
          }}
          class="mx-auto grid max-w-[1040px] gap-6 text-left lg:grid-cols-[1.1fr_0.9fr] lg:items-end lg:gap-16"
        >
          <div>
            <p
              data-shown={headerShown() ? 'true' : 'false'}
              style={{ '--sp-delay': '0ms' }}
              class="sp-reveal sp-eyebrow"
            >
              {t.marginEyebrow}
            </p>
            <h2
              data-shown={headerShown() ? 'true' : 'false'}
              style={{ '--sp-delay': '60ms' }}
              class="sp-reveal mt-3 text-[32px] leading-[1.12] font-semibold tracking-[-0.03em] text-[#1D1D1F] [text-wrap:balance] sm:text-[46px]"
            >
              {t.marginTitle[0]}
              <br />
              {t.marginTitle[1]}
            </h2>
          </div>
          <div
            data-shown={headerShown() ? 'true' : 'false'}
            style={{ '--sp-delay': '120ms' }}
            class="sp-reveal"
          >
            <p class="text-[16px] leading-[1.65] text-[#6E6E73] [text-wrap:pretty] sm:text-[17px]">{t.marginLead}</p>
            <a
              href={ALAN_URL}
              target="_blank"
              rel="noopener noreferrer"
              class="sp-link mt-3 inline-block text-[14px]"
            >
              {t.poweredByAlan} ›
            </a>
          </div>
        </div>

        {/* Authentic Slowpage Reading Window Frame */}
        <div class="mx-auto mt-14 max-w-[1040px] rounded-[22px] bg-gradient-to-b from-[#EEEEF0] to-[#E8E8EC] p-2.5 shadow-[var(--sp-window-shadow)] ring-1 ring-black/[0.08] sm:p-3">
          {/* Integrated Top Window Bar with Compact Glass Actions */}
          <div class="flex flex-wrap items-center justify-between gap-2 px-3 pt-1 pb-2.5">
            <div class="flex items-center gap-3">
              <div class="flex items-center gap-2" aria-hidden="true">
                <span class="size-3 rounded-full bg-[#FF5F57] shadow-[inset_0_0_0_0.5px_rgba(0,0,0,0.15)]" />
                <span class="size-3 rounded-full bg-[#FEBC2E] shadow-[inset_0_0_0_0.5px_rgba(0,0,0,0.15)]" />
                <span class="size-3 rounded-full bg-[#28C840] shadow-[inset_0_0_0_0.5px_rgba(0,0,0,0.15)]" />
              </div>
              <span class="text-[12.5px] font-medium text-[#5c544a]">
                Pride and Prejudice · Chapter I
              </span>
            </div>

            <div class="flex items-center gap-2.5">
              <GlassPopover subtleTrigger>
                <GlassPopoverTrigger class="!h-8 !px-3.5 !text-[12.5px]">
                  {isEn ? 'Try asking @alan' : '试试问 @alan'}
                </GlassPopoverTrigger>
                <GlassPopoverContent>
                  <GlassPopoverTitle>{isEn ? 'Read this passage with @alan' : '让 @alan 陪你读这一段'}</GlassPopoverTitle>
                  <GlassPopoverDescription class="opacity-85">
                    {isEn
                      ? 'Pick a question to watch the reply stream right into the margin:'
                      : '挑一个问题，看看回复如何直接流式写到页边：'}
                  </GlassPopoverDescription>
                  <div class="flex flex-col gap-2">
                    <For each={QUICK_PROMPTS}>
                      {(prompt) => (
                        <GlassPopoverClose
                          variant="quiet"
                          class="h-auto w-full justify-start rounded-2xl px-3.5 py-2.5 text-left text-[13px] font-medium whitespace-normal"
                          onClick={() => askQuickAlan(prompt)}
                        >
                          {prompt.question}
                        </GlassPopoverClose>
                      )}
                    </For>
                  </div>
                </GlassPopoverContent>
              </GlassPopover>

              <GlassMenu subtleTrigger>
                <GlassMenuTrigger class="!h-8 !px-3.5 !text-[12.5px]">
                  {isEn ? 'Filter notes' : '筛选批注'}
                </GlassMenuTrigger>
                <GlassMenuContent align="end">
                  <GlassMenuGroup>
                    <GlassMenuLabel class="opacity-80">{isEn ? 'Show' : '显示范围'}</GlassMenuLabel>
                    <GlassMenuRadioGroup
                      value={filter()}
                      onValueChange={(value) => setFilter(value as FilterMode)}
                    >
                      <GlassMenuRadioItem value="all">{isEn ? 'All notes' : '全部批注'}</GlassMenuRadioItem>
                      <GlassMenuRadioItem value="mine">{isEn ? 'My notes only' : '仅看我的笔记'}</GlassMenuRadioItem>
                      <GlassMenuRadioItem value="alan">{isEn ? '@alan replies' : '仅看 @alan 回复'}</GlassMenuRadioItem>
                    </GlassMenuRadioGroup>
                  </GlassMenuGroup>
                  <GlassMenuSeparator />
                  <GlassMenuItem
                    onSelect={() => {
                      setNotes(INITIAL_NOTES)
                      setFilter('all')
                      setEditingId(null)
                      setSelectedId('n1')
                      setSavedNotice('')
                    }}
                  >
                    {isEn ? 'Reset demo notes' : '恢复默认示例'}
                  </GlassMenuItem>
                </GlassMenuContent>
              </GlassMenu>
            </div>
          </div>

          {/* Window Body: Paper Left + Measured SVG Connectors + Right Margin */}
          <div
            ref={(el) => {
              gridEl = el
            }}
            class="relative grid gap-3 lg:grid-cols-[1fr_256px]"
          >
            {/* Whisper-thin orthogonal SVG connector overlay with draw-on transition */}
            <svg
              aria-hidden="true"
              class="pointer-events-none absolute inset-0 z-20 hidden h-full w-full overflow-visible lg:block"
            >
              <For each={visibleNotes()}>
                {(note) => {
                  const pts = () => connectors()[note.id]
                  const alpha = () => connectorAlpha(note.id)
                  return (
                    <Show when={pts()}>
                      {(p) => (
                        <g
                          style={{
                            opacity: alpha(),
                            transition: 'opacity 180ms cubic-bezier(0.22, 1, 0.36, 1)',
                          }}
                        >
                          <path
                            d={`M ${p().x1.toFixed(1)} ${p().y1.toFixed(1)} L ${p().elbowX.toFixed(1)} ${p().y1.toFixed(1)} L ${p().elbowX.toFixed(1)} ${p().y2.toFixed(1)} L ${p().x2.toFixed(1)} ${p().y2.toFixed(1)}`}
                            fill="none"
                            stroke="#1D1D1F"
                            stroke-width="1"
                            stroke-dasharray={p().length.toFixed(1)}
                            stroke-dashoffset={alpha() > 0 ? '0' : p().length.toFixed(1)}
                            style={{
                              transition: 'stroke-dashoffset 260ms cubic-bezier(0.22, 1, 0.36, 1)',
                            }}
                          />
                          <circle cx={p().x1} cy={p().y1} r="2.2" fill="#1D1D1F" />
                        </g>
                      )}
                    </Show>
                  )
                }}
              </For>
            </svg>

            {/* Pure #FFFFFF reading page with organic box-decoration-break highlights */}
            <div
              ref={(el) => {
                paperEl = el
              }}
              class="sp-card relative rounded-[12px] px-7 py-10 text-left sm:px-14 sm:py-12"
            >
              <article class="sp-reading-en mx-auto max-w-[440px] text-[19px] leading-[1.6]">
                <div class="mb-5 flex items-center justify-between font-sans text-[11px] text-[#59595E]">
                  <span>Chapter I</span>
                  <span class="sp-tabular inline-flex items-center gap-1">
                    <IconBookmark class="size-3" />{' '}
                    {isEn
                      ? `${visibleNotes().length} ${visibleNotes().length === 1 ? 'note' : 'notes'} anchored`
                      : `已锚定 ${visibleNotes().length} 条批注`}
                  </span>
                </div>

                <p class="text-justify indent-6">
                  <span
                    ref={(el) => {
                      anchorRefs.n1 = el
                    }}
                    role="button"
                    tabindex={0}
                    onClick={() => selectAnchorSentence('n1')}
                    onKeyDown={(e) => onHighlightKeyDown(e, 'n1')}
                    onMouseEnter={() => {
                      const match = visibleNotes().find((n) => n.anchorId === 'n1')
                      if (match) setHoveredId(match.id)
                    }}
                    onMouseLeave={() => setHoveredId(null)}
                    class={`sp-highlight-amber cursor-pointer outline-none focus-visible:ring-2 focus-visible:ring-[#B4531F] ${
                      selectedAnchor() === 'n1'
                        ? 'bg-[#F0B267]/34'
                        : 'bg-[#F0B267]/16 hover:bg-[#F0B267]/26'
                    }`}
                  >
                    It is a truth universally acknowledged, that a single man in possession of a good fortune, must be in want of a wife.
                  </span>
                </p>

                <p class="mt-4 text-justify indent-6">
                  However little known the feelings or views of such a man may be on his first entering a neighbourhood, this truth is so well fixed in the minds of the surrounding families, that he is considered the rightful property of some one or other of their daughters.
                </p>

                <p class="mt-4 text-justify indent-6">
                  <span
                    ref={(el) => {
                      anchorRefs.n2 = el
                    }}
                    role="button"
                    tabindex={0}
                    onClick={() => selectAnchorSentence('n2')}
                    onKeyDown={(e) => onHighlightKeyDown(e, 'n2')}
                    onMouseEnter={() => {
                      const match = visibleNotes().find((n) => n.anchorId === 'n2')
                      if (match) setHoveredId(match.id)
                    }}
                    onMouseLeave={() => setHoveredId(null)}
                    class={`sp-highlight-blue cursor-pointer outline-none focus-visible:ring-2 focus-visible:ring-[#5B8DEF] ${
                      selectedAnchor() === 'n2'
                        ? 'bg-[#5B8DEF]/24'
                        : 'bg-[#5B8DEF]/12 hover:bg-[#5B8DEF]/20'
                    }`}
                  >
                    “My dear Mr. Bennet,” said his lady to him one day, “have you heard that Netherfield Park is let at last?”
                  </span>
                </p>
              </article>
            </div>

            {/* 256px Right Annotation Margin with opaque white cards */}
            <div class="flex flex-col justify-between gap-3 text-left">
              <div class="space-y-2.5">
                <For each={visibleNotes()}>
                  {(note) => {
                    const selected = () => selectedId() === note.id
                    const editing = () => editingId() === note.id
                    return (
                      <div
                        ref={(el) => {
                          cardRefs[note.id] = el
                        }}
                        onMouseEnter={() => setHoveredId(note.id)}
                        onMouseLeave={() => setHoveredId(null)}
                        onClick={() => setSelectedId(note.id)}
                        class={`sp-card-enter cursor-pointer rounded-[12px] bg-white p-3.5 ${
                          selected()
                            ? 'shadow-[0_0_0_1px_rgba(29,29,31,0.2),0_6px_18px_rgba(0,0,0,0.06)]'
                            : 'shadow-[0_0_0_0.5px_rgba(0,0,0,0.07),0_1px_2px_rgba(0,0,0,0.03)]'
                        }`}
                      >
                        <div class="flex items-center justify-between text-[11px] text-[#59595E]">
                          <span class={`font-semibold ${note.author === '@alan' ? 'text-[#B4531F]' : 'text-[#1D1D1F]'}`}>
                            {note.author}
                          </span>
                          <div class="flex items-center gap-1.5">
                            <span>{note.time}</span>
                            <button
                              type="button"
                              aria-label={isEn ? 'Edit note' : '编辑笔记'}
                              onClick={(e) => {
                                e.stopPropagation()
                                startEditing(note)
                              }}
                              class="sp-pressable flex size-5 cursor-pointer items-center justify-center rounded text-[#59595E] hover:bg-black/[0.05]"
                            >
                              <IconEdit class="size-3" />
                            </button>
                          </div>
                        </div>

                        <Show
                          when={!editing()}
                          fallback={
                            <div onClick={(e) => e.stopPropagation()} class="mt-2 space-y-2">
                              <textarea
                                rows={3}
                                value={editDraft()}
                                onInput={(e) => setEditDraft(e.currentTarget.value)}
                                class="w-full resize-none rounded-[8px] border border-black/12 bg-[#FAFAFC] p-2 text-[12px] leading-[1.45] text-[#1D1D1F] outline-none focus:border-black/30"
                              />
                              <div class="flex items-center justify-end gap-2 text-[11px]">
                                <button
                                  type="button"
                                  onClick={() => setEditingId(null)}
                                  class="sp-pressable cursor-pointer rounded px-2 py-1 text-[#59595E] hover:bg-black/[0.05]"
                                >
                                  {isEn ? 'Cancel' : '取消'}
                                </button>
                                <button
                                  type="button"
                                  onClick={() => saveEdit(note.id)}
                                  class="sp-pressable cursor-pointer rounded bg-[#1D1D1F] px-2.5 py-1 font-medium text-white"
                                >
                                  {isEn ? 'Save' : '保存'}
                                </button>
                              </div>
                            </div>
                          }
                        >
                          <p class={`mt-1.5 text-[13px] leading-[1.48] text-[#1D1D1F] ${selected() ? '' : 'line-clamp-2'}`}>
                            {note.body}
                          </p>

                          <Show when={selected() && note.reply}>
                            <div class="mt-2.5 rounded-[8px] bg-[#F5F5F7] p-2.5 text-[12px] leading-[1.48] text-[#3A3A3F]">
                              {note.reply}
                            </div>
                          </Show>
                        </Show>
                      </div>
                    )
                  }}
                </For>
              </div>

              {/* Bottom New-Note Composer — collapses cleanly when editing a card */}
              <Show
                when={!editingId()}
                fallback={
                  <div class="flex h-[84px] items-center justify-between rounded-[14px] bg-white/85 px-3.5 text-[12px] text-[#59595E] shadow-[0_0_0_0.5px_rgba(0,0,0,0.07)]">
                    <span class="flex items-center gap-1.5">
                      <IconNote class="size-3.5" /> {isEn ? 'Editing note in card' : '正在编辑卡片内笔记'}
                    </span>
                    <button
                      type="button"
                      onClick={() => setEditingId(null)}
                      class="sp-pressable cursor-pointer font-medium text-[#1D1D1F]"
                    >
                      {isEn ? 'Cancel' : '取消编辑'}
                    </button>
                  </div>
                }
              >
                <form
                  onSubmit={submitNewNote}
                  class="flex h-[84px] flex-col justify-between rounded-[14px] bg-white p-3 shadow-[0_0_0_0.5px_rgba(0,0,0,0.08),0_2px_10px_rgba(0,0,0,0.04)]"
                >
                  <input
                    type="text"
                    value={draftNote()}
                    onInput={(e) => {
                      setDraftNote(e.currentTarget.value)
                      if (savedNotice()) setSavedNotice('')
                    }}
                    placeholder={isEn ? 'Write a margin note, or ask @alan…' : '记一笔页边注，或 @alan 提问…'}
                    class="w-full bg-transparent text-[13px] text-[#1D1D1F] placeholder-[#636366] outline-none"
                  />
                  <div class="flex items-center justify-between text-[11px] text-[#59595E]">
                    <span class="truncate pr-2">
                      {savedNotice() || (isEn ? 'Press Return to save in margin' : '按回车记在页边')}
                    </span>
                    <button
                      type="submit"
                      aria-label={isEn ? 'Save note' : '保存批注'}
                      class="sp-pressable flex size-6 shrink-0 cursor-pointer items-center justify-center rounded-full bg-[#1D1D1F] text-white"
                    >
                      <IconArrowUp class="size-3.5" />
                    </button>
                  </div>
                </form>
              </Show>
            </div>
          </div>
        </div>
      </div>
    </section>
  )
}
