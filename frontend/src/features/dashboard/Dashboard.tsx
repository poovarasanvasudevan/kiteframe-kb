import { ArrowRight, FileClock, FilePenLine, Globe2, SearchX, Sparkles } from 'lucide-react'
import { Link } from 'react-router'
import { Badge, Button, Card, CardBody, PageHeader, Stat, StatBar } from '@kiteframe/ui'

const metrics = [
  { label: 'Published', value: '—', hint: 'Connect the API to see totals', icon: Globe2 },
  { label: 'Drafts', value: '—', hint: 'No article data yet', icon: FilePenLine },
  { label: 'Awaiting review', value: '—', hint: 'Review workflow ships in phase 4', icon: FileClock },
  { label: 'Search gaps', value: '—', hint: 'Public search ships in phase 5', icon: SearchX },
]

export default function Dashboard() {
  return (
    <div className="dashboard-page">
      <PageHeader
        title="Good afternoon."
        description="Build the structure your writers need, then turn on the article workflow."
        actions={
          <Link to="/knowledge">
            <Button variant="primary" leftIcon={<Sparkles aria-hidden />}>
              Create knowledge base
            </Button>
          </Link>
        }
      />

      <StatBar aria-label="Workspace metrics" className="metric-strip">
        {metrics.map(({ label, value }) => (
          <Stat key={label} label={label} value={value} />
        ))}
      </StatBar>

      <div className="dashboard-grid">
        <Card className="work-queue">
          <CardBody>
            <div className="section-heading">
              <div>
                <span className="eyebrow">Work queue</span>
                <h2>Start with the information architecture</h2>
              </div>
              <Badge tone="accent">Phase 2 ready</Badge>
            </div>
            <ol className="task-list">
              <li>
                <span className="task-index">01</span>
                <div>
                  <strong>Create a knowledge base</strong>
                  <p>Choose its audience and a stable URL slug.</p>
                </div>
                <Link to="/knowledge" aria-label="Create a knowledge base">
                  <ArrowRight />
                </Link>
              </li>
              <li>
                <span className="task-index">02</span>
                <div>
                  <strong>Shape the category tree</strong>
                  <p>Categories inherit the tenant and knowledge-base boundary.</p>
                </div>
                <button type="button" disabled aria-label="Category editor is not yet available">
                  <ArrowRight />
                </button>
              </li>
              <li>
                <span className="task-index">03</span>
                <div>
                  <strong>Invite the editorial team</strong>
                  <p>Assign roles before content enters review.</p>
                </div>
                <button type="button" disabled aria-label="Team invitations are not yet available">
                  <ArrowRight />
                </button>
              </li>
            </ol>
          </CardBody>
        </Card>

        <Card className="system-panel">
          <CardBody>
            <span className="eyebrow">System status</span>
            <h2>Foundation online</h2>
            <p>
              Authentication, tenant-scoped queries, migrations, API documentation, and the application
              shell are in place.
            </p>
            <dl>
              <div>
                <dt>API contract</dt>
                <dd>v1</dd>
              </div>
              <div>
                <dt>Database</dt>
                <dd>MySQL 8+</dd>
              </div>
              <div>
                <dt>Tenant guard</dt>
                <dd className="status-ok">Enforced</dd>
              </div>
            </dl>
            <a href="http://localhost:8080/docs" target="_blank" rel="noreferrer">
              Open API reference <ArrowRight />
            </a>
          </CardBody>
        </Card>
      </div>
    </div>
  )
}
