import { useTranslation } from 'react-i18next'
import { STORY_POINT_OPTIONS } from '../lib/storyPoints'

export function StoryPointGuide() {
  const { t } = useTranslation()

  return (
    <span className="group relative inline-flex cursor-help" tabIndex={0} aria-describedby="story-point-guide">
      {t('ticket.detail.storyPointsLabel')}
      <span
        id="story-point-guide"
        role="tooltip"
        className="pointer-events-none absolute bottom-full left-0 z-20 mb-2 hidden w-80 max-w-[calc(100vw-3rem)] rounded border border-blue-100 bg-blue-50 px-3 py-2 text-left text-xs font-normal text-blue-900 shadow-lg group-hover:block group-focus:block"
      >
        <span className="block font-medium">{t('ticket.detail.storyPointsGuideTitle')}</span>
        <span className="mt-1 block text-blue-800">{t('ticket.detail.storyPointsGuideDescription')}</span>
        <span className="mt-2 grid grid-cols-1 gap-x-3 gap-y-1 sm:grid-cols-2">
          {STORY_POINT_OPTIONS.map((point) => (
            <span key={point}>
              <span className="mr-1 font-semibold">{point}SP</span>
              <span>{t(`ticket.detail.storyPointsGuide.${point}`)}</span>
            </span>
          ))}
        </span>
      </span>
    </span>
  )
}
