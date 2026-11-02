import { StrictMode } from "react"
import { createRoot } from "react-dom/client"

import "./index.css"
import App from "./App.tsx"
import { TooltipProvider } from "@/components/ui/tooltip"
import { commands } from "./gen/bindings"
import { inHost } from "./lib/irori"

// Card #232: index.html shows the opening screen while the Core starts on
// its own thread (a fully synced log takes a while on a phone); every
// command needs the Core, so the app mounts once it is up. Outside the
// window (the browser preview, #260) there is no Core to wait for.
async function mount() {
  try {
    if (inHost()) await commands.coreReady()
  } catch (err) {
    const text = document.getElementById("irori-opening-text")
    if (text) text.textContent = `Irori couldn't open: ${err instanceof Error ? err.message : String(err)}`
    return
  }
  createRoot(document.getElementById("root")!).render(
    <StrictMode>
      {/* Dark only, like Toru: index.html carries class="dark", so there is
          no theme provider and no light/dark toggle on this branch. */}
      <TooltipProvider delay={300}>
        <App />
      </TooltipProvider>
    </StrictMode>
  )
}

void mount()
