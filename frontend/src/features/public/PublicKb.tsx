import { useState } from 'react'
import { useMutation, useQuery } from '@tanstack/react-query'
import { ArrowLeft, BookOpen, Search, ThumbsDown, ThumbsUp } from 'lucide-react'
import { Link, useParams, useSearchParams } from 'react-router'
import { api } from '../../lib/api'

type PublicKb = { id: string; name: string; slug: string; description?: string; language: string }
type PublicArticle = { id: string; title: string; slug: string; short_description?: string; content: string; category_name: string; published_at?: string; updated_at: string }

export function PublicHome() {
  const { kbSlug = '' } = useParams(); const [queryText, setQueryText] = useState(''); const [params, setParams] = useSearchParams(); const q = params.get('q') ?? ''
  const kb = useQuery({ queryKey: ['public-kb', kbSlug], queryFn: ({ signal }) => api<PublicKb>(`/api/v1/public/${kbSlug}`, { signal }) })
  const results = useQuery({ queryKey: ['public-search', kbSlug, q], enabled: Boolean(q), queryFn: ({ signal }) => api<PublicArticle[]>(`/api/v1/public/${kbSlug}/search?q=${encodeURIComponent(q)}`, { signal }) })
  return <div className="public-site"><header className="public-nav"><Link to={`/help/${kbSlug}`} className="public-brand"><span><BookOpen /></span>{kb.data?.name ?? 'Help center'}</Link><span>English</span></header><main>{!q && <section className="public-hero"><span className="eyebrow">Knowledge that gets you unstuck</span><h1>{kb.data?.description || 'What can we help you find?'}</h1><form onSubmit={(event) => { event.preventDefault(); setParams({ q: queryText }) }}><Search /><input value={queryText} onChange={(event) => setQueryText(event.target.value)} placeholder="Search for an answer" aria-label="Search for an answer" /><button>Search</button></form></section>}{q && <section className="public-results"><button className="back-link" onClick={() => setParams({})}><ArrowLeft />Back to help center</button><span className="eyebrow">Search results</span><h1>Results for “{q}”</h1>{results.data?.length ? <div>{results.data.map((article) => <Link key={article.id} to={`/help/${kbSlug}/article/${article.slug}`}><span>{article.category_name}</span><h2>{article.title}</h2><p>{article.short_description}</p></Link>)}</div> : <section className="public-no-results"><Search /><h2>No published answers found</h2><p>Try a shorter phrase or a different term.</p></section>}</section>}</main></div>
}

export function PublicArticlePage() {
  const { kbSlug = '', articleSlug = '' } = useParams(); const [voted, setVoted] = useState(false)
  const query = useQuery({ queryKey: ['public-article', kbSlug, articleSlug], queryFn: ({ signal }) => api<PublicArticle>(`/api/v1/public/${kbSlug}/articles/${articleSlug}`, { signal }) })
  const feedback = useMutation({ mutationFn: (helpful: boolean) => api(`/api/v1/public/articles/${query.data?.id}/feedback`, { method: 'POST', body: JSON.stringify({ helpful, visitor_id: getVisitorId() }) }), onSuccess: () => setVoted(true) })
  if (query.isLoading) return <div className="public-site"><div className="public-loading" /></div>
  if (!query.data) return <div className="public-site"><section className="public-no-results"><h1>Article not found</h1></section></div>
  return <div className="public-site"><header className="public-nav"><Link to={`/help/${kbSlug}`} className="public-brand"><span><BookOpen /></span>Help center</Link><Link to={`/help/${kbSlug}`}><Search />Search</Link></header><main className="reader-layout"><article className="reader"><nav><Link to={`/help/${kbSlug}`}>Help center</Link><span>/</span><span>{query.data.category_name}</span></nav><span className="eyebrow">{query.data.category_name}</span><h1>{query.data.title}</h1>{query.data.short_description && <p className="article-deck">{query.data.short_description}</p>}<div className="article-body" dangerouslySetInnerHTML={{ __html: query.data.content }} /><footer className="article-feedback">{voted ? <div><CheckMark />Thanks. Your feedback helps keep this answer useful.</div> : <><strong>Was this article helpful?</strong><span><button onClick={() => feedback.mutate(true)}><ThumbsUp />Yes</button><button onClick={() => feedback.mutate(false)}><ThumbsDown />No</button></span></>}</footer></article><aside className="article-toc"><span className="eyebrow">On this page</span><a href="#top">Overview</a><small>Updated {new Date(query.data.updated_at).toLocaleDateString()}</small></aside></main></div>
}

function getVisitorId() { const key = 'kb_visitor_id'; let id = localStorage.getItem(key); if (!id) { id = crypto.randomUUID(); localStorage.setItem(key, id) } return id }
function CheckMark() { return <span aria-hidden>✓</span> }
