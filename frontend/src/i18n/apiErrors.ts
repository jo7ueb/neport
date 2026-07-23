import type { TFunction } from 'i18next'
import { ApiError } from '../api/client'

// The backend (backend/src/error.rs, plus scattered AppError::BadRequest(...)
// call sites) always returns fixed English message strings. This maps those
// exact strings to translation keys so they render in the user's chosen
// language; anything not in this table (e.g. a future backend message we
// haven't mapped yet) falls back to the raw English text, which is a safe
// default since the backend already speaks English.
const apiErrorKeyByMessage: Record<string, string> = {
  'Invalid credentials': 'apiErrors.invalidCredentials',
  'Unauthorized': 'apiErrors.unauthorized',
  'Forbidden': 'apiErrors.forbidden',
  'CSRF token missing or invalid': 'apiErrors.invalidCsrf',
  'Not found': 'apiErrors.notFound',
  'Internal server error': 'apiErrors.internalServerError',
  'cannot change your own role': 'apiErrors.cannotChangeOwnRole',
  'cannot delete your own account': 'apiErrors.cannotDeleteOwnAccount',
  'cannot suspend your own account': 'apiErrors.cannotSuspendOwnAccount',
  'invalid year/month': 'apiErrors.invalidYearMonth',
  'no work session is currently in progress': 'apiErrors.noActiveWorkSession',
  'project name already exists': 'apiErrors.projectNameExists',
  'work is already in progress': 'apiErrors.workAlreadyInProgress',
  "direction must be 'predecessor' or 'successor'": 'apiErrors.invalidDependencyDirection',
  'a ticket cannot depend on itself': 'apiErrors.selfDependency',
  'this dependency would create a cycle': 'apiErrors.dependencyCycle',
  'a ticket cannot be its own parent': 'apiErrors.selfParent',
  'this move would create a cycle in the ticket hierarchy': 'apiErrors.hierarchyCycle',
  'password must be at least 8 characters': 'apiErrors.passwordTooShort',
  'password must contain both letters and numbers': 'apiErrors.passwordNeedsLetterAndNumber',
  'invalid locale': 'apiErrors.invalidLocale',
  'invalid timezone': 'apiErrors.invalidTimezone',
  "this status transition is not allowed by the project's workflow": 'apiErrors.invalidStatusTransition',
  'all child tickets must be done before this ticket can be closed': 'apiErrors.childTicketsNotDone',
  'this workflow is assigned to one or more projects': 'apiErrors.workflowInUse',
  'this status is used by one or more tickets': 'apiErrors.statusInUse',
  'the default workflow cannot be deleted': 'apiErrors.defaultWorkflowUndeletable',
  'display name cannot be empty': 'apiErrors.displayNameEmpty',
  'unsupported image type (must be PNG, JPEG, WebP, or GIF)': 'apiErrors.unsupportedImageType',
  'image too large (max 2MB)': 'apiErrors.imageTooLarge',
  'this user is already a member of this project': 'apiErrors.userAlreadyMember',
  'a project must have at least one manager': 'apiErrors.projectNeedsOneManager',
  'this user does not have the approver flag enabled': 'apiErrors.approverFlagRequired',
  'this user is already an approver for this user': 'apiErrors.alreadyApprover',
  'this day has no work sessions to decide on': 'apiErrors.noSessionsToDecide',
  'this day is not currently awaiting a decision': 'apiErrors.notAwaitingDecision',
  'this month is already closed': 'apiErrors.monthAlreadyClosed',
  'every day with work sessions this month must be approved before it can be closed': 'apiErrors.notAllDaysApproved',
  'this month is not closed': 'apiErrors.monthNotClosed',
  "this day's report has already been approved and can no longer be edited": 'apiErrors.reportApprovedLocked',
  'this month has been closed and can no longer be edited': 'apiErrors.monthClosedLocked',
  'System is under maintenance': 'apiErrors.maintenanceMode',
}

// These keys come from a runtime string lookup rather than a literal, so
// they can't be statically narrowed to i18next's generated key union; the
// loosely-typed alias below is scoped to this file only.
type LooseT = (key: string, options?: Record<string, unknown>) => string

export function translateApiErrorMessage(message: string, t: TFunction): string {
  const key = apiErrorKeyByMessage[message]
  return key ? (t as unknown as LooseT)(key) : message
}

export function getErrorMessage(err: unknown, t: TFunction, fallbackKey: string): string {
  if (err instanceof ApiError) {
    const message = translateApiErrorMessage(err.message, t)
    if (!err.errorId) return message
    const loose = t as unknown as LooseT
    return message + loose('apiErrors.errorIdSuffix', { id: err.errorId })
  }
  return (t as unknown as LooseT)(fallbackKey)
}
