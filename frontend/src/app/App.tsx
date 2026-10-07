import { lazy, Suspense } from 'react'
import { Navigate, Outlet, Route, Routes } from 'react-router'
import { EmptyState, Spinner } from '@kiteframe/ui'
import { hasSession } from '../lib/api'
import { AppShell } from './AppShell'

const Dashboard = lazy(() => import('../features/dashboard/Dashboard'))
const KnowledgeBases = lazy(() => import('../features/knowledge-bases/KnowledgeBases'))
const CategoryWorkspace = lazy(() => import('../features/categories/CategoryWorkspace'))
const Articles = lazy(() => import('../features/articles/Articles'))
const ArticleEditor = lazy(() => import('../features/articles/ArticleEditor'))
const Reviews = lazy(() => import('../features/reviews/Reviews'))
const Analytics = lazy(() => import('../features/analytics/Analytics'))
const PublicHome = lazy(() => import('../features/public/PublicKb').then((module) => ({ default: module.PublicHome })))
const PublicArticlePage = lazy(() => import('../features/public/PublicKb').then((module) => ({ default: module.PublicArticlePage })))
const Login = lazy(() => import('../features/auth/Login'))
const People = lazy(() => import('../features/people/People'))

function Placeholder({ title }: { title: string }) {
  return (
    <EmptyState
      title={title}
      description="Coming in the next product slice. The foundation is ready for this workflow without introducing mock APIs."
    />
  )
}

export function App() {
  return <Routes>
    <Route path="help/:kbSlug" element={<Suspense fallback={<PageSkeleton />}><PublicHome /></Suspense>} />
    <Route path="help/:kbSlug/article/:articleSlug" element={<Suspense fallback={<PageSkeleton />}><PublicArticlePage /></Suspense>} />
    <Route path="login" element={<Suspense fallback={<PageSkeleton />}><Login /></Suspense>} />
    <Route element={<RequireSession />}>
    <Route element={<AppShell />}>
      <Route index element={<Navigate replace to="/dashboard" />} />
      <Route path="dashboard" element={<Suspense fallback={<PageSkeleton />}><Dashboard /></Suspense>} />
      <Route path="knowledge" element={<Suspense fallback={<PageSkeleton />}><KnowledgeBases /></Suspense>} />
      <Route path="knowledge/:knowledgeBaseId/categories" element={<Suspense fallback={<PageSkeleton />}><CategoryWorkspace /></Suspense>} />
      <Route path="articles" element={<Suspense fallback={<PageSkeleton />}><Articles /></Suspense>} />
      <Route path="articles/new" element={<Suspense fallback={<PageSkeleton />}><ArticleEditor /></Suspense>} />
      <Route path="articles/:id/edit" element={<Suspense fallback={<PageSkeleton />}><ArticleEditor /></Suspense>} />
      <Route path="reviews" element={<Suspense fallback={<PageSkeleton />}><Reviews /></Suspense>} />
      <Route path="analytics" element={<Suspense fallback={<PageSkeleton />}><Analytics /></Suspense>} />
      <Route path="people" element={<Suspense fallback={<PageSkeleton />}><People /></Suspense>} />
      <Route path="settings" element={<Placeholder title="Workspace settings" />} />
    </Route>
    </Route>
  </Routes>
}

function RequireSession() { return hasSession() ? <Outlet /> : <Navigate to="/login" replace /> }

function PageSkeleton() {
  return (
    <div
      aria-label="Loading page"
      className="skeleton-page"
      style={{ display: 'grid', placeItems: 'center', minHeight: 240 }}
    >
      <Spinner size="lg" label="Loading page" />
    </div>
  )
}
