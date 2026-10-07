import { useState } from 'react'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { Check, Clock3, ExternalLink, MessageSquareText } from 'lucide-react'
import { Link } from 'react-router'
import {
  Alert,
  Badge,
  Button,
  Dialog,
  DialogFooter,
  EmptyState,
  PageHeader,
  Spinner,
  TextArea,
} from '@kiteframe/ui'
import { articlesApi, type Article } from '../articles/api'

export default function Reviews() {
  const queryClient = useQueryClient()
  const [rejecting, setRejecting] = useState<Article | null>(null)
  const [reason, setReason] = useState('')
  const query = useQuery({
    queryKey: ['articles', 'IN_REVIEW'],
    queryFn: ({ signal }) => articlesApi.list('IN_REVIEW', signal),
  })
  const decision = useMutation({
    mutationFn: ({ article, action, comment }: { article: Article; action: string; comment?: string }) =>
      articlesApi.action(article.id, action, { comment }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['articles'] })
      setRejecting(null)
      setReason('')
    },
  })

  return (
    <div>
      <PageHeader
        title="Review queue"
        description="Approve what is ready. Return what needs a clearer answer."
        actions={
          <span className="review-count">
            <Clock3 />
            {query.data?.length ?? 0} waiting
          </span>
        }
      />

      {query.isLoading ? (
        <div className="skeleton-page" style={{ display: 'grid', placeItems: 'center', minHeight: 160 }}>
          <Spinner label="Loading review queue" />
        </div>
      ) : query.isError ? (
        <Alert tone="danger" title="Could not load review queue">
          {(query.error as Error).message}
        </Alert>
      ) : query.data?.length ? (
        <section className="review-list">
          {query.data.map((article) => (
            <article key={article.id}>
              <div className="review-meta">
                <Badge tone="warning">In review</Badge>
                <span>Updated {new Date(article.updated_at).toLocaleDateString()}</span>
              </div>
              <h2>{article.title}</h2>
              <p>{article.short_description || 'No summary was provided.'}</p>
              <div className="review-actions">
                <Link to={`/articles/${article.id}/edit`}>
                  <Button variant="secondary" rightIcon={<ExternalLink />}>
                    Inspect draft
                  </Button>
                </Link>
                <Button
                  variant="secondary"
                  leftIcon={<MessageSquareText />}
                  onClick={() => setRejecting(article)}
                >
                  Request changes
                </Button>
                <Button
                  variant="primary"
                  leftIcon={<Check />}
                  onClick={() => decision.mutate({ article, action: 'approve' })}
                >
                  Approve
                </Button>
              </div>
            </article>
          ))}
        </section>
      ) : (
        <EmptyState
          icon={<Check />}
          title="No articles need review"
          description="New submissions will appear here with their current draft and change summary."
        />
      )}

      <Dialog
        open={Boolean(rejecting)}
        onOpenChange={(open) => {
          if (!open) {
            setRejecting(null)
            setReason('')
          }
        }}
        title={rejecting?.title}
        description="Explain what needs to change"
        footer={
          <DialogFooter>
            <Button
              variant="secondary"
              onClick={() => {
                setRejecting(null)
                setReason('')
              }}
            >
              Cancel
            </Button>
            <Button
              variant="primary"
              disabled={reason.trim().length < 2 || decision.isPending}
              onClick={() =>
                rejecting && decision.mutate({ article: rejecting, action: 'reject', comment: reason })
              }
            >
              Return to draft
            </Button>
          </DialogFooter>
        }
      >
        <TextArea
          label="Feedback for the author"
          autoFocus
          rows={5}
          value={reason}
          onChange={(event) => setReason(event.target.value)}
          placeholder="Give the author a specific next step."
        />
      </Dialog>
    </div>
  )
}
