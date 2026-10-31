import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { SettingsMenu } from "@/components/settings/settings-menu"
import type { Roster } from "@/lib/irori"
import { go, type Section } from "@/lib/nav"

// AppHeader is the top bar: the product mark, Home | Boards (card #96),
// this device, the version, the Held list (with how many messages wait on
// the person) and the settings.
export function AppHeader({
  roster,
  held,
  onHeld,
  section,
  boardsNeedYou = 0,
}: {
  roster: Roster
  held: number
  onHeld: () => void
  // Where the window is; null on a card's page (neither is current).
  section?: Section | null
  // Cards on boards waiting on the person, shown on Boards.
  boardsNeedYou?: number
}) {
  const { device, version, source, agents } = roster
  const busy = agents.filter((a) => a.status === "busy").length

  return (
    <header className="sticky top-0 z-20 glass-panel border-x-0! border-t-0!">
      <div className="mx-auto flex h-14 max-w-[1920px] items-center gap-3 px-4 sm:px-6">
        <div className="flex items-baseline gap-2">
          <span className="text-lg font-semibold tracking-tight text-ink">
            Irori
          </span>
          <span lang="ja" className="text-sm font-normal text-muted-foreground">
            囲炉裏
          </span>
        </div>
        <nav className="ml-2 flex items-center gap-1" aria-label="Sections">
          <Button
            size="sm"
            variant={section?.kind === "home" ? "secondary" : "ghost"}
            aria-current={section?.kind === "home" ? "page" : undefined}
            onClick={() => go({ kind: "home" })}
          >
            Home
          </Button>
          <Button
            size="sm"
            variant={section && section.kind !== "home" ? "secondary" : "ghost"}
            aria-current={section && section.kind !== "home" ? "page" : undefined}
            onClick={() => go({ kind: "boards" })}
          >
            Boards
            {boardsNeedYou > 0 && (
              <Badge variant="destructive" className="tabular-nums" title="Cards on your boards waiting on you">
                {boardsNeedYou}
              </Badge>
            )}
          </Button>
        </nav>
        {source === "sample" && (
          <Badge variant="warning" title="Not connected to the Irori app">
            Sample data
          </Badge>
        )}
        <div className="ml-auto flex min-w-0 items-center gap-3 text-sm text-muted-foreground">
          <span className="hidden text-xs tabular-nums sm:inline">
            {agents.length} {agents.length === 1 ? "agent" : "agents"}
            {busy > 0 && <span className="text-primary"> · {busy} busy</span>}
          </span>
          <span className="truncate text-xs font-medium text-ink-2" title={device?.id}>
            {device?.name || "this device"}
          </span>
          <Button
            size="sm"
            variant={held > 0 ? "outline" : "ghost"}
            onClick={onHeld}
            title="Messages Irori's wake policy held"
          >
            Held
            {held > 0 && (
              <Badge variant="warning" className="tabular-nums">
                {held}
              </Badge>
            )}
          </Button>
          {version && (
            <Badge variant="slate" className="font-mono normal-case">
              {version === "dev" || version === "sample"
                ? version
                : `v${version.replace(/^v/, "")}`}
            </Badge>
          )}
          <SettingsMenu />
        </div>
      </div>
    </header>
  )
}
