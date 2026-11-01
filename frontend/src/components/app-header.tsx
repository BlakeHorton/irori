import type { ReactNode } from "react"

import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import type { Roster } from "@/lib/irori"
import type { Section } from "@/lib/nav"

// AppHeader is the page header (card #135): the page's title and its own
// actions. The nav lives in the sidebar. New card on Home and on a board's
// page (card #116, there it goes on that board), New board on Boards (card
// #126); a board's page adds its own actions (pin, rename, leave) as
// `actions`.
export function AppHeader({
  roster,
  title,
  sub,
  bar,
  actions,
  onNewCard,
  onNewBoard,
  section,
}: {
  roster: Roster
  // The page's title (Home, Boards, a board's name) and a line beside it.
  title: ReactNode
  sub?: ReactNode
  // bar replaces the title (a board's page: its name, rename and actions).
  bar?: ReactNode
  // The page's own actions, before New card / New board.
  actions?: ReactNode
  onNewCard: () => void
  onNewBoard: () => void
  // Where the window is; null on a card's page.
  section?: Section | null
}) {
  return (
    <header className="sticky top-0 z-20 border-x-0! border-t-0! glass-panel">
      <div className="mx-auto flex h-14 max-w-[1920px] items-center gap-3 px-4 sm:px-6">
        {bar ?? (
          <h1 className="flex min-w-0 items-baseline gap-2 text-lg font-semibold text-ink">
            {title}
          </h1>
        )}
        {!bar && sub && (
          <span className="hidden min-w-0 truncate text-xs text-muted-foreground md:inline">
            {sub}
          </span>
        )}
        {roster.source === "sample" && (
          <Badge variant="warning" title="Not connected to the Irori app">
            Sample data
          </Badge>
        )}
        <div className="ml-auto flex shrink-0 items-center gap-1.5">
          {actions}
          {section?.kind === "boards" ? (
            <Button
              size="sm"
              onClick={onNewBoard}
              title="Make a board for a project, or join one with an invite"
            >
              + New board
            </Button>
          ) : (
            section && (
              <Button
                size="sm"
                onClick={onNewCard}
                title={
                  section.kind === "board"
                    ? "Make a card on this board"
                    : "Make a card for an agent to pick up"
                }
              >
                + New card
              </Button>
            )
          )}
        </div>
      </div>
    </header>
  )
}
