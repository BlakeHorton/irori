import {
  useEffect,
  useMemo,
  useRef,
  useState,
  type ClipboardEvent,
  type DragEvent,
} from "react"
import { PaperclipIcon, XIcon } from "lucide-react"

import { Button } from "@/components/ui/button"
import { MaxUploadFiles } from "@/lib/cards"

import { FileKindIcon } from "./file-kind-icon"

// useFileAttach is a box's files waiting to go (cards #25, #73, #124): pick
// them with the clip, drop them on the box, or paste a screenshot. The
// ask box, the comment and thread Reply boxes and New card share it.
// off (a shared board's box, or while sending) takes no new files.
export function useFileAttach(off = false) {
  const [files, setFiles] = useState<File[]>([])
  const picker = useRef<HTMLInputElement>(null)
  const add = (list: FileList | File[] | null | undefined) => {
    const more = Array.from(list ?? [])
    if (!more.length || off) return false
    setFiles((fs) => [...fs, ...more].slice(0, MaxUploadFiles))
    return true
  }
  return {
    files,
    full: files.length >= MaxUploadFiles,
    add,
    clear: () => setFiles([]),
    remove: (i: number) => setFiles((fs) => fs.filter((_, j) => j !== i)),
    pick: () => picker.current?.click(),
    // A pasted screenshot is a file; pasted text stays text.
    onPaste: (e: ClipboardEvent) => {
      if (off || !e.clipboardData.files.length) return false
      e.preventDefault()
      return add(e.clipboardData.files)
    },
    // Spread on the element files are dropped on.
    drop: {
      onDragOver: (e: DragEvent) => {
        if (e.dataTransfer.types.includes("Files")) e.preventDefault()
      },
      onDrop: (e: DragEvent) => {
        if (!e.dataTransfer.files.length) return
        e.preventDefault()
        add(e.dataTransfer.files)
      },
    },
    // The hidden file input the clip opens; render it once in the box.
    input: (
      <input
        ref={picker}
        type="file"
        multiple
        hidden
        onChange={(e) => {
          add(e.target.files)
          e.target.value = ""
        }}
      />
    ),
  }
}

export type FileAttach = ReturnType<typeof useFileAttach>

// AttachButton is the clip: it opens the file picker.
export function AttachButton({
  attach,
  disabled,
}: {
  attach: FileAttach
  disabled?: boolean
}) {
  return (
    <Button
      type="button"
      size="icon-sm"
      variant="ghost"
      className="shrink-0"
      title="Add files (or drop them here, or paste a screenshot)"
      aria-label="Add files"
      disabled={disabled || attach.full}
      onClick={attach.pick}
    >
      <PaperclipIcon />
    </Button>
  )
}

// PendingFiles lists the files waiting to go, each with an × to take it
// back off.
export function PendingFiles({
  attach,
  disabled,
  label = "Files to send",
  className = "flex flex-wrap gap-2 px-1",
}: {
  attach: FileAttach
  disabled?: boolean
  label?: string
  className?: string
}) {
  if (!attach.files.length) return null
  return (
    <ul className={className} aria-label={label}>
      {attach.files.map((f, i) => (
        <PendingFile
          key={fileKey(f)}
          file={f}
          disabled={!!disabled}
          onRemove={() => attach.remove(i)}
        />
      ))}
    </ul>
  )
}

// fileKey keeps a pending file's row (and its preview) when another one
// before it is removed.
const keys = new WeakMap<File, string>()
let nextKey = 0
function fileKey(f: File): string {
  let k = keys.get(f)
  if (!k) keys.set(f, (k = `f${nextKey++}`))
  return k
}

// Pictures the webview can draw: these show as a thumbnail before sending.
const Previewable = /^image\/(png|jpe?g|gif|webp|bmp|avif|svg\+xml)$/i
const MaxThumb = 32 << 20

// PendingFile is one file waiting to go (card #73): a pasted or picked
// picture shows itself, anything else an icon for its kind and its name.
function PendingFile({
  file,
  disabled,
  onRemove,
}: {
  file: File
  disabled: boolean
  onRemove: () => void
}) {
  const name = file.name || "pasted image"
  const image = Previewable.test(file.type) && file.size <= MaxThumb
  // One object URL per file, let go when the file leaves the list.
  const url = useMemo(
    () => (image ? URL.createObjectURL(file) : null),
    [file, image]
  )
  useEffect(() => () => void (url && URL.revokeObjectURL(url)), [url])
  const remove = (
    <button
      type="button"
      className="text-muted-foreground hover:text-foreground disabled:opacity-50"
      aria-label={`Remove ${name}`}
      title="Remove"
      disabled={disabled}
      onClick={onRemove}
    >
      <XIcon className="size-3" />
    </button>
  )
  if (image && url) {
    return (
      <li className="group relative" title={name}>
        <img
          src={url}
          alt={name}
          draggable={false}
          className="h-20 max-w-40 min-w-12 rounded-md object-cover ring-1 ring-border"
        />
        <span className="absolute top-1 right-1 flex rounded-full bg-background/85 p-0.5 shadow-sm ring-1 ring-border">
          {remove}
        </span>
      </li>
    )
  }
  return (
    <li className="flex h-8 items-center gap-1.5 rounded-md bg-secondary px-2 text-xs ring-1 ring-border">
      <FileKindIcon
        name={file.name}
        mime={file.type}
        className="size-4 shrink-0 text-muted-foreground"
      />
      <span className="max-w-48 truncate" title={name}>
        {name}
      </span>
      {remove}
    </li>
  )
}
