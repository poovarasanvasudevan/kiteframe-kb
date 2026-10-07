import { useQuery } from '@tanstack/react-query'
import { ThumbsDown, ThumbsUp } from 'lucide-react'
import { Alert, PageHeader, ProgressBar, Select, Spinner, Stat, StatBar } from '@kiteframe/ui'
import { api } from '../../lib/api'

type Summary = {
  total_articles: number
  published_articles: number
  draft_articles: number
  pending_reviews: number
  article_views: number
  searches: number
  failed_searches: number
  helpful_votes: number
  unhelpful_votes: number
}

export default function Analytics() {
  const query = useQuery({
    queryKey: ['analytics'],
    queryFn: ({ signal }) => api<Summary>('/api/v1/analytics', { signal }),
  })
  const data = query.data
  const helpfulTotal = (data?.helpful_votes ?? 0) + (data?.unhelpful_votes ?? 0)
  const helpfulRate = helpfulTotal ? Math.round((data!.helpful_votes / helpfulTotal) * 100) : 0
  const total = data?.total_articles ?? 0

  return (
    <div>
      <PageHeader
        title="Analytics"
        description="Find content gaps before they become support volume."
        actions={
          <Select
            aria-label="Date range"
            defaultValue="30"
            options={[
              { value: '7', label: 'Last 7 days' },
              { value: '30', label: 'Last 30 days' },
              { value: '90', label: 'Last 90 days' },
            ]}
            className="date-range"
          />
        }
      />

      {query.isLoading ? (
        <div className="skeleton-page" style={{ display: 'grid', placeItems: 'center', minHeight: 160 }}>
          <Spinner label="Loading analytics" />
        </div>
      ) : query.isError ? (
        <Alert tone="danger" title="Analytics could not load">
          {query.error.message}
        </Alert>
      ) : (
        <>
          <StatBar className="analytics-ledger">
            <Stat label="Article views" value={data?.article_views.toLocaleString() ?? '0'} />
            <Stat label="Published" value={data?.published_articles.toLocaleString() ?? '0'} />
            <Stat label="Failed searches" value={data?.failed_searches.toLocaleString() ?? '0'} />
            <Stat label="Helpful rate" value={`${helpfulRate}%`} />
          </StatBar>

          <div className="analytics-grid">
            <section className="health-panel">
              <span className="eyebrow">Content inventory</span>
              <h2>
                {data?.total_articles} articles across the workspace
              </h2>
              <div style={{ display: 'grid', gap: 12, marginTop: 16 }}>
                <ProgressBar
                  label="Published"
                  value={total ? data!.published_articles : 0}
                  max={total || 1}
                  tone="success"
                  showValue
                />
                <ProgressBar
                  label="Draft"
                  value={total ? data!.draft_articles : 0}
                  max={total || 1}
                  tone="warning"
                  showValue
                />
                <ProgressBar
                  label="Pending review"
                  value={total ? data!.pending_reviews : 0}
                  max={total || 1}
                  tone="accent"
                  showValue
                />
              </div>
            </section>
            <aside className="feedback-panel">
              <span className="eyebrow">Reader signal</span>
              <div>
                <ThumbsUp />
                <strong>{data?.helpful_votes}</strong>
                <span>Helpful</span>
              </div>
              <div>
                <ThumbsDown />
                <strong>{data?.unhelpful_votes}</strong>
                <span>Needs work</span>
              </div>
            </aside>
          </div>
        </>
      )}
    </div>
  )
}
