import { describe, expect, test } from "bun:test"
import {
  FileArchiveIcon,
  FileCodeIcon,
  FileIcon,
  FileImageIcon,
  FileSpreadsheetIcon,
  FileTextIcon,
} from "lucide-react"

import { fileKindIcon } from "./file-kind-icon"

describe("fileKindIcon", () => {
  test("goes by the extension, any case", () => {
    expect(fileKindIcon("Shot.PNG")).toBe(FileImageIcon)
    expect(fileKindIcon("notes.md")).toBe(FileTextIcon)
    expect(fileKindIcon("data.csv")).toBe(FileSpreadsheetIcon)
    expect(fileKindIcon("main.rs")).toBe(FileCodeIcon)
    expect(fileKindIcon("logs.tar.gz")).toBe(FileArchiveIcon)
  })
  test("falls back to the mime type, then a plain page", () => {
    expect(fileKindIcon("", "image/png")).toBe(FileImageIcon)
    expect(fileKindIcon("blob", "text/plain; charset=utf-8")).toBe(FileTextIcon)
    expect(fileKindIcon(".bashrc")).toBe(FileIcon)
    expect(fileKindIcon("thing.xyz", null)).toBe(FileIcon)
  })
})
