import { useEffect, useState } from "react"

// useReducedMotion follows the OS's Reduce Motion: the cards' confetti
// honours it (toast skips the burst while it is on), and so does the
// thread's changing word (card #317).
export function useReducedMotion(): boolean {
  const query = "(prefers-reduced-motion: reduce)"
  const [on, setOn] = useState(
    () => window.matchMedia?.(query).matches ?? false
  )
  useEffect(() => {
    const m = window.matchMedia?.(query)
    if (!m) return
    const change = () => setOn(m.matches)
    m.addEventListener("change", change)
    return () => m.removeEventListener("change", change)
  }, [])
  return on
}
