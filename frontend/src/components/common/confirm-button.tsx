import { useRef, useState, type ReactNode } from "react"

import { Button } from "@/components/ui/button"
import {
  AlertDialog,
  AlertDialogClose,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
  AlertDialogTrigger,
} from "@/components/ui/dialog"
import { errorText } from "@/lib/talk"

// ConfirmButton asks before it acts: the click opens an alert dialog with
// the question and Confirm / Cancel. Confirm has focus, so Enter confirms
// and Escape cancels. A plain-text question is the dialog's title; a richer
// one sits under confirmLabel as the description. It runs onConfirm once
// and shows its error, if any, under the button until the next try.
export function ConfirmButton({
  label,
  disabled,
  variant = "outline",
  ...ask
}: {
  label: ReactNode
  disabled?: boolean
} & ConfirmAsk) {
  const [asking, setAsking] = useState(false)
  const [error, setError] = useState<string | null>(null)
  return (
    <span className="flex flex-col items-end gap-1">
      <ConfirmDialog
        {...ask}
        variant={variant}
        open={asking}
        onOpenChange={(open) => {
          if (open) setError(null)
          setAsking(open)
        }}
        onError={setError}
        trigger={
          <AlertDialogTrigger
            render={
              <Button
                size="xs"
                variant={variant === "destructive" ? "outline" : variant}
                disabled={disabled}
              />
            }
          >
            {label}
          </AlertDialogTrigger>
        }
      />
      {error && (
        <span className="max-w-xs text-right text-xs text-destructive">
          {error}
        </span>
      )}
    </span>
  )
}

// ConfirmAsk is what a confirm step asks and does.
export type ConfirmAsk = {
  question: ReactNode
  confirmLabel?: string
  cancelLabel?: string
  variant?: "outline" | "destructive" | "default"
  onConfirm: () => Promise<unknown>
}

// ConfirmDialog is ConfirmButton's dialog on its own, opened by its owner
// (a menu item, say) through open. onError gets onConfirm's error, if any;
// the dialog closes either way, as ConfirmButton's does.
export function ConfirmDialog({
  question,
  confirmLabel = "Confirm",
  cancelLabel = "Cancel",
  variant = "outline",
  onConfirm,
  open,
  onOpenChange,
  onError,
  trigger,
}: ConfirmAsk & {
  open: boolean
  onOpenChange: (open: boolean) => void
  onError: (error: string) => void
  trigger?: ReactNode
}) {
  const [busy, setBusy] = useState(false)
  const confirmRef = useRef<HTMLButtonElement>(null)

  const confirm = async () => {
    setBusy(true)
    try {
      await onConfirm()
    } catch (err) {
      onError(errorText(err))
    } finally {
      setBusy(false)
      onOpenChange(false)
    }
  }

  return (
    <AlertDialog
      open={open}
      onOpenChange={(next) => {
        if (busy) return
        onOpenChange(next)
      }}
    >
      {trigger}
      <AlertDialogContent initialFocus={confirmRef}>
        <AlertDialogHeader>
          {typeof question === "string" ? (
            <AlertDialogTitle>{question}</AlertDialogTitle>
          ) : (
            <>
              <AlertDialogTitle>{confirmLabel}</AlertDialogTitle>
              <AlertDialogDescription render={<div />}>
                {question}
              </AlertDialogDescription>
            </>
          )}
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogClose
            disabled={busy}
            render={<Button size="sm" variant="ghost" />}
          >
            {cancelLabel}
          </AlertDialogClose>
          <Button
            ref={confirmRef}
            size="sm"
            variant={variant === "outline" ? "default" : variant}
            disabled={busy}
            onClick={() => void confirm()}
          >
            {busy ? "Working…" : confirmLabel}
          </Button>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  )
}
