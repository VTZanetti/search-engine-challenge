import { createApp, h } from 'vue'

import 'glasstora/style.css'
import './style.css'

import App from './App.vue'
import { GlassProvider } from 'glasstora'

// The whole app lives inside a single GlassProvider: it drives the shared light
// source, the refraction filter and the dark theme tokens.
createApp({
  render: () =>
    h(
      GlassProvider,
      { theme: 'dark', grain: true },
      { default: () => h(App) },
    ),
}).mount('#app')