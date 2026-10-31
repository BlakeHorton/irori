import { createElement } from "react"
import type { LucideIcon, LucideProps } from "lucide-react"
import {
  FileArchiveIcon,
  FileAudioIcon,
  FileBracesIcon,
  FileCodeIcon,
  FileIcon,
  FileImageIcon,
  FileJsonIcon,
  FileKeyIcon,
  FileSpreadsheetIcon,
  FileTerminalIcon,
  FileTextIcon,
  FileTypeIcon,
  FileVideoIcon,
  PresentationIcon,
} from "lucide-react"

// FileKindIcon draws a file's kind (card #73): a picture, a document, a
// sheet, code, an archive… so a file that can't show as a thumbnail still
// reads as something at a glance. It goes by the extension, then the mime
// type, and falls back to a plain page.
export function FileKindIcon({
  name,
  mime,
  ...props
}: { name: string; mime?: string | null } & LucideProps) {
  // The icon is one of a fixed set, never made here: draw it by reference.
  return createElement(fileKindIcon(name, mime), {
    "aria-hidden": true,
    ...props,
  })
}

const byExt: Record<string, LucideIcon> = {}
function kind(icon: LucideIcon, exts: string) {
  for (const e of exts.split(" ")) byExt[e] = icon
}
kind(
  FileImageIcon,
  "png jpg jpeg gif webp bmp heic heif tif tiff svg avif ico psd"
)
kind(FileVideoIcon, "mp4 mov m4v webm mkv avi")
kind(FileAudioIcon, "mp3 wav m4a aac flac ogg opus aiff")
kind(FileArchiveIcon, "zip tar gz tgz bz2 xz 7z rar zst dmg")
kind(FileSpreadsheetIcon, "csv tsv xls xlsx numbers ods")
kind(PresentationIcon, "ppt pptx key odp")
kind(FileTextIcon, "txt md markdown rtf doc docx pages odt pdf log")
kind(FileJsonIcon, "json jsonl ndjson")
kind(FileBracesIcon, "yaml yml toml xml ini conf cfg plist")
kind(FileTerminalIcon, "sh bash zsh fish ps1 bat cmd")
kind(FileKeyIcon, "pem key crt cer p12")
kind(FileTypeIcon, "ttf otf woff woff2")
kind(
  FileCodeIcon,
  "js jsx ts tsx mjs cjs rs go py rb java kt swift c h cc cpp hpp cs php lua sql html css scss vue svelte"
)

// fileKindIcon picks the icon for a file by its name, else its mime type.
export function fileKindIcon(name: string, mime?: string | null): LucideIcon {
  const dot = name.lastIndexOf(".")
  if (dot > 0) {
    const icon = byExt[name.slice(dot + 1).toLowerCase()]
    if (icon) return icon
  }
  const m = (mime ?? "").split(";")[0].trim().toLowerCase()
  if (m.startsWith("image/")) return FileImageIcon
  if (m.startsWith("video/")) return FileVideoIcon
  if (m.startsWith("audio/")) return FileAudioIcon
  if (m.startsWith("text/")) return FileTextIcon
  if (m === "application/pdf") return FileTextIcon
  if (m === "application/json") return FileJsonIcon
  if (m === "application/zip" || m.includes("compressed"))
    return FileArchiveIcon
  return FileIcon
}
