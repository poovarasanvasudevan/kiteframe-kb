import { useState } from 'react'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { Plus, ShieldCheck, Users } from 'lucide-react'
import {
  Avatar,
  Badge,
  Button,
  Dialog,
  DialogFooter,
  PageHeader,
  Tab,
  TabList,
  TabPanel,
  Tabs,
  TextArea,
  TextField,
} from '@kiteframe/ui'
import { api } from '../../lib/api'

type User = { id: string; name: string; email: string; status: string; last_login_at?: string; created_at: string }
type Team = { id: string; name: string; description?: string; member_count: number; created_at: string }
type Audit = { id: string; action: string; resource_type: string; resource_id?: string; created_at: string }

export default function People() {
  const [tab, setTab] = useState('users')
  const [creating, setCreating] = useState(false)
  const [name, setName] = useState('')
  const [description, setDescription] = useState('')
  const queryClient = useQueryClient()
  const users = useQuery({
    queryKey: ['users'],
    enabled: tab === 'users',
    queryFn: ({ signal }) => api<User[]>('/api/v1/users', { signal }),
  })
  const teams = useQuery({
    queryKey: ['teams'],
    enabled: tab === 'teams',
    queryFn: ({ signal }) => api<Team[]>('/api/v1/teams', { signal }),
  })
  const audit = useQuery({
    queryKey: ['audit'],
    enabled: tab === 'audit',
    queryFn: ({ signal }) => api<Audit[]>('/api/v1/audit-logs', { signal }),
  })
  const createTeam = useMutation({
    mutationFn: () =>
      api<Team>('/api/v1/teams', {
        method: 'POST',
        body: JSON.stringify({ name, description: description || null }),
      }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['teams'] })
      setCreating(false)
      setName('')
      setDescription('')
    },
  })

  return (
    <div>
      <PageHeader
        title="People & teams"
        description="Keep membership, working groups, and important actions visible."
        actions={
          tab === 'teams' ? (
            <Button variant="primary" leftIcon={<Plus />} onClick={() => setCreating(true)}>
              New team
            </Button>
          ) : undefined
        }
      />

      <Tabs value={tab} defaultValue="users" onValueChange={setTab}>
        <TabList aria-label="People sections">
          <Tab value="users">Users</Tab>
          <Tab value="teams">Teams</Tab>
          <Tab value="audit">Audit log</Tab>
        </TabList>

        <TabPanel value="users">
          <section className="people-list">
            {users.data?.map((user) => (
              <article key={user.id}>
                <Avatar name={user.name} size="md" />
                <div>
                  <strong>{user.name}</strong>
                  <small>{user.email}</small>
                </div>
                <Badge tone={user.status === 'ACTIVE' ? 'success' : 'neutral'}>{user.status}</Badge>
                <small>
                  {user.last_login_at
                    ? `Last seen ${new Date(user.last_login_at).toLocaleDateString()}`
                    : 'Never signed in'}
                </small>
              </article>
            ))}
          </section>
        </TabPanel>

        <TabPanel value="teams">
          <section className="team-grid">
            {teams.data?.map((team) => (
              <article key={team.id}>
                <span className="team-icon">
                  <Users />
                </span>
                <h2>{team.name}</h2>
                <p>{team.description || 'No description.'}</p>
                <footer>
                  <strong>{team.member_count}</strong>
                  <span>members</span>
                </footer>
              </article>
            ))}
          </section>
        </TabPanel>

        <TabPanel value="audit">
          <section className="audit-list">
            {audit.data?.map((entry) => (
              <article key={entry.id}>
                <span>
                  <ShieldCheck />
                </span>
                <div>
                  <strong>{entry.action.replaceAll('_', ' ').toLowerCase()}</strong>
                  <small>
                    {entry.resource_type}
                    {entry.resource_id ? ` · ${entry.resource_id.slice(0, 8)}` : ''}
                  </small>
                </div>
                <time>{new Date(entry.created_at).toLocaleString()}</time>
              </article>
            ))}
          </section>
        </TabPanel>
      </Tabs>

      <Dialog
        open={creating}
        onOpenChange={setCreating}
        title="Create a team"
        description="Working group"
        footer={
          <DialogFooter>
            <Button variant="secondary" onClick={() => setCreating(false)}>
              Cancel
            </Button>
            <Button
              variant="primary"
              disabled={name.trim().length < 2 || createTeam.isPending}
              onClick={() => createTeam.mutate()}
            >
              Create team
            </Button>
          </DialogFooter>
        }
      >
        <div style={{ display: 'grid', gap: 12 }}>
          <TextField
            label="Team name"
            value={name}
            onChange={(event) => setName(event.target.value)}
            placeholder="Customer support"
          />
          <TextArea
            label="Description"
            value={description}
            onChange={(event) => setDescription(event.target.value)}
            rows={3}
          />
        </div>
      </Dialog>
    </div>
  )
}
