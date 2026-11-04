// "Main is red" (card #436): a test that fails on clean main has one card,
// and every card that hit it waits on that card. Home leads with a line per
// red test so the person sees the bottleneck at once.

type Card = {
  id: string
  short: string
  tag?: string
  state: string
  mainRed?: string
  waitTask?: string
}

export type MainRed = { id: string; ref: string; test: string; waiting: number }

const closed = new Set(["completed", "failed", "canceled"])

// mainReds lists the open red-main cards, the most waited on first.
export function mainReds(cards: Card[]): MainRed[] {
  const open = cards.filter((c) => !closed.has(c.state))
  return open
    .filter((c) => c.mainRed)
    .map((c) => ({
      id: c.id,
      ref: `${c.tag ?? ""}#${c.short}`,
      test: c.mainRed ?? "",
      waiting: open.filter((o) => o.waitTask === c.id).length,
    }))
    .sort((a, b) => b.waiting - a.waiting || a.id.localeCompare(b.id))
}

// mainRedText is one line: "main is red: attach_exe fails; 5 cards wait on #436".
export function mainRedText(r: MainRed): string {
  const n = r.waiting === 1 ? "1 card waits" : `${r.waiting} cards wait`
  return `Main is red: ${r.test} fails on clean main; ${n} on ${r.ref}.`
}
