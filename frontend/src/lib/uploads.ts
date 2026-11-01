import { UploadFile } from "@/gen/models"

// MaxUpload and MaxUploadFiles mirror board_service.go's MaxUpload and
// inbox.MaxAttachments: files added in the window cross the webview bridge
// in memory.
export const MaxUpload = 32 << 20
export const MaxUploadFiles = 10

// uploadOf is a File as BoardService takes it: its name and base64 bytes
// (Go's []byte in JSON).
// checkUploads refuses what the window would: too many files, or one too
// big to cross the bridge.
export function checkUploads(files: File[]) {
  if (files.length > MaxUploadFiles)
    throw new Error(`at most ${MaxUploadFiles} files`)
  const big = files.find((f) => f.size > MaxUpload)
  if (big)
    throw new Error(
      `${big.name || "pasted image"} is over ${MaxUpload >> 20} MiB; ask an agent to attach it from disk`
    )
}

export async function uploadOf(f: File): Promise<UploadFile> {
  const bytes = new Uint8Array(await f.arrayBuffer())
  let bin = ""
  for (let i = 0; i < bytes.length; i += 0x8000)
    bin += String.fromCharCode(...bytes.subarray(i, i + 0x8000))
  return UploadFile({ name: f.name || "pasted.png", data: btoa(bin) })
}
