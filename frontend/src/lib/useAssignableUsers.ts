import { useQuery } from '@tanstack/react-query'
import { listProjectMembers } from '../api/projectMembers'
import { listAllUsers } from '../api/users'
import type { User } from '../types/models'

// Who can be picked as a ticket's assignee within a project: its members,
// normally - except a legacy project with no project_members rows at all is
// open to every user (see backend/src/access.rs and the "legacyOpenNotice"
// copy on the members page), so falling back to the full user list there
// keeps assignment working instead of showing an empty picker.
export function useAssignableUsers(projectId: number | undefined): User[] | undefined {
  const { data: members } = useQuery({
    queryKey: ['project', projectId, 'members'],
    queryFn: () => listProjectMembers(projectId!),
    enabled: projectId !== undefined,
  })
  const { data: allUsers } = useQuery({
    queryKey: ['users', 'all'],
    queryFn: listAllUsers,
    enabled: members?.length === 0,
  })

  if (members === undefined) return undefined
  return members.length > 0 ? members.map((m) => m.user) : allUsers
}
