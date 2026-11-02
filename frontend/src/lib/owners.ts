// Whose a shared board's card is (card #268): the member who made it, by
// themselves or through one of their agents; they answer for it, whoever
// holds it. The window sends the person id (CardSummary.owner); the board's
// summary lists its members by id.

// The few fields read here, so tests needn't build whole summaries.
export type OwnerMember = { personId: string; name: string; me: boolean }
export type OwnerBoard = { boardId: string; members?: OwnerMember[] | null }
export type OwnerCard = { scope: string; board?: string; owner?: string }

export type Owner = {
  personId: string
  name: string
  // me: the person at this device.
  me: boolean
  // known: a current member; false for one who left the board (or isn't
  // known on this device yet), named "Former member".
  known: boolean
}

export const FORMER_MEMBER = "Former member"

// memberOf names person id `personId` from a board's members: null for no
// id, else that member, or "Former member" when they aren't on it now.
export function memberOf(
  personId: string | undefined,
  members: OwnerMember[] | null | undefined
): Owner | null {
  if (!personId) return null
  const m = (members ?? []).find((x) => x.personId === personId)
  if (!m) return { personId, name: FORMER_MEMBER, me: false, known: false }
  return { personId, name: m.name.trim() || "Someone", me: m.me, known: true }
}

// sharedBoard is card c's board when it is one other people are on (more
// than one member): only there does whose a card is say anything; on a
// board that is only the person's, every card is theirs.
export function sharedBoard<B extends OwnerBoard>(
  c: OwnerCard,
  boards: B[]
): B | null {
  if (c.scope !== "board" || !c.board) return null
  const b = boards.find((x) => x.boardId === c.board)
  if (!b || (b.members ?? []).length < 2) return null
  return b
}

// cardOwner is the member card c belongs to on a shared board, or null off
// one (a private card, a board that is only the person's) or when the
// window couldn't say whose it is.
export function cardOwner(c: OwnerCard, boards: OwnerBoard[]): Owner | null {
  const b = sharedBoard(c, boards)
  return b ? memberOf(c.owner, b.members) : null
}

// ownerFilter is the "by member" filter's value for person id p.
export function ownerFilter(p: string): `owner:${string}` {
  return `owner:${p}`
}

// ownedBy says whether card c is owned by the person id in filter value f
// ("owner:<personId>").
export function ownedBy(c: OwnerCard, f: string): boolean {
  return !!c.owner && f === ownerFilter(c.owner)
}
