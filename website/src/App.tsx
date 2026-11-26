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

const SECTION_HASHES = new Set(['typography', 'marginalia', 'library', 'download'])

function DeferredSection(props: { id: string; render: () => JSX.Element }) {
  const initialHash = typeof window !== 'undefined' ? window.location.hash.slice(1) : ''
  const [active, setActive] = createSignal(SECTION_HASHES.has(initialHash))
  let sentinel: HTMLDivElement | undefined

  onSettled(() => {
    if (active()) return

    let observer: IntersectionObserver | undefined
    const activate = () => {
      setActive(true)
      observer?.disconnect()
    }

    const onHash = () => {
      if (SECTION_HASHES.has(window.location.hash.slice(1))) activate()
    }
    const onAnchorClick = (event: MouseEvent) => {
      const anchor = (event.target as Element | null)?.closest('a[href^="#"]')
      const target = anchor?.getAttribute('href')?.slice(1)
      if (target && SECTION_HASHES.has(target)) activate()
    }

    window.addEventListener('hashchange', onHash)
    document.addEventListener('click', onAnchorClick, true)

    if (sentinel && typeof IntersectionObserver !== 'undefined') {
      observer = new IntersectionObserver(
        (entries) => {
          if (entries.some((entry) => entry.isIntersecting)) activate()
        },
        { rootMargin: '250px 0px' },
      )
      observer.observe(sentinel)
    } else {
      activate()
    }

    return () => {
      window.removeEventListener('hashchange', onHash)
      document.removeEventListener('click', onAnchorClick, true)
      observer?.disconnect()
    }
  })

  return (
    <Show
      when={active()}
      fallback={<div id={props.id} ref={(el) => (sentinel = el)} class="min-h-[640px]" />}
    >
      <Loading fallback={<div id={props.id} class="min-h-[640px]" />}>{props.render()}</Loading>
    </Show>
  )
}

export function App() {
  return (
    <div class="min-h-screen bg-white">
      <GlassStyles />
      <Navbar />
      <main>
        <HeroSection />
        <DeferredSection id="typography" render={() => <TypographySection />} />
        <DeferredSection id="marginalia" render={() => <MarginaliaSection />} />
        <DeferredSection id="library" render={() => <LibrarySection />} />
        <ClosingSection />
      </main>
      <Footer />
    </div>
  )
}
