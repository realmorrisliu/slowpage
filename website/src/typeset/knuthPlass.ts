/**
 * Paragraph-at-once line breaking for the typography comparison.
 *
 * This reuses the exact Knuth–Plass core and hyphenation patterns that the
 * Slowpage reader injects into its EPUB pages (`spikes/justify`), with the
 * same break options, hyphen cost and hyphenation limits. Only the DOM glue
 * is website-specific: words are measured with canvas and each line is set
 * as a block whose word-spacing fills the measure.
 */
import layoutSource from '../../../spikes/justify/vendor/tex-linebreak/layout.js?raw'
import enUs from '../../../spikes/justify/vendor/hyphenated-en-us/patterns.json'

type Box = { type: 'box'; width: number }
type Glue = { type: 'glue'; width: number; stretch: number; shrink: number }
type Penalty = { type: 'penalty'; width: number; cost: number; flagged: boolean }
type Item = Box | Glue | Penalty

type TexLayout = {
  MIN_COST: number
  breakLines: (items: Item[], lineLengths: number[], opts: Record<string, unknown>) => number[]
  MaxAdjustmentExceededError: new () => Error
}

// The vendored file declares `createTexLayout` without exporting it.
const tex = new Function(`${layoutSource}\nreturn createTexLayout();`)() as TexLayout

/** Same values as `BREAKS`, `HYPHEN_COST`, `LEFT_MIN` and `RIGHT_MIN` in spikes/justify/justify.js. */
const BREAKS = {
  maxAdjustmentRatio: 0.75,
  initialMaxAdjustmentRatio: 0.5,
  doubleHyphenPenalty: 10000,
  adjacentLooseTightPenalty: 10000,
}
const HYPHEN_COST = 200
const LEFT_MIN = 2
const RIGHT_MIN = 3

type TrieNode = { points: number[] | null; kids: Record<string, TrieNode> }

let trie: TrieNode | null = null
let exceptions: Map<string, number[]> | null = null

function ensurePatterns() {
  if (trie) return
  const root: TrieNode = { points: null, kids: Object.create(null) }
  for (const pattern of enUs.patterns as string[]) {
    const chars = pattern.replace(/[0-9]/g, '')
    const points = pattern.split(/\D/).map((part) => (part === '' ? 0 : Number(part)))
    let node = root
    for (const ch of chars) node = node.kids[ch] ?? (node.kids[ch] = { points: null, kids: Object.create(null) })
    node.points = points
  }
  trie = root
  exceptions = new Map()
  for (const definition of (enUs.exceptions ?? []) as string[]) {
    const points: number[] = []
    let bare = 0
    for (const ch of definition) {
      if (ch === '-') points.push(bare)
      else bare += 1
    }
    exceptions.set(definition.replace(/-/g, '').toLowerCase(), points)
  }
}

/** Liang hyphenation points inside `word`, as character offsets. */
function liangPoints(word: string): number[] {
  ensurePatterns()
  const lower = word.toLowerCase()
  const known = exceptions!.get(lower)
  if (known) return known
  const padded = `.${lower}.`
  const scores = new Array<number>(padded.length).fill(0)
  for (let i = 0; i < padded.length; i++) {
    let node: TrieNode | undefined = trie!
    for (let j = i; j < padded.length; j++) {
      node = node.kids[padded.charAt(j)]
      if (!node) break
      if (node.points) {
        for (let k = 0; k < node.points.length; k++) {
          const at = i + k
          if (at < scores.length && node.points[k] > scores[at]) scores[at] = node.points[k]
        }
      }
    }
  }
  const out: number[] = []
  for (let i = 0; i < padded.length; i++) {
    if (i > LEFT_MIN && i < padded.length - RIGHT_MIN && scores[i] % 2 === 1) out.push(i - 1)
  }
  return out
}

function hyphenPoints(word: string): number[] {
  const match = /[A-Za-z]{5,}/.exec(word)
  if (!match) return []
  return liangPoints(match[0])
    .map((p) => match.index + p)
    .filter((at) => at > 0 && at < word.length)
}

export type SetLine = { text: string; wordSpacing: number; justify: boolean }

let canvas: HTMLCanvasElement | null = null

function measurer(font: string) {
  canvas ??= document.createElement('canvas')
  const ctx = canvas.getContext('2d')!
  ctx.font = font
  const cache = new Map<string, number>()
  return (s: string) => {
    let w = cache.get(s)
    if (w === undefined) {
      w = ctx.measureText(s).width
      cache.set(s, w)
    }
    return w
  }
}

/**
 * Break one paragraph. `indent` shortens the first line only. Returns null
 * when no break set fits the adjustment cap, which is when the reader also
 * leaves a paragraph to WebKit.
 */
export function setParagraph(text: string, font: string, width: number, indent: number): SetLine[] | null {
  const measure = measurer(font)
  const space = measure(' ')
  const hyphen = Math.max(1, measure('-'))
  const items: Item[] = []
  const strings: string[] = []

  const words = text.split(/\s+/).filter(Boolean)
  words.forEach((word, wi) => {
    if (wi > 0) {
      items.push({ type: 'glue', width: space, stretch: space, shrink: Math.max(0, space - 3) })
      strings.push(' ')
    }
    const bounds = [...hyphenPoints(word), word.length]
    let prev = 0
    bounds.forEach((at, n) => {
      const piece = word.slice(prev, at)
      if (piece) {
        items.push({ type: 'box', width: measure(piece) })
        strings.push(piece)
      }
      if (n < bounds.length - 1) {
        items.push({ type: 'penalty', width: hyphen, cost: HYPHEN_COST, flagged: true })
        strings.push('')
      }
      prev = at
    })
  })
  items.push({ type: 'glue', width: 0, stretch: 1000, shrink: 0 })
  strings.push('')
  items.push({ type: 'penalty', width: 0, cost: tex.MIN_COST, flagged: false })
  strings.push('')

  const lengths = [width - indent, ...new Array(64).fill(width)]
  let breaks: number[]
  try {
    breaks = tex.breakLines(items, lengths, BREAKS)
  } catch (err) {
    if (err instanceof tex.MaxAdjustmentExceededError) return null
    throw err
  }

  const lines: SetLine[] = []
  for (let l = 0; l < breaks.length - 1; l++) {
    let start = l === 0 ? 0 : breaks[l] + 1
    while (start < items.length && items[start].type !== 'box') start++
    const end = breaks[l + 1]
    let natural = 0
    let gaps = 0
    let s = ''
    for (let i = start; i < end; i++) {
      const item = items[i]
      if (item.type === 'box') natural += item.width
      if (item.type === 'glue' && item.stretch < 1000) {
        natural += item.width
        gaps += 1
      }
      s += strings[i]
    }
    const breakItem = items[end]
    if (breakItem.type === 'penalty' && breakItem.flagged) {
      s += '-'
      natural += hyphen
    }
    const last = l === breaks.length - 2
    const target = lengths[l]
    const wordSpacing = !last && gaps > 0 ? (target - natural) / gaps : 0
    lines.push({ text: s.trimEnd(), wordSpacing, justify: !last })
  }
  return lines
}
