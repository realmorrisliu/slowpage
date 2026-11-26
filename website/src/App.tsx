import { GlassStyles } from 'solid-liquid-glass'

import { ClosingSection } from './sections/ClosingSection'
import { Footer } from './sections/Footer'
import { HeroSection } from './sections/HeroSection'
import { HighlightsBand } from './sections/HighlightsBand'
import { LibrarySection } from './sections/LibrarySection'
import { MarginaliaSection } from './sections/MarginaliaSection'
import { Navbar } from './sections/Navbar'
import { TypographySection } from './sections/TypographySection'

export function App() {
  return (
    <div class="min-h-screen bg-white">
      <GlassStyles />
      <Navbar />
      <main>
        <HeroSection />
        <HighlightsBand />
        <TypographySection />
        <MarginaliaSection />
        <LibrarySection />
        <ClosingSection />
      </main>
      <Footer />
    </div>
  )
}
