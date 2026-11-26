import { render } from '@solidjs/web'

import './styles/app.css'
import { installAnalytics } from './analytics'
import { App } from './App'

const root = document.getElementById('root')
if (root) {
  root.replaceChildren()
  render(() => <App />, root)
}
installAnalytics()
