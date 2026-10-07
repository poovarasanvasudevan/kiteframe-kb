import { useState } from 'react'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { useForm } from 'react-hook-form'
import { zodResolver } from '@hookform/resolvers/zod'
import { BookOpen, ExternalLink, LockKeyhole, Plus, ShieldCheck } from 'lucide-react'
import { z } from 'zod'
import { Link } from 'react-router'
import {
  Alert,
  Button,
  Drawer,
  DrawerFooter,
  EmptyState,
  PageHeader,
  Select,
  Spinner,
  TextArea,
  TextField,
} from '@kiteframe/ui'
import { api, ApiError } from '../../lib/api'

type KnowledgeBase = {
  id: string
  name: string
  slug: string
  description?: string
  visibility: 'PUBLIC' | 'PRIVATE' | 'INTERNAL'
  status: string
  language: string
  updated_at: string
}

const schema = z.object({
  name: z.string().min(2).max(120),
  slug: z.string().regex(/^[a-z0-9]+(?:-[a-z0-9]+)*$/, 'Use lowercase letters, numbers, and single hyphens'),
  description: z.string().max(1000).optional(),
  visibility: z.enum(['PUBLIC', 'PRIVATE', 'INTERNAL']),
})
type FormValues = z.infer<typeof schema>

const icons = { PUBLIC: ExternalLink, PRIVATE: LockKeyhole, INTERNAL: ShieldCheck }

export default function KnowledgeBases() {
  const [creating, setCreating] = useState(false)
  const queryClient = useQueryClient()
  const query = useQuery({
    queryKey: ['knowledge-bases'],
    queryFn: ({ signal }) => api<KnowledgeBase[]>('/api/v1/knowledge-bases', { signal }),
  })
  const form = useForm<FormValues>({
    resolver: zodResolver(schema),
    defaultValues: { name: '', slug: '', description: '', visibility: 'INTERNAL' },
  })
  const create = useMutation({
    mutationFn: (values: FormValues) =>
      api<KnowledgeBase>('/api/v1/knowledge-bases', { method: 'POST', body: JSON.stringify(values) }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['knowledge-bases'] })
      setCreating(false)
      form.reset()
    },
  })

  return (
    <div>
      <PageHeader
        title="Knowledge bases"
        description="Separate audiences without fragmenting governance."
        actions={
          <Button variant="primary" leftIcon={<Plus aria-hidden />} onClick={() => setCreating(true)}>
            New knowledge base
          </Button>
        }
      />

      {query.isLoading ? (
        <div className="skeleton-page" style={{ display: 'grid', placeItems: 'center', minHeight: 160 }}>
          <Spinner label="Loading knowledge bases" />
        </div>
      ) : query.isError ? (
        <Alert tone="danger" title="Could not load knowledge bases">
          {query.error instanceof ApiError && query.error.status === 401
            ? 'Sign in to connect this workspace'
            : query.error.message}
        </Alert>
      ) : query.data?.length ? (
        <section className="kb-list" aria-label="Knowledge bases">
          {query.data.map((kb) => {
            const Icon = icons[kb.visibility]
            return (
              <article key={kb.id}>
                <div className="kb-icon">
                  <BookOpen aria-hidden />
                </div>
                <div className="kb-copy">
                  <div>
                    <h2>{kb.name}</h2>
                    <span className="visibility">
                      <Icon aria-hidden />
                      {kb.visibility.toLowerCase()}
                    </span>
                  </div>
                  <p>{kb.description || 'No description yet.'}</p>
                  <small>
                    /help/{kb.slug} · {kb.language.toUpperCase()}
                  </small>
                </div>
                <Link to={`/knowledge/${kb.id}/categories`}>
                  <Button variant="secondary" rightIcon={<ExternalLink aria-hidden />}>
                    Open
                  </Button>
                </Link>
              </article>
            )
          })}
        </section>
      ) : (
        <EmptyState
          icon={<BookOpen />}
          title="Create the first home for your knowledge"
          description="Start with one audience. You can add public, private, or internal spaces as the content model grows."
          actions={
            <Button variant="primary" leftIcon={<Plus />} onClick={() => setCreating(true)}>
              Create knowledge base
            </Button>
          }
        />
      )}

      <Drawer
        open={creating}
        onOpenChange={(open) => {
          setCreating(open)
          if (!open) form.reset()
        }}
        side="right"
        size="md"
        title="Knowledge base details"
        description="New space"
        footer={
          <DrawerFooter>
            <Button type="button" variant="secondary" onClick={() => setCreating(false)}>
              Cancel
            </Button>
            <Button
              type="submit"
              form="create-kb-form"
              variant="primary"
              disabled={create.isPending}
            >
              {create.isPending ? 'Creating…' : 'Create knowledge base'}
            </Button>
          </DrawerFooter>
        }
      >
        <form
          id="create-kb-form"
          style={{ display: 'grid', gap: 14 }}
          onSubmit={form.handleSubmit((values) => create.mutate(values))}
        >
          <TextField
            label="Name"
            {...form.register('name')}
            onChange={(event) => {
              form.setValue('name', event.target.value)
              if (!form.formState.dirtyFields.slug) {
                form.setValue(
                  'slug',
                  event.target.value
                    .toLowerCase()
                    .trim()
                    .replace(/[^a-z0-9]+/g, '-')
                    .replace(/^-|-$/g, ''),
                )
              }
            }}
            placeholder="Customer support"
            error={form.formState.errors.name?.message}
          />
          <TextField
            label="URL slug"
            {...form.register('slug')}
            placeholder="customer-support"
            hint="Published at /help/{slug}"
            error={form.formState.errors.slug?.message}
          />
          <TextArea
            label="Description"
            {...form.register('description')}
            rows={4}
            placeholder="What belongs in this knowledge base?"
          />
          <Select
            label="Visibility"
            value={form.watch('visibility')}
            onValueChange={(value) =>
              form.setValue('visibility', value as FormValues['visibility'])
            }
            options={[
              { value: 'PUBLIC', label: 'Public — Anyone can read published content' },
              { value: 'INTERNAL', label: 'Internal — Selected members and teams' },
              { value: 'PRIVATE', label: 'Private — Signed-in workspace members' },
            ]}
          />
          {create.error ? (
            <Alert tone="danger">
              {create.error instanceof ApiError
                ? create.error.message
                : 'Could not create this knowledge base.'}
            </Alert>
          ) : null}
        </form>
      </Drawer>
    </div>
  )
}
