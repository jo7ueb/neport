import { Link } from 'react-router-dom'
import { useTranslation } from 'react-i18next'

export function NotFoundPage() {
  const { t } = useTranslation()
  return (
    <div className="flex h-screen flex-col items-center justify-center gap-3 text-gray-500">
      <p className="text-xl">{t('notFound.message')}</p>
      <Link to="/" className="text-blue-600 hover:underline">
        {t('notFound.backHome')}
      </Link>
    </div>
  )
}
