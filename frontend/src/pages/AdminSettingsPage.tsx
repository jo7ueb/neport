import { useState } from 'react'
import { useMutation } from '@tanstack/react-query'
import { Link } from 'react-router-dom'
import { useTranslation } from 'react-i18next'
import { useSettings } from '../context/SettingsContext'
import { getErrorMessage } from '../i18n/apiErrors'
import { ALL_TIMEZONES, timezoneOptionLabel } from '../lib/timezones'

export function AdminSettingsPage() {
  const { t } = useTranslation()
  const { katexEnabled, defaultTimezone, maintenanceMode, updateSettings } = useSettings()
  const [katexChecked, setKatexChecked] = useState(katexEnabled)
  const [timezone, setTimezone] = useState(defaultTimezone)
  const [maintenanceChecked, setMaintenanceChecked] = useState(maintenanceMode)
  const [error, setError] = useState<string | null>(null)
  const [success, setSuccess] = useState(false)

  const mutation = useMutation({
    mutationFn: () => updateSettings(katexChecked, timezone, maintenanceChecked),
    onSuccess: () => {
      setError(null)
      setSuccess(true)
    },
    onError: (err) => {
      setSuccess(false)
      setError(getErrorMessage(err, t, 'settings.saveFailed'))
    },
  })

  return (
    <div>
      <Link to="/admin" className="mb-4 inline-block text-sm text-blue-600 hover:underline">
        {t('adminHub.backToHub')}
      </Link>
      <h1 className="mb-4 text-2xl font-semibold text-gray-800">{t('settings.title')}</h1>

      <div className="max-w-sm rounded border bg-white p-4 text-sm">
        {error && <div className="mb-3 rounded bg-red-50 px-3 py-2 text-red-700">{error}</div>}
        {success && (
          <div className="mb-3 rounded bg-green-50 px-3 py-2 text-green-700">{t('settings.saveSuccess')}</div>
        )}

        <label className="flex items-start gap-2">
          <input
            type="checkbox"
            checked={katexChecked}
            onChange={(e) => setKatexChecked(e.target.checked)}
            className="mt-0.5"
          />
          <span>
            <span className="block font-medium text-gray-800">{t('settings.katexEnabled')}</span>
            <span className="block text-xs text-gray-500">{t('settings.katexDescription')}</span>
          </span>
        </label>

        <div className="mt-4">
          <label className="mb-1 block font-medium text-gray-800">{t('settings.defaultTimezone')}</label>
          <select
            value={timezone}
            onChange={(e) => setTimezone(e.target.value)}
            className="w-full rounded border border-gray-300 px-2 py-1.5"
          >
            {ALL_TIMEZONES.map((tz) => (
              <option key={tz} value={tz}>
                {timezoneOptionLabel(tz)}
              </option>
            ))}
          </select>
          <p className="mt-1 text-xs text-gray-500">{t('settings.defaultTimezoneDescription')}</p>
        </div>

        <label className="mt-4 flex items-start gap-2 rounded border border-amber-200 bg-amber-50 p-3">
          <input
            type="checkbox"
            checked={maintenanceChecked}
            onChange={(e) => setMaintenanceChecked(e.target.checked)}
            className="mt-0.5"
          />
          <span>
            <span className="block font-medium text-amber-800">{t('settings.maintenanceMode')}</span>
            <span className="block text-xs text-amber-700">{t('settings.maintenanceModeDescription')}</span>
          </span>
        </label>

        <button
          onClick={() => mutation.mutate()}
          disabled={mutation.isPending}
          className="mt-4 rounded bg-blue-600 px-4 py-2 text-sm text-white hover:bg-blue-700 disabled:opacity-50"
        >
          {t('settings.save')}
        </button>
      </div>
    </div>
  )
}
