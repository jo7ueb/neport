import { useTranslation } from 'react-i18next'
import { STORY_POINT_OPTIONS } from '../lib/storyPoints'

export function StoryPointGuide() {
  const { t } = useTranslation()

  return (
    <div className="mt-2 rounded border border-blue-100 bg-blue-50 px-3 py-2 text-xs text-blue-900">
      <div className="font-medium">{t('ticket.detail.storyPointsGuideTitle')}</div>
      <div className="mt-1 text-blue-800">{t('ticket.detail.storyPointsGuideDescription')}</div>
      <div className="mt-2 grid grid-cols-1 gap-x-3 gap-y-1 sm:grid-cols-2">
        {STORY_POINT_OPTIONS.map((point) => (
          <div key={point}>
            <span className="mr-1 font-semibold">{point}SP</span>
            <span>{t(`ticket.detail.storyPointsGuide.${point}`)}</span>
          </div>
        ))}
      </div>
    </div>
  )
}
