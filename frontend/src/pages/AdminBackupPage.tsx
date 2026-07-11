import { useRef, useState } from 'react'
import { useMutation } from '@tanstack/react-query'
import { Link } from 'react-router-dom'
import { useTranslation } from 'react-i18next'
import { useAuth } from '../context/AuthContext'
import { useSettings } from '../context/SettingsContext'
import { createBackup, restoreBackup } from '../api/backup'
import { getErrorMessage } from '../i18n/apiErrors'

function downloadBlob(blob: Blob, filename: string) {
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = filename
  a.click()
  URL.revokeObjectURL(url)
}

export function AdminBackupPage() {
  const { t } = useTranslation()
  const { user: currentUser } = useAuth()
  const { maintenanceMode } = useSettings()
  const fileInputRef = useRef<HTMLInputElement>(null)
  const [error, setError] = useState<string | null>(null)
  const [success, setSuccess] = useState<string | null>(null)

  const backupMutation = useMutation({
    mutationFn: createBackup,
    onSuccess: (blob) => {
      const filename = `neport-backup-${new Date().toISOString().replace(/[:.]/g, '-')}.tar.gz`
      downloadBlob(blob, filename)
      setError(null)
      setSuccess(t('adminBackup.backupSuccess'))
    },
    onError: (err) => {
      setSuccess(null)
      setError(getErrorMessage(err, t, 'adminBackup.backupFailed'))
    },
  })

  const restoreMutation = useMutation({
    mutationFn: restoreBackup,
    onSuccess: () => {
      setError(null)
      setSuccess(t('adminBackup.restoreSuccess'))
      if (fileInputRef.current) fileInputRef.current.value = ''
    },
    onError: (err) => {
      setSuccess(null)
      setError(getErrorMessage(err, t, 'adminBackup.restoreFailed'))
    },
  })

  function handleRestoreClick() {
    const file = fileInputRef.current?.files?.[0]
    if (!file) return
    if (window.confirm(t('adminBackup.confirmRestore'))) {
      restoreMutation.mutate(file)
    }
  }

  if (currentUser?.role !== 'admin') {
    return (
      <div className="rounded border bg-white p-6 text-center text-gray-500">
        {t('adminBackup.adminOnly')}
      </div>
    )
  }

  return (
    <div>
      <Link to="/admin" className="mb-4 inline-block text-sm text-blue-600 hover:underline">
        {t('adminHub.backToHub')}
      </Link>
      <h1 className="mb-4 text-2xl font-semibold text-gray-800">{t('adminBackup.title')}</h1>

      {error && <div className="mb-4 rounded bg-red-50 px-3 py-2 text-sm text-red-700">{error}</div>}
      {success && <div className="mb-4 rounded bg-green-50 px-3 py-2 text-sm text-green-700">{success}</div>}

      <div className="max-w-lg space-y-6 rounded border bg-white p-4 text-sm">
        <div>
          <h2 className="mb-1 font-semibold text-gray-800">{t('adminBackup.createTitle')}</h2>
          <p className="mb-3 text-xs text-gray-500">{t('adminBackup.createDescription')}</p>
          <button
            onClick={() => backupMutation.mutate()}
            disabled={backupMutation.isPending}
            className="rounded bg-blue-600 px-4 py-2 text-sm text-white hover:bg-blue-700 disabled:opacity-50"
          >
            {backupMutation.isPending ? t('adminBackup.creating') : t('adminBackup.createButton')}
          </button>
        </div>

        <div className="border-t pt-4">
          <h2 className="mb-1 font-semibold text-gray-800">{t('adminBackup.restoreTitle')}</h2>
          <p className="mb-3 text-xs text-gray-500">{t('adminBackup.restoreDescription')}</p>
          {!maintenanceMode && (
            <div className="mb-3 rounded border border-amber-200 bg-amber-50 px-3 py-2 text-xs text-amber-800">
              {t('adminBackup.maintenanceModeRequired')}
            </div>
          )}
          <div className="flex items-center gap-2">
            <input ref={fileInputRef} type="file" accept=".gz,.tar.gz" className="text-sm" />
            <button
              onClick={handleRestoreClick}
              disabled={restoreMutation.isPending}
              className="rounded bg-red-600 px-4 py-2 text-sm text-white hover:bg-red-700 disabled:opacity-50"
            >
              {restoreMutation.isPending ? t('adminBackup.restoring') : t('adminBackup.restoreButton')}
            </button>
          </div>
        </div>
      </div>
    </div>
  )
}
