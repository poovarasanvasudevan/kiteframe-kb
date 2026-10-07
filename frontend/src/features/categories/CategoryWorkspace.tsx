import { useMemo, useState, type CSSProperties } from 'react'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { zodResolver } from '@hookform/resolvers/zod'
import { useForm } from 'react-hook-form'
import {
  ArrowDown,
  ArrowLeft,
  ArrowUp,
  ChevronDown,
  ChevronRight,
  Folder,
  GripVertical,
  Plus,
} from 'lucide-react'
import { Link, useNavigate, useParams } from 'react-router'
import { z } from 'zod'
import {
  Alert,
  Breadcrumb,
  Button,
  Drawer,
  DrawerFooter,
  PageHeader,
  Select,
  TextArea,
  TextField,
} from '@kiteframe/ui'
import { api, ApiError } from '../../lib/api'

type Category = {
  id: string
  knowledge_base_id: string
  parent_id: string | null
  name: string
  slug: string
  description?: string
  visibility: 'PUBLIC' | 'PRIVATE' | 'INTERNAL'
  sort_order: number
}
type TreeNode = Category & { children: TreeNode[] }

const schema = z.object({
  name: z.string().min(2, 'Enter at least two characters').max(120),
  slug: z.string().regex(/^[a-z0-9]+(?:-[a-z0-9]+)*$/, 'Use lowercase letters, numbers, and single hyphens'),
  description: z.string().max(1000).optional(),
  parent_id: z.string().optional(),
  visibility: z.enum(['PUBLIC', 'PRIVATE', 'INTERNAL']),
})
type FormValues = z.infer<typeof schema>

export default function CategoryWorkspace() {
  const { knowledgeBaseId = '' } = useParams()
  const navigate = useNavigate()
  const [creating, setCreating] = useState(false)
  const [collapsed, setCollapsed] = useState<Set<string>>(new Set())
  const [dragging, setDragging] = useState<string | null>(null)
  const queryClient = useQueryClient()
  const queryKey = ['categories', knowledgeBaseId]
  const query = useQuery({
    queryKey,
    enabled: Boolean(knowledgeBaseId),
    queryFn: ({ signal }) =>
      api<Category[]>(`/api/v1/categories/by-knowledge-base/${knowledgeBaseId}`, { signal }),
  })
  const tree = useMemo(() => buildTree(query.data ?? []), [query.data])
  const form = useForm<FormValues>({
    resolver: zodResolver(schema),
    defaultValues: { name: '', slug: '', description: '', parent_id: '', visibility: 'PUBLIC' },
  })
  const create = useMutation({
    mutationFn: (values: FormValues) =>
      api<Category>('/api/v1/categories', {
        method: 'POST',
        body: JSON.stringify({
          ...values,
          knowledge_base_id: knowledgeBaseId,
          parent_id: values.parent_id || null,
        }),
      }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey })
      setCreating(false)
      form.reset()
    },
  })
  const reorder = useMutation({
    mutationFn: (categoryIds: string[]) =>
      api<Category[]>('/api/v1/categories/reorder', {
        method: 'POST',
        body: JSON.stringify({ category_ids: categoryIds }),
      }),
    onSuccess: () => queryClient.invalidateQueries({ queryKey }),
  })

  const moveSibling = (category: Category, direction: -1 | 1) => {
    const siblings = (query.data ?? [])
      .filter((item) => item.parent_id === category.parent_id)
      .sort((a, b) => a.sort_order - b.sort_order)
    const current = siblings.findIndex((item) => item.id === category.id)
    const target = current + direction
    if (target < 0 || target >= siblings.length) return
    const reordered = [...siblings]
    ;[reordered[current], reordered[target]] = [reordered[target], reordered[current]]
    reorder.mutate(reordered.map((item) => item.id))
  }

  const dropBefore = (target: Category) => {
    if (!dragging || dragging === target.id) return setDragging(null)
    const source = query.data?.find((item) => item.id === dragging)
    if (!source || source.parent_id !== target.parent_id) return setDragging(null)
    const siblings = (query.data ?? [])
      .filter((item) => item.parent_id === target.parent_id)
      .sort((a, b) => a.sort_order - b.sort_order)
    const without = siblings.filter((item) => item.id !== source.id)
    without.splice(
      without.findIndex((item) => item.id === target.id),
      0,
      source,
    )
    reorder.mutate(without.map((item) => item.id))
    setDragging(null)
  }

  return (
    <div>
      <PageHeader
        breadcrumbs={
          <Breadcrumb
            items={[
              { id: 'kb', label: 'Knowledge bases', onClick: () => navigate('/knowledge') },
              { id: 'cat', label: 'Categories', current: true },
            ]}
          />
        }
        title="Structure the knowledge"
        description="Arrange the navigation readers and writers will share."
        actions={
          <Button variant="primary" leftIcon={<Plus />} onClick={() => setCreating(true)}>
            New category
          </Button>
        }
      >
        <Link to="/knowledge" className="back-link" style={{ display: 'inline-flex', alignItems: 'center', gap: 6 }}>
          <ArrowLeft size={14} /> Knowledge bases
        </Link>
      </PageHeader>

      <div className="category-layout">
        <section className="tree-panel">
          <header>
            <div>
              <span className="eyebrow">Navigation tree</span>
              <h2>Categories</h2>
            </div>
            <span className="count-label">{query.data?.length ?? 0} total</span>
          </header>
          {query.isLoading ? (
            <div className="tree-loading">
              <span />
              <span />
              <span />
            </div>
          ) : query.isError ? (
            <Alert tone="danger" title="Categories could not load">
              {query.error.message}
            </Alert>
          ) : tree.length ? (
            <ul className="category-tree">
              {tree.map((node) => (
                <CategoryRow
                  key={node.id}
                  node={node}
                  depth={0}
                  collapsed={collapsed}
                  setCollapsed={setCollapsed}
                  onMove={moveSibling}
                  onDrag={setDragging}
                  onDrop={dropBefore}
                  pending={reorder.isPending}
                />
              ))}
            </ul>
          ) : (
            <div className="tree-message">
              <Folder />
              <strong>No categories yet</strong>
              <p>Create a top-level category, then add nested sections as the content grows.</p>
              <Button variant="secondary" leftIcon={<Plus />} onClick={() => setCreating(true)}>
                Create category
              </Button>
            </div>
          )}
        </section>
        <aside className="category-guidance">
          <span className="eyebrow">A useful tree</span>
          <h2>Keep paths predictable.</h2>
          <p>
            Start broad. Use nested categories only when readers need another decision before reaching an
            article.
          </p>
          <ol>
            <li>
              <span>01</span>Use task language
            </li>
            <li>
              <span>02</span>Keep sibling labels distinct
            </li>
            <li>
              <span>03</span>Put frequent paths first
            </li>
          </ol>
          <small>Drag siblings to reorder. Keyboard users can use the up and down actions on every row.</small>
        </aside>
      </div>

      <Drawer
        open={creating}
        onOpenChange={(open) => {
          setCreating(open)
          if (!open) form.reset()
        }}
        title="Category details"
        description="New section"
        footer={
          <DrawerFooter>
            <Button type="button" variant="secondary" onClick={() => setCreating(false)}>
              Cancel
            </Button>
            <Button type="submit" form="create-category-form" variant="primary" disabled={create.isPending}>
              {create.isPending ? 'Creating…' : 'Create category'}
            </Button>
          </DrawerFooter>
        }
      >
        <form
          id="create-category-form"
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
            placeholder="Getting started"
            error={form.formState.errors.name?.message}
          />
          <TextField
            label="Slug"
            {...form.register('slug')}
            placeholder="getting-started"
            error={form.formState.errors.slug?.message}
          />
          <Select
            label="Parent category"
            value={form.watch('parent_id') || ''}
            onValueChange={(value) => form.setValue('parent_id', value)}
            options={[
              { value: '', label: 'Top level' },
              ...flattenTree(tree).map(({ category, depth }) => ({
                value: category.id,
                label: `${'— '.repeat(depth)}${category.name}`,
              })),
            ]}
          />
          <TextArea
            label="Description"
            {...form.register('description')}
            rows={3}
            placeholder="What readers will find here"
          />
          <Select
            label="Visibility"
            value={form.watch('visibility')}
            onValueChange={(value) =>
              form.setValue('visibility', value as FormValues['visibility'])
            }
            options={[
              { value: 'PUBLIC', label: 'Public — Visible in a public knowledge base' },
              { value: 'INTERNAL', label: 'Internal — Limited to granted teams' },
              { value: 'PRIVATE', label: 'Private — Authenticated members' },
            ]}
          />
          {create.error ? (
            <Alert tone="danger">
              {create.error instanceof ApiError
                ? create.error.message
                : 'Could not create this category.'}
            </Alert>
          ) : null}
        </form>
      </Drawer>
    </div>
  )
}

function CategoryRow({
  node,
  depth,
  collapsed,
  setCollapsed,
  onMove,
  onDrag,
  onDrop,
  pending,
}: {
  node: TreeNode
  depth: number
  collapsed: Set<string>
  setCollapsed: (value: Set<string>) => void
  onMove: (category: Category, direction: -1 | 1) => void
  onDrag: (id: string | null) => void
  onDrop: (category: Category) => void
  pending: boolean
}) {
  const isCollapsed = collapsed.has(node.id)
  const toggle = () => {
    const next = new Set(collapsed)
    isCollapsed ? next.delete(node.id) : next.add(node.id)
    setCollapsed(next)
  }
  return (
    <li>
      <div
        className="category-row"
        style={{ '--tree-depth': depth } as CSSProperties}
        draggable
        onDragStart={() => onDrag(node.id)}
        onDragEnd={() => onDrag(null)}
        onDragOver={(event) => event.preventDefault()}
        onDrop={() => onDrop(node)}
      >
        <GripVertical className="drag-handle" aria-hidden />
        {node.children.length ? (
          <button
            type="button"
            className="tree-toggle"
            onClick={toggle}
            aria-label={`${isCollapsed ? 'Expand' : 'Collapse'} ${node.name}`}
          >
            {isCollapsed ? <ChevronRight /> : <ChevronDown />}
          </button>
        ) : (
          <span className="tree-spacer" />
        )}
        <span className="folder-mark">
          <Folder />
        </span>
        <span className="category-name">
          <strong>{node.name}</strong>
          <small>
            /{node.slug} · {node.visibility.toLowerCase()}
          </small>
        </span>
        <span className="category-actions">
          <Button
            variant="ghost"
            iconOnly
            size="sm"
            disabled={pending}
            onClick={() => onMove(node, -1)}
            aria-label={`Move ${node.name} up`}
          >
            <ArrowUp />
          </Button>
          <Button
            variant="ghost"
            iconOnly
            size="sm"
            disabled={pending}
            onClick={() => onMove(node, 1)}
            aria-label={`Move ${node.name} down`}
          >
            <ArrowDown />
          </Button>
        </span>
      </div>
      {!isCollapsed && node.children.length > 0 && (
        <ul>
          {node.children.map((child) => (
            <CategoryRow
              key={child.id}
              node={child}
              depth={depth + 1}
              collapsed={collapsed}
              setCollapsed={setCollapsed}
              onMove={onMove}
              onDrag={onDrag}
              onDrop={onDrop}
              pending={pending}
            />
          ))}
        </ul>
      )}
    </li>
  )
}

function buildTree(categories: Category[]): TreeNode[] {
  const nodes = new Map(categories.map((category) => [category.id, { ...category, children: [] as TreeNode[] }]))
  const roots: TreeNode[] = []
  for (const node of nodes.values()) {
    const parent = node.parent_id ? nodes.get(node.parent_id) : undefined
    if (parent) parent.children.push(node)
    else roots.push(node)
  }
  const sort = (items: TreeNode[]) => {
    items.sort((a, b) => a.sort_order - b.sort_order || a.name.localeCompare(b.name))
    items.forEach((item) => sort(item.children))
  }
  sort(roots)
  return roots
}

function flattenTree(tree: TreeNode[], depth = 0): { category: TreeNode; depth: number }[] {
  return tree.flatMap((category) => [
    { category, depth },
    ...flattenTree(category.children, depth + 1),
  ])
}
