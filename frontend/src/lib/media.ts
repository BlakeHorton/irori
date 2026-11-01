import { useEffect, useState } from "react"

// narrowQuery is a phone or a narrow window (card #227): below Tailwind's md
// (768 px) the sidebar is a drawer and the pages go single-column.
export const narrowQuery = "(max-width: 767.98px)"

// useMedia follows a CSS media query.
export function useMedia(query: string): boolean {
  const [on, setOn] = useState(
    () => window.matchMedia?.(query).matches ?? false
  )
  useEffect(() => {
    const m = window.matchMedia?.(query)
    if (!m) return
    const change = () => setOn(m.matches)
    change()
    m.addEventListener("change", change)
    return () => m.removeEventListener("change", change)
  }, [query])
  return on
}

// useNarrow is useMedia(narrowQuery).
export function useNarrow(): boolean {
  return useMedia(narrowQuery)
}
