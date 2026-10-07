import { useMemo, useState } from 'react'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { createColumnHelper } from '@tanstack/react-table'
import { Archive, ArrowUpRight, CheckCircle2, FilePenLine, MoreHorizontal, Plus } from 'lucide-react'
import { Link, useNavigate } from 'react-router'
import {
  Alert,
  Button,
  Chip,
  DataTable,
  EmptyState,
  IconMenuItem,
  Menu,
  PageHeader,
  Spinner,
  Tab,
  TabList,
  Tabs,
} from '@kiteframe/ui'
import { articlesApi, type Article, type ArticleStatus } from './api'

const statuses: Array<ArticleStatus | 'ALL'> = ['ALL', 'DRAFT', 'IN_REVIEW', 'APPROVED', 'PUBLISHED', 'ARCHIVED']
const columnHelper = createColumnHelper<Article>()

export default function Articles() {
  const [status, setStatus] = useState<ArticleStatus | 'ALL'>('ALL')
  const navigate = useNavigate()
  const queryClient = useQueryClient()
  const query = useQuery({
    queryKey: ['articles', status],
    queryFn: ({ signal }) => articlesApi.list(status === 'ALL' ? undefined : status, signal),
  })
  const action = useMutation({
    mutationFn: ({ id, name }: { id: string; name: string }) => articlesApi.action(id, name),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['articles'] }),
  })

  const columns = useMemo(
    () => [
      columnHelper.accessor('title', {
        header: 'Article',
        cell: (info) => {
          const article = info.row.original
          return (
            <Link className="article-title-cell" to={`/articles/${article.id}/edit`}>
              <span className="document-glyph">
                <FilePenLine />
              </span>
              <span>
                <strong>{article.title}</strong>
                <small>/{article.slug}</small>
              </span>
            </Link>
          )
        },
      }),
      columnHelper.accessor('status', {
        header: 'Status',
        cell: (info) => (
          <Chip tone={statusTone(info.getValue())}>{info.getValue().replace('_', ' ')}</Chip>
        ),
      }),
      columnHelper.accessor('visibility', {
        header: 'Visibility',
        cell: (info) => <span className="table-muted">{info.getValue().toLowerCase()}</span>,
      }),
      columnHelper.accessor('updated_at', {
        header: 'Updated',
        cell: (info) => <span className="table-muted">{new Date(info.getValue()).toLocaleDateString()}</span>,
      }),
      columnHelper.display({
        id: 'actions',
        header: () => <span className="sr-only">Actions</span>,
        cell: ({ row }) => {
          const article = row.original
          return (
            <div className="row-actions">
              <Link aria-label={`Edit ${article.title}`} to={`/articles/${article.id}/edit`}>
                <ArrowUpRight />
              </Link>
              {article.status === 'DRAFT' && (
                <button aria-label={`Submit ${article.title} for review`} onClick={() => action.mutate({ id: article.id, name: 'submit-review' })}>
                  <CheckCircle2 />
                </button>
              )}
              {article.status === 'PUBLISHED' && (
                <button aria-label={`Archive ${article.title}`} onClick={() => action.mutate({ id: article.id, name: 'archive' })}>
                  <Archive />
                </button>
              )}
              <Menu
                label={`Actions for ${article.title}`}
                trigger={
                  <Button variant="ghost" iconOnly size="sm" aria-label={`More actions for ${article.title}`}>
                    <MoreHorizontal />
                  </Button>
                }
              >
                <IconMenuItem
                  icon={<ArrowUpRight />}
                  onClick={() => navigate(`/articles/${article.id}/edit`)}
                >
                  Open
                </IconMenuItem>
                {article.status === 'DRAFT' ? (
                  <IconMenuItem
                    icon={<CheckCircle2 />}
                    onClick={() => action.mutate({ id: article.id, name: 'submit-review' })}
                  >
                    Submit for review
                  </IconMenuItem>
                ) : null}
                {article.status === 'PUBLISHED' ? (
                  <IconMenuItem
                    icon={<Archive />}
                    onClick={() => action.mutate({ id: article.id, name: 'archive' })}
                  >
                    Archive
                  </IconMenuItem>
                ) : null}
              </Menu>
            </div>
          )
        },
      }),
    ],
    [action, navigate],
  )

  return (
    <div>
      <PageHeader
        title="Articles"
        description="Draft, review, publish, and maintain every knowledge asset."
        actions={
          <Link to="/articles/new">
            <Button variant="primary" leftIcon={<Plus />}>
              New article
            </Button>
          </Link>
        }
      />

      <Tabs value={status} defaultValue="ALL" onValueChange={(value) => setStatus(value as ArticleStatus | 'ALL')}>
        <TabList aria-label="Article status" style={{ marginBottom: 12 }}>
          {statuses.map((value) => (
            <Tab key={value} value={value}>
              {value.replace('_', ' ').toLowerCase()}
            </Tab>
          ))}
        </TabList>
      </Tabs>

      {query.isLoading ? (
        <div className="skeleton-page" style={{ display: 'grid', placeItems: 'center', minHeight: 160 }}>
          <Spinner label="Loading articles" />
        </div>
      ) : query.isError ? (
        <Alert tone="danger" title="Articles could not load">
          {query.error.message}
        </Alert>
      ) : (
        <DataTable
          data={query.data ?? []}
          columns={columns}
          searchPlaceholder="Search article titles"
          empty={
            <EmptyState
              icon={<FilePenLine />}
              title="Write the first useful answer"
              description="Start with a question readers ask often and give it one clear outcome."
              actions={
                <Link to="/articles/new">
                  <Button variant="primary" leftIcon={<Plus />}>
                    New article
                  </Button>
                </Link>
              }
            />
          }
          getRowId={(row) => row.id}
        />
      )}
    </div>
  )
}

function statusTone(status: ArticleStatus) {
  if (status === 'PUBLISHED' || status === 'APPROVED') return 'success' as const
  if (status === 'IN_REVIEW') return 'warning' as const
  if (status === 'ARCHIVED') return 'neutral' as const
  return 'accent' as const
}
