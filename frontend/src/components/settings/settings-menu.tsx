import { useState } from "react"
import { SettingsIcon } from "lucide-react"

import {
  SettingsDialog,
  settingsSections,
  type SettingsSection,
} from "@/components/settings/settings-dialog"
import { Button } from "@/components/ui/button"
import { previewFlag } from "@/lib/preview"

// SettingsMenu is the header's gear: it opens Settings (card #132), every
// setting in one dialog with its sections down the left.
export function SettingsMenu({ version }: { version?: string }) {
  // ?settings[=<section>] and ?devices open it in the browser preview.
  const [section, setSection] = useState<SettingsSection>(() => {
    if (previewFlag("devices") !== null) return "devices"
    const want = previewFlag("settings")
    return settingsSections.find((s) => s.id === want)?.id ?? "notifications"
  })
  const [open, setOpen] = useState(
    () => previewFlag("settings") !== null || previewFlag("devices") !== null
  )

  return (
    <>
      <Button
        variant="ghost"
        size="icon-sm"
        aria-label="Settings"
        title="Settings"
        onClick={() => setOpen(true)}
      >
        <SettingsIcon />
      </Button>
      <SettingsDialog
        open={open}
        onOpenChange={setOpen}
        section={section}
        onSection={setSection}
        version={version}
      />
    </>
  )
}
