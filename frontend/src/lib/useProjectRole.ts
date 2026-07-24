import { useQuery } from '@tanstack/react-query'
import { useAuth } from '../context/AuthContext'
import { listProjectMembers } from '../api/projectMembers'

// Mirrors the canManage logic that used to live only in ProjectMembersPage -
// pulled out so any project-scoped page (not just the Members tab) can gate
// manager-only actions the same way. Shares the ['project', id, 'members']
// query key so it reuses the cache other project pages already populate.
export function useProjectRole(projectId: number | undefined) {
  const { user: currentUser } = useAuth()
  const { data: members } = useQuery({
    queryKey: ['project', projectId, 'members'],
    queryFn: () => listProjectMembers(projectId!),
    enabled: projectId !== undefined,
  })

  const isSiteAdmin = currentUser?.role === 'admin'
  const isManager = members?.some((m) => m.user.id === currentUser?.id && m.role === 'manager') ?? false
  return { isSiteAdmin, isManager, canManage: isSiteAdmin || isManager }
}
