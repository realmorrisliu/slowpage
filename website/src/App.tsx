import type { JSX } from '@solidjs/web'
import { createSignal, lazy, Loading, onSettled, Show } from 'solid-js'
import { GlassStyles } from 'solid-liquid-glass'

import { ClosingSection } from './sections/ClosingSection'
import { Footer } from './sections/Footer'
import { HeroSection } from './sections/HeroSection'
import { Navbar } from './sections/Navbar'

const TypographySection = lazy(() => import('./sections/TypographySection'), {
  export: 'TypographySection',
})
const MarginaliaSection = lazy(() => import('./sections/MarginaliaSection'), {
  export: 'MarginaliaSection',
})
const LibrarySection = lazy(() => import('./sections/LibrarySection'), {
  export: 'LibrarySection',
})

const DEFERRED_SECTIONS = [TypographySection, MarginaliaSection, LibrarySection]
const SECTION_HASHES = new Set(['typography', 'marginalia', 'library', 'download'])

function DeferredSection(props: {
  id: string
  forceActive: () => boolean
  render: () => JSX.Element
}) {
  const [visible, setVisible] = createSignal(false)
  const active = () => props.forceActive() || visible()
  let sentinel: HTMLDivElement | undefined

  onSettled(() => {
    if (active()) return

    if (!sentinel || typeof IntersectionObserver === 'undefined') {
      setVisible(true)
      return
    }

    const observer = new IntersectionObserver(
      (entries) => {
        if (entries.some((entry) => entry.isIntersecting)) {
          setVisible(true)
          observer.disconnect()
        }
      },
      { rootMargin: '250px 0px' },
    )
    observer.observe(sentinel)

    return () => observer.disconnect()
  })

  return (
    <Show
      when={active()}
      fallback={
        <div
          id={props.id}
          data-deferred-placeholder=""
          ref={(el) => (sentinel = el)}
          class="min-h-[640px]"
        />
      }
    >
      <Loading fallback={<div id={props.id} data-deferred-placeholder="" class="min-h-[640px]" />}>
        {props.render()}
      </Loading>
    </Show>
  )
}

export function App() {
  const initialHash = typeof window !== 'undefined' ? window.location.hash.slice(1) : ''
  const [anchorTriggered, setAnchorTriggered] = createSignal(SECTION_HASHES.has(initialHash))

  onSettled(() => {
    let pendingAnchor: string | null = null

    const alignAnchor = (targetId: string) => {
      pendingAnchor = targetId
      setAnchorTriggered(true)
      Promise.all(DEFERRED_SECTIONS.map((section) => section.preload()))
        .then(() => {
          const step = (attemptsLeft: number) => {
            if (pendingAnchor !== targetId) return
            if (document.querySelector('[data-deferred-placeholder]') && attemptsLeft > 0) {
              requestAnimationFrame(() => step(attemptsLeft - 1))
              return
            }
            pendingAnchor = null
            document.getElementById(targetId)?.scrollIntoView()
          }
          requestAnimationFrame(() => step(10))
        })
        .catch(() => {})
    }

    if (SECTION_HASHES.has(initialHash)) {
      alignAnchor(initialHash)
    }

    const onHash = () => {
      const target = window.location.hash.slice(1)
      if (SECTION_HASHES.has(target)) alignAnchor(target)
      else pendingAnchor = null
    }
    const onAnchorClick = (event: MouseEvent) => {
      const anchor = (event.target as Element | null)?.closest('a[href^="#"]')
      if (!anchor) return
      const target = anchor.getAttribute('href')?.slice(1)
      if (target && SECTION_HASHES.has(target)) alignAnchor(target)
      else pendingAnchor = null
    }
    const onCancelScroll = () => {
      pendingAnchor = null
    }

    window.addEventListener('hashchange', onHash)
    document.addEventListener('click', onAnchorClick, true)
    window.addEventListener('wheel', onCancelScroll, { passive: true })
    window.addEventListener('touchstart', onCancelScroll, { passive: true })

    return () => {
      pendingAnchor = null
      window.removeEventListener('hashchange', onHash)
      document.removeEventListener('click', onAnchorClick, true)
      window.removeEventListener('wheel', onCancelScroll)
      window.removeEventListener('touchstart', onCancelScroll)
    }
  })

  return (
    <div class="min-h-screen bg-white">
      <GlassStyles />
      <Navbar />
      <main>
        <HeroSection />
        <DeferredSection
          id="typography"
          forceActive={anchorTriggered}
          render={() => <TypographySection />}
        />
        <DeferredSection
          id="marginalia"
          forceActive={anchorTriggered}
          render={() => <MarginaliaSection />}
        />
        <DeferredSection
          id="library"
          forceActive={anchorTriggered}
          render={() => <LibrarySection />}
        />
        <ClosingSection />
      </main>
      <Footer />
    </div>
  )
}
