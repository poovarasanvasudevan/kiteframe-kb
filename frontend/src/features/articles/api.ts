import { api } from '../../lib/api'

export type ArticleStatus = 'DRAFT' | 'IN_REVIEW' | 'APPROVED' | 'PUBLISHED' | 'ARCHIVED'
export type Article = {
  id: string; knowledge_base_id: string; category_id: string; author_id: string; current_version_id?: string
  title: string; slug: string; short_description?: string; draft_content: string; status: ArticleStatus
  visibility: 'PUBLIC' | 'PRIVATE' | 'INTERNAL'; language: string; featured: boolean; pinned: boolean
  seo_title?: string; seo_description?: string; canonical_url?: string; published_at?: string
  scheduled_publish_at?: string; lock_version: number; created_at: string; updated_at: string
}
export type Category = { id: string; knowledge_base_id: string; parent_id: string | null; name: string; slug: string; sort_order: number }
export type KnowledgeBase = { id: string; name: string; slug: string; visibility: string }

export const articlesApi = {
  list: (status?: string, signal?: AbortSignal) => api<Article[]>(`/api/v1/articles${status ? `?status=${status}` : ''}`, { signal }),
  get: (id: string, signal?: AbortSignal) => api<Article>(`/api/v1/articles/${id}`, { signal }),
  create: (body: Record<string, unknown>) => api<Article>('/api/v1/articles', { method: 'POST', body: JSON.stringify(body) }),
  update: (id: string, body: Record<string, unknown>) => api<Article>(`/api/v1/articles/${id}`, { method: 'PATCH', body: JSON.stringify(body) }),
  action: (id: string, action: string, body?: Record<string, unknown>) => api<Article>(`/api/v1/articles/${id}/${action}`, { method: 'POST', body: JSON.stringify(body ?? {}) }),
}
