import type { TFunction } from 'i18next'
import type { WorkflowStatus } from '../types/models'

// The built-in "シンプル" workflow's 4 statuses keep using the existing
// ticket.status.* translation keys (so ja/en labels stay hand-translated).
// Any other status - custom workflows, or a renamed key that no longer
// matches a translation key - falls back to whatever label is stored on the
// status itself, which is untranslated free text (same as a project name).
export function statusLabelByKey(t: TFunction, key: string, label: string): string {
  return t(`ticket.status.${key}` as const, { defaultValue: label })
}

export function statusLabel(t: TFunction, status: WorkflowStatus): string {
  return statusLabelByKey(t, status.key, status.label)
}
