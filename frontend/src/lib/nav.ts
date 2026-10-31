// Where the window is (card #96): Home (your own cards), the Boards page,
// or one board's page. It lives in the address, like an open card
// ("#card/<id>"), so back and forward move between them: "#boards",
// "#board/<id>", and no hash for Home. A card's page opens over whichever
// of them it was opened from.

export type Section = { kind: "home" } | { kind: "boards" } | { kind: "board"; id: string }

export const Home: Section = { kind: "home" }

// sectionFromHash is the section an address names (a card's page counts
// as the section history has behind it, so Home here).
export function sectionFromHash(hash: string): Section {
  if (hash === "#boards") return { kind: "boards" }
  const m = hash.match(/^#board\/(.+)$/)
  if (m) return { kind: "board", id: decodeURIComponent(m[1]) }
  return Home
}

export function sectionHash(s: Section): string {
  if (s.kind === "boards") return "#boards"
  if (s.kind === "board") return `#board/${encodeURIComponent(s.id)}`
  return ""
}

// go moves the window to section s as a new history entry; App follows
// it through popstate, the same as the browser's back and forward.
export function go(s: Section): void {
  const url = window.location.pathname + window.location.search + sectionHash(s)
  window.history.pushState({ section: s }, "", url)
  window.dispatchEvent(new PopStateEvent("popstate", { state: { section: s } }))
}

export function sameSection(a: Section, b: Section): boolean {
  if (a.kind !== b.kind) return false
  return a.kind !== "board" || (b.kind === "board" && a.id === b.id)
}
