import { createSignal, onSettled } from 'solid-js'

export function prefersReducedMotion(): boolean {
  return typeof window !== 'undefined' && window.matchMedia('(prefers-reduced-motion: reduce)').matches
}

export function rubberband(overshoot: number, dimension: number, constant = 0.55): number {
  if (dimension <= 0) return 0
  return (overshoot * dimension * constant) / (dimension + constant * Math.abs(overshoot))
}

export function projectMomentum(velocityPxPerSec: number, decelerationRate = 0.992): number {
  return ((velocityPxPerSec / 1000) * decelerationRate) / (1 - decelerationRate)
}

export type SpringOptions = {
  damping?: number
  response?: number
  precision?: number
}

/**
 * Interruptible critically/slightly-underdamped spring driven by requestAnimationFrame.
 * Preserves current position and velocity on re-target so mid-flight interruptions stay fluid.
 */
export function createSpringSignal(
  initial: number,
  opts: SpringOptions = {},
): [() => number, (nextTarget: number, initialVelocity?: number) => void, (instant: number) => void] {
  const damping = opts.damping ?? 1.0
  const response = opts.response ?? 0.36
  const precision = opts.precision ?? 0.05

  const [value, setValue] = createSignal(initial)

  let current = initial
  let velocity = 0
  let target = initial
  let rafId = 0
  let lastTime = 0

  const cancel = () => {
    if (rafId) {
      cancelAnimationFrame(rafId)
      rafId = 0
    }
    lastTime = 0
  }

  const jumpTo = (instant: number) => {
    cancel()
    current = instant
    target = instant
    velocity = 0
    setValue(instant)
  }

  const tick = (now: number) => {
    if (!lastTime) {
      lastTime = now
      rafId = requestAnimationFrame(tick)
      return
    }
    const dt = Math.min((now - lastTime) / 1000, 0.032)
    lastTime = now

    const omega = (2 * Math.PI) / response
    const stiffness = omega * omega
    const friction = 2 * damping * omega

    const displacement = current - target
    const accel = -stiffness * displacement - friction * velocity
    velocity += accel * dt
    current += velocity * dt

    if (Math.abs(current - target) < precision && Math.abs(velocity) < precision * 4) {
      current = target
      velocity = 0
      setValue(target)
      rafId = 0
      lastTime = 0
      return
    }

    setValue(current)
    rafId = requestAnimationFrame(tick)
  }

  const setTarget = (nextTarget: number, initialVelocity?: number) => {
    if (prefersReducedMotion()) {
      jumpTo(nextTarget)
      return
    }
    target = nextTarget
    if (initialVelocity !== undefined) {
      velocity = initialVelocity
    }
    if (Math.abs(current - target) < precision && Math.abs(velocity) < precision * 4) {
      jumpTo(target)
      return
    }
    if (!rafId) {
      lastTime = 0
      rafId = requestAnimationFrame(tick)
    }
  }

  onSettled(() => cancel)

  return [value, setTarget, jumpTo]
}

export type SwipePagerOptions = {
  pageIndex: () => number
  pageCount: () => number
  onTurn: (nextIndex: number) => void
  width?: () => number
}

/**
 * 1:1 pointer swipe/drag pager with horizontal hysteresis, edge rubber-banding,
 * momentum projection, and velocity handoff into an interruptible spring.
 */
export function createSwipePager(opts: SwipePagerOptions) {
  const [springX, setSpringTarget, jumpSpringX] = createSpringSignal(0, {
    damping: 0.94,
    response: 0.34,
    precision: 0.25,
  })
  const [isDragging, setIsDragging] = createSignal(false)
  const [turnDirection, setTurnDirection] = createSignal<-1 | 1>(1)

  let activePointerId: number | null = null
  let startX = 0
  let startY = 0
  let committed = false
  let cancelledVertical = false
  let samples: Array<{ x: number; t: number }> = []

  const getDim = () => opts.width?.() ?? 420

  const isInteractiveTarget = (target: EventTarget | null) => {
    if (!(target instanceof Element)) return false
    return Boolean(target.closest('a, input, textarea, select, [data-no-drag], [role="button"]'))
  }

  const applyDrag = (rawDelta: number) => {
    const idx = opts.pageIndex()
    const count = opts.pageCount()
    const dim = getDim()
    const atStart = idx === 0 && rawDelta > 0
    const atEnd = idx >= count - 1 && rawDelta < 0
    const nextX = atStart || atEnd ? rubberband(rawDelta, dim * 0.45, 0.45) : rawDelta
    jumpSpringX(nextX)
  }

  const computeVelocity = (): number => {
    if (samples.length < 2) return 0
    const first = samples[0]
    const last = samples[samples.length - 1]
    const dt = (last.t - first.t) / 1000
    if (dt <= 0.004) return 0
    return (last.x - first.x) / dt
  }

  const triggerTurn = (delta: -1 | 1) => {
    const cur = opts.pageIndex()
    const next = cur + delta
    if (next < 0 || next >= opts.pageCount()) return
    setTurnDirection(delta)
    opts.onTurn(next)
    if (prefersReducedMotion()) {
      jumpSpringX(0)
      return
    }
    jumpSpringX(delta * 42)
    setSpringTarget(0)
  }

  const onPointerDown = (e: PointerEvent) => {
    if (e.button !== 0 && e.pointerType === 'mouse') return
    if (activePointerId !== null || isInteractiveTarget(e.target)) return
    activePointerId = e.pointerId
    startX = e.clientX
    startY = e.clientY
    committed = false
    cancelledVertical = false
    samples = [{ x: e.clientX, t: performance.now() }]
  }

  const onPointerMove = (e: PointerEvent) => {
    if (e.pointerId !== activePointerId || cancelledVertical) return
    const dx = e.clientX - startX
    const dy = e.clientY - startY

    if (!committed) {
      if (Math.abs(dy) > 10 && Math.abs(dy) > Math.abs(dx) * 1.15) {
        cancelledVertical = true
        activePointerId = null
        return
      }
      if (Math.abs(dx) < 10) return
      committed = true
      setIsDragging(true)
      try {
        ;(e.currentTarget as HTMLElement | null)?.setPointerCapture(e.pointerId)
      } catch {
        // ignore if pointer capture is unsupported
      }
    }

    const now = performance.now()
    samples.push({ x: e.clientX, t: now })
    if (samples.length > 6) samples.shift()
    applyDrag(dx)
  }

  const finishDrag = (e: PointerEvent) => {
    if (e.pointerId !== activePointerId) return
    activePointerId = null
    if (!committed) return
    committed = false
    setIsDragging(false)

    try {
      ;(e.currentTarget as HTMLElement | null)?.releasePointerCapture(e.pointerId)
    } catch {
      // ignore
    }

    const dx = e.clientX - startX
    const velocity = computeVelocity()
    const projected = dx + projectMomentum(velocity, 0.991)
    const threshold = Math.min(84, getDim() * 0.16)
    const idx = opts.pageIndex()
    const count = opts.pageCount()

    if (projected < -threshold && idx < count - 1) {
      setTurnDirection(1)
      opts.onTurn(idx + 1)
      if (!prefersReducedMotion()) {
        jumpSpringX(Math.max(-28, dx * 0.25 + 48))
        setSpringTarget(0, velocity * 0.22)
      } else {
        jumpSpringX(0)
      }
      return
    }

    if (projected > threshold && idx > 0) {
      setTurnDirection(-1)
      opts.onTurn(idx - 1)
      if (!prefersReducedMotion()) {
        jumpSpringX(Math.min(28, dx * 0.25 - 48))
        setSpringTarget(0, velocity * 0.22)
      } else {
        jumpSpringX(0)
      }
      return
    }

    setSpringTarget(0, velocity * 0.35)
  }

  return {
    offsetX: springX,
    isDragging,
    turnDirection,
    triggerTurn,
    handlers: {
      onPointerDown,
      onPointerMove,
      onPointerUp: finishDrag,
      onPointerCancel: finishDrag,
    },
  }
}

/**
 * One-shot IntersectionObserver reveal hook; resolves immediately under prefers-reduced-motion.
 */
export function useReveal(
  ref: () => HTMLElement | undefined,
  opts: { threshold?: number; rootMargin?: string; immediate?: boolean } = {},
) {
  const [revealed, setRevealed] = createSignal(Boolean(opts.immediate) || prefersReducedMotion())

  onSettled(() => {
    if (revealed()) return
    const el = ref()
    if (!el || typeof IntersectionObserver === 'undefined') {
      setRevealed(true)
      return
    }
    const io = new IntersectionObserver(
      (entries) => {
        for (const entry of entries) {
          if (entry.isIntersecting) {
            setRevealed(true)
            io.disconnect()
            break
          }
        }
      },
      {
        threshold: opts.threshold ?? 0.12,
        rootMargin: opts.rootMargin ?? '0px 0px -40px 0px',
      },
    )
    io.observe(el)
    return () => io.disconnect()
  })

  return revealed
}

/**
 * Passive scroll threshold hook for the edge-to-edge sticky Apple navbar.
 */
export function useScrolled(threshold = 12) {
  const [scrolled, setScrolled] = createSignal(
    typeof window !== 'undefined' ? window.scrollY > threshold : false,
  )

  onSettled(() => {
    const update = () => setScrolled(window.scrollY > threshold)
    update()
    window.addEventListener('scroll', update, { passive: true })
    return () => window.removeEventListener('scroll', update)
  })

  return scrolled
}

/**
 * Subtle normalized entrance progress (0..1) as an element settles into viewport focus.
 */
export function useScrollProgress(ref: () => HTMLElement | undefined) {
  const [progress, setProgress] = createSignal(prefersReducedMotion() ? 1 : 0.72)

  onSettled(() => {
    if (prefersReducedMotion()) {
      setProgress(1)
      return
    }
    let raf = 0
    const measure = () => {
      raf = 0
      const el = ref()
      if (!el) return
      const rect = el.getBoundingClientRect()
      const vh = window.innerHeight || 900
      // Settles toward 1 once the top half of the window enters comfortably into view
      const raw = (vh - rect.top) / (vh * 0.65)
      setProgress(Math.max(0.5, Math.min(1, raw)))
    }
    const onScroll = () => {
      if (!raf) raf = requestAnimationFrame(measure)
    }
    measure()
    window.addEventListener('scroll', onScroll, { passive: true })
    window.addEventListener('resize', onScroll, { passive: true })
    return () => {
      if (raf) cancelAnimationFrame(raf)
      window.removeEventListener('scroll', onScroll)
      window.removeEventListener('resize', onScroll)
    }
  })

  return progress
}
