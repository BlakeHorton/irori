import { useEffect, useState, type FormEvent } from "react"

import { ChoiceSelect } from "@/components/common/choice-select"
import { Button } from "@/components/ui/button"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Textarea } from "@/components/ui/textarea"
import {
  cards,
  displayName,
  useMentionCandidates,
  type AskRepo,
} from "@/lib/cards"
import { errorText } from "@/lib/talk"

// NewCardDialog is New card (card #116): the person makes a card straight
// from the header, no ask. Title is all it needs; goal, done-when and the
// repo say more. With nobody picked, Irori hands it to one free agent (the
// one idle longest), as it does any unheld card; picking an agent hands it
// to that one, woken. onMade gets the new card's id.
export function NewCardDialog({
  open,
  onClose,
  onMade,
}: {
  open: boolean
  onClose: () => void
  onMade: (id: string) => void
}) {
  const [title, setTitle] = useState("")
  const [objective, setObjective] = useState("")
  const [doneWhen, setDoneWhen] = useState("")
  const [repoKey, setRepoKey] = useState("")
  const [assignTo, setAssignTo] = useState("")
  const [repos, setRepos] = useState<AskRepo[]>([])
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const agents = useMentionCandidates(open).filter((c) => c.who.live)

  useEffect(() => {
    if (!open) return
    let live = true
    void cards.askRepos().then((r) => live && setRepos(r))
    return () => {
      live = false
    }
  }, [open])

  const reset = () => {
    setTitle("")
    setObjective("")
    setDoneWhen("")
    setRepoKey("")
    setAssignTo("")
    setError(null)
  }

  const submit = async (e: FormEvent) => {
    e.preventDefault()
    if (busy) return
    if (!title.trim()) {
      setError("Give the card a title.")
      return
    }
    setBusy(true)
    setError(null)
    try {
      const res = await cards.create({
        title,
        objective,
        doneWhen,
        repoKey,
        assignTo,
      })
      reset()
      onMade(res.task.id)
      onClose()
    } catch (err) {
      setError(errorText(err))
    } finally {
      setBusy(false)
    }
  }

  return (
    <Dialog open={open} onOpenChange={(o) => !o && !busy && onClose()}>
      <DialogContent className="sm:max-w-lg">
        <form className="grid gap-3" onSubmit={(e) => void submit(e)}>
          <DialogHeader>
            <DialogTitle>New card</DialogTitle>
            <DialogDescription>
              Work for an agent to pick up. Nobody picked: Irori hands it to
              the free agent idle longest.
            </DialogDescription>
          </DialogHeader>
          <div className="grid gap-1.5">
            <Label htmlFor="new-card-title">Title</Label>
            <Input
              id="new-card-title"
              value={title}
              autoFocus
              maxLength={200}
              placeholder="What needs doing"
              onChange={(e) => setTitle(e.target.value)}
            />
          </div>
          <div className="grid gap-1.5">
            <Label htmlFor="new-card-goal">Goal</Label>
            <Textarea
              id="new-card-goal"
              value={objective}
              autoSize
              className="max-h-48"
              placeholder="What and why (optional)"
              onChange={(e) => setObjective(e.target.value)}
            />
          </div>
          <div className="grid gap-1.5">
            <Label htmlFor="new-card-done">Done when</Label>
            <Input
              id="new-card-done"
              value={doneWhen}
              placeholder="How to tell it's done (optional)"
              onChange={(e) => setDoneWhen(e.target.value)}
            />
          </div>
          <div className="grid grid-cols-2 gap-3">
            <ChoiceSelect
              label="Repo"
              showLabel
              value={repoKey}
              onValueChange={setRepoKey}
              className="w-full"
              choices={[
                { value: "", label: "No repo" },
                ...repos.map((r) => ({ value: r.repoKey, label: r.label })),
              ]}
            />
            <ChoiceSelect
              label="Agent"
              showLabel
              value={assignTo}
              onValueChange={setAssignTo}
              className="w-full"
              choices={[
                { value: "", label: "Any free agent" },
                ...agents.map((c) => ({
                  value: c.who.key,
                  label: c.repo
                    ? `${displayName(c.who)} · ${c.repo}`
                    : displayName(c.who),
                })),
              ]}
            />
          </div>
          {error && <p className="text-xs text-destructive">{error}</p>}
          <DialogFooter>
            <Button
              type="button"
              variant="ghost"
              disabled={busy}
              onClick={onClose}
            >
              Cancel
            </Button>
            <Button type="submit" disabled={busy}>
              {busy ? "Making it…" : "Make the card"}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  )
}
