// copyText puts text on the clipboard: the host's own in the app (the
// webview may refuse navigator.clipboard), the browser's otherwise. It
// resolves false when neither could.

import { Clipboard } from "./host"
import { inHost } from "./irori"

export async function copyText(text: string): Promise<boolean> {
  if (inHost()) {
    try {
      await Clipboard.SetText(text)
      return true
    } catch {
      // Fall through to the browser's.
    }
  }
  try {
    await navigator.clipboard.writeText(text)
    return true
  } catch {
    return false
  }
}

// clipboardImage is the picture on the clipboard as a PNG file, read by
// the host (card #321): a paste the webview says carries files but hands
// over none (WebKit with a HEIC-only clipboard, a macOS screenshot). It
// rejects outside the app or when the clipboard holds no picture.
export async function clipboardImage(): Promise<File> {
  if (!inHost()) throw new Error("no picture on the clipboard")
  const { rgba, width, height } = await Clipboard.Image()
  if (!width || !height) throw new Error("no picture on the clipboard")
  const canvas = document.createElement("canvas")
  canvas.width = width
  canvas.height = height
  const ctx = canvas.getContext("2d")
  if (!ctx) throw new Error("can't draw the picture")
  const px = new Uint8ClampedArray(width * height * 4)
  px.set(rgba.subarray(0, px.length))
  ctx.putImageData(new ImageData(px, width, height), 0, 0)
  const png = await new Promise<Blob | null>((ok) =>
    canvas.toBlob(ok, "image/png")
  )
  if (!png) throw new Error("can't make a PNG of the picture")
  return new File([png], "image.png", { type: "image/png" })
}
