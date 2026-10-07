import { useEffect, useRef, useState, type ReactNode } from 'react'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { EditorContent, useEditor, type Editor } from '@tiptap/react'
import StarterKit from '@tiptap/starter-kit'
import Underline from '@tiptap/extension-underline'
import LinkExtension from '@tiptap/extension-link'
import Placeholder from '@tiptap/extension-placeholder'
import TaskList from '@tiptap/extension-task-list'
import TaskItem from '@tiptap/extension-task-item'
import TextAlign from '@tiptap/extension-text-align'
import { TextStyle } from '@tiptap/extension-text-style'
import { Color } from '@tiptap/extension-color'
import Highlight from '@tiptap/extension-highlight'
import Image from '@tiptap/extension-image'
import Youtube from '@tiptap/extension-youtube'
import { Table } from '@tiptap/extension-table'
import { TableRow } from '@tiptap/extension-table-row'
import { TableCell } from '@tiptap/extension-table-cell'
import { TableHeader } from '@tiptap/extension-table-header'
import Subscript from '@tiptap/extension-subscript'
import Superscript from '@tiptap/extension-superscript'
import {
  AlignCenter,
  AlignJustify,
  AlignLeft,
  AlignRight,
  ArrowLeft,
  Bold,
  Check,
  Code2,
  Eraser,
  Heading1,
  Heading2,
  Heading3,
  Highlighter,
  ImageIcon,
  IndentDecrease,
  IndentIncrease,
  Italic,
  Link as LinkIcon,
  List,
  ListChecks,
  ListOrdered,
  Minus,
  Pilcrow,
  Redo2,
  RemoveFormatting,
  Send,
  Strikethrough,
  Subscript as SubscriptIcon,
  Superscript as SuperscriptIcon,
  Table as TableIcon,
  Underline as UnderlineIcon,
  Undo2,
  Video,
} from 'lucide-react'
import { Link, useNavigate, useParams } from 'react-router'
import { Alert, Button, Popover, Select, TextArea, TextField } from '@kiteframe/ui'
import { api, ApiError } from '../../lib/api'
import { articlesApi, type Category, type KnowledgeBase } from './api'

type SaveState = 'clean' | 'dirty' | 'saving' | 'saved' | 'error'

const TEXT_COLORS = ['#12344d', '#2c5cc5', '#00a886', '#e86f25', '#d72d30', '#7b8e9f', '#ffffff']
const HIGHLIGHT_COLORS = ['#fff3bf', '#d3f9d8', '#d0ebff', '#ffe3e3', '#f3d9fa', '#fff9db']

export default function ArticleEditor() {
  const { id } = useParams()
  const isNew = !id || id === 'new'
  const navigate = useNavigate()
  const queryClient = useQueryClient()
  const articleQuery = useQuery({
    queryKey: ['article', id],
    enabled: !isNew,
    queryFn: ({ signal }) => articlesApi.get(id!, signal),
  })
  const kbs = useQuery({
    queryKey: ['knowledge-bases'],
    queryFn: ({ signal }) => api<KnowledgeBase[]>('/api/v1/knowledge-bases', { signal }),
  })
  const [kbId, setKbId] = useState('')
  const [categoryId, setCategoryId] = useState('')
  const [title, setTitle] = useState('')
  const [slug, setSlug] = useState('')
  const [description, setDescription] = useState('')
  const [visibility, setVisibility] = useState('INTERNAL')
  const [lockVersion, setLockVersion] = useState(0)
  const [saveState, setSaveState] = useState<SaveState>('clean')
  const hydrated = useRef(false)
  const categories = useQuery({
    queryKey: ['categories', kbId],
    enabled: Boolean(kbId),
    queryFn: ({ signal }) => api<Category[]>(`/api/v1/categories/by-knowledge-base/${kbId}`, { signal }),
  })

  const editor = useEditor({
    extensions: [
      StarterKit.configure({ heading: { levels: [1, 2, 3] } }),
      Underline,
      TextStyle,
      Color,
      Highlight.configure({ multicolor: true }),
      Subscript,
      Superscript,
      LinkExtension.configure({ openOnClick: false }),
      Placeholder.configure({ placeholder: 'Write the answer readers came for…' }),
      TaskList,
      TaskItem.configure({ nested: true }),
      TextAlign.configure({ types: ['heading', 'paragraph'] }),
      Image.configure({ allowBase64: true, inline: false }),
      Youtube.configure({ controls: true, modestBranding: true, width: 640, height: 360 }),
      Table.configure({ resizable: true }),
      TableRow,
      TableHeader,
      TableCell,
    ],
    content: '',
    onUpdate: () => setSaveState('dirty'),
  })

  useEffect(() => {
    const article = articleQuery.data
    if (!article || !editor || hydrated.current) return
    hydrated.current = true
    setKbId(article.knowledge_base_id)
    setCategoryId(article.category_id)
    setTitle(article.title)
    setSlug(article.slug)
    setDescription(article.short_description ?? '')
    setVisibility(article.visibility)
    setLockVersion(article.lock_version)
    editor.commands.setContent(article.draft_content)
    setSaveState('clean')
  }, [articleQuery.data, editor])

  const dirty = () => {
    if (hydrated.current || isNew) setSaveState('dirty')
  }
  const payload = () => ({
    title,
    slug,
    short_description: description || null,
    content: editor?.getHTML() ?? '',
    category_id: categoryId,
    knowledge_base_id: kbId,
    visibility,
    lock_version: lockVersion,
    change_summary: 'Autosaved draft',
  })
  const save = useMutation({
    mutationFn: () => (isNew ? articlesApi.create(payload()) : articlesApi.update(id!, payload())),
    onMutate: () => setSaveState('saving'),
    onSuccess: (article) => {
      setSaveState('saved')
      setLockVersion(article.lock_version)
      queryClient.setQueryData(['article', article.id], article)
      if (isNew) navigate(`/articles/${article.id}/edit`, { replace: true })
    },
    onError: () => setSaveState('error'),
  })
  useEffect(() => {
    if (isNew || saveState !== 'dirty' || !title || !categoryId) return
    const timer = window.setTimeout(() => save.mutate(), 1200)
    return () => window.clearTimeout(timer)
  }, [saveState, title, description, categoryId, visibility, editor?.getHTML()])
  const submit = useMutation({
    mutationFn: async () => {
      if (saveState === 'dirty') await save.mutateAsync()
      return articlesApi.action(id!, 'submit-review')
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['articles'] })
      navigate('/articles')
    },
  })
  const canSave = title.trim().length >= 2 && kbId && categoryId
  const stateLabel = {
    clean: 'No unsaved changes',
    dirty: 'Unsaved changes',
    saving: 'Saving…',
    saved: 'Saved',
    error: 'Save failed',
  }[saveState]

  return (
    <div className="editor-page">
      <header className="editor-header">
        <Link className="editor-back" to="/articles" aria-label="Back to articles">
          <ArrowLeft />
        </Link>
        <div className="editor-title-block">
          <input
            value={title}
            onChange={(event) => {
              setTitle(event.target.value)
              if (isNew && !slug) setSlug(event.target.value.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, ''))
              dirty()
            }}
            placeholder="Untitled article"
            aria-label="Article title"
          />
          <span className={`save-state is-${saveState}`}>
            <Check />
            {stateLabel}
          </span>
        </div>
        <div className="editor-actions">
          <Button variant="secondary" disabled={!canSave || save.isPending} onClick={() => save.mutate()}>
            Save draft
          </Button>
          {!isNew && articleQuery.data?.status === 'DRAFT' ? (
            <Button
              variant="primary"
              leftIcon={<Send />}
              disabled={submit.isPending}
              onClick={() => submit.mutate()}
            >
              Submit review
            </Button>
          ) : null}
        </div>
      </header>
      <div className="editor-workbench">
        <main className="writing-pane">
          <EditorToolbar editor={editor} />
          <EditorContent editor={editor} className="article-canvas" />
        </main>
        <aside className="article-settings">
          <span className="eyebrow">Article settings</span>
          <section>
            <h2>Placement</h2>
            <Select
              label="Knowledge base"
              placeholder="Choose a knowledge base"
              value={kbId}
              disabled={!isNew}
              options={(kbs.data ?? []).map((kb) => ({ value: kb.id, label: kb.name }))}
              onValueChange={(next) => {
                setKbId(next)
                setCategoryId('')
                dirty()
              }}
            />
            <Select
              label="Category"
              placeholder="Choose a category"
              value={categoryId}
              options={(categories.data ?? []).map((category) => ({ value: category.id, label: category.name }))}
              onValueChange={(next) => {
                setCategoryId(next)
                dirty()
              }}
            />
          </section>
          <section>
            <h2>Presentation</h2>
            <TextField
              label="URL slug"
              value={slug}
              onChange={(event) => {
                setSlug(event.target.value)
                dirty()
              }}
              disabled={!isNew}
            />
            <TextArea
              label="Short description"
              rows={4}
              value={description}
              onChange={(event) => {
                setDescription(event.target.value)
                dirty()
              }}
            />
            <Select
              label="Visibility"
              value={visibility}
              options={[
                { value: 'PUBLIC', label: 'Public' },
                { value: 'INTERNAL', label: 'Internal' },
                { value: 'PRIVATE', label: 'Private' },
              ]}
              onValueChange={(next) => {
                setVisibility(next)
                dirty()
              }}
            />
          </section>
          {save.error || submit.error ? (
            <Alert tone="danger">
              {(save.error ?? submit.error) instanceof ApiError
                ? (save.error ?? submit.error)?.message
                : 'The article could not be saved.'}
            </Alert>
          ) : null}
        </aside>
      </div>
    </div>
  )
}

function EditorToolbar({ editor }: { editor: Editor | null }) {
  if (!editor) return <div className="editor-toolbar" aria-label="Formatting toolbar" />

  const setLink = () => {
    const previous = editor.getAttributes('link').href as string | undefined
    const url = window.prompt('Link URL', previous ?? 'https://')
    if (url === null) return
    if (url === '') {
      editor.chain().focus().extendMarkRange('link').unsetLink().run()
      return
    }
    editor.chain().focus().extendMarkRange('link').setLink({ href: url }).run()
  }

  const insertImage = () => {
    const url = window.prompt('Image URL', 'https://')
    if (!url) return
    editor.chain().focus().setImage({ src: url, alt: 'Article image' }).run()
  }

  const insertVideo = () => {
    const url = window.prompt('YouTube or Vimeo URL', 'https://www.youtube.com/watch?v=')
    if (!url) return
    editor.commands.setYoutubeVideo({ src: url, width: 640, height: 360 })
  }

  const insertTable = () => {
    editor.chain().focus().insertTable({ rows: 3, cols: 3, withHeaderRow: true }).run()
  }

  return (
    <div className="editor-toolbar" aria-label="Formatting toolbar">
      <ToolGroup>
        <Tool active={editor.isActive('paragraph')} onClick={() => editor.chain().focus().setParagraph().run()} label="Paragraph">
          <Pilcrow />
        </Tool>
        <Tool active={editor.isActive('heading', { level: 1 })} onClick={() => editor.chain().focus().toggleHeading({ level: 1 }).run()} label="Heading 1">
          <Heading1 />
        </Tool>
        <Tool active={editor.isActive('heading', { level: 2 })} onClick={() => editor.chain().focus().toggleHeading({ level: 2 }).run()} label="Heading 2">
          <Heading2 />
        </Tool>
        <Tool active={editor.isActive('heading', { level: 3 })} onClick={() => editor.chain().focus().toggleHeading({ level: 3 }).run()} label="Heading 3">
          <Heading3 />
        </Tool>
      </ToolGroup>

      <ToolGroup>
        <ColorTool
          label="Text color"
          colors={TEXT_COLORS}
          value={editor.getAttributes('textStyle').color ?? '#12344d'}
          onPick={(color) => editor.chain().focus().setColor(color).run()}
          onClear={() => editor.chain().focus().unsetColor().run()}
        />
        <ColorTool
          label="Highlight"
          colors={HIGHLIGHT_COLORS}
          value={editor.getAttributes('highlight').color ?? '#fff3bf'}
          icon={<Highlighter />}
          onPick={(color) => editor.chain().focus().toggleHighlight({ color }).run()}
          onClear={() => editor.chain().focus().unsetHighlight().run()}
        />
        <Tool onClick={() => editor.chain().focus().unsetAllMarks().clearNodes().run()} label="Clear formatting">
          <RemoveFormatting />
        </Tool>
      </ToolGroup>

      <ToolGroup>
        <Tool active={editor.isActive('bold')} onClick={() => editor.chain().focus().toggleBold().run()} label="Bold">
          <Bold />
        </Tool>
        <Tool active={editor.isActive('italic')} onClick={() => editor.chain().focus().toggleItalic().run()} label="Italic">
          <Italic />
        </Tool>
        <Tool active={editor.isActive('underline')} onClick={() => editor.chain().focus().toggleUnderline().run()} label="Underline">
          <UnderlineIcon />
        </Tool>
        <Tool active={editor.isActive('strike')} onClick={() => editor.chain().focus().toggleStrike().run()} label="Strikethrough">
          <Strikethrough />
        </Tool>
        <Tool active={editor.isActive('subscript')} onClick={() => editor.chain().focus().toggleSubscript().run()} label="Subscript">
          <SubscriptIcon />
        </Tool>
        <Tool active={editor.isActive('superscript')} onClick={() => editor.chain().focus().toggleSuperscript().run()} label="Superscript">
          <SuperscriptIcon />
        </Tool>
        <Tool active={editor.isActive('code')} onClick={() => editor.chain().focus().toggleCode().run()} label="Inline code">
          <Code2 />
        </Tool>
      </ToolGroup>

      <ToolGroup>
        <Tool active={editor.isActive({ textAlign: 'left' })} onClick={() => editor.chain().focus().setTextAlign('left').run()} label="Align left">
          <AlignLeft />
        </Tool>
        <Tool active={editor.isActive({ textAlign: 'center' })} onClick={() => editor.chain().focus().setTextAlign('center').run()} label="Align center">
          <AlignCenter />
        </Tool>
        <Tool active={editor.isActive({ textAlign: 'right' })} onClick={() => editor.chain().focus().setTextAlign('right').run()} label="Align right">
          <AlignRight />
        </Tool>
        <Tool active={editor.isActive({ textAlign: 'justify' })} onClick={() => editor.chain().focus().setTextAlign('justify').run()} label="Justify">
          <AlignJustify />
        </Tool>
      </ToolGroup>

      <ToolGroup>
        <Tool active={editor.isActive('bulletList')} onClick={() => editor.chain().focus().toggleBulletList().run()} label="Bullet list">
          <List />
        </Tool>
        <Tool active={editor.isActive('orderedList')} onClick={() => editor.chain().focus().toggleOrderedList().run()} label="Numbered list">
          <ListOrdered />
        </Tool>
        <Tool active={editor.isActive('taskList')} onClick={() => editor.chain().focus().toggleTaskList().run()} label="Checklist">
          <ListChecks />
        </Tool>
        <Tool onClick={() => editor.chain().focus().liftListItem('listItem').run()} label="Decrease indent" disabled={!editor.can().liftListItem('listItem')}>
          <IndentDecrease />
        </Tool>
        <Tool onClick={() => editor.chain().focus().sinkListItem('listItem').run()} label="Increase indent" disabled={!editor.can().sinkListItem('listItem')}>
          <IndentIncrease />
        </Tool>
      </ToolGroup>

      <ToolGroup>
        <Tool active={editor.isActive('link')} onClick={setLink} label="Link">
          <LinkIcon />
        </Tool>
        <Tool active={editor.isActive('table')} onClick={insertTable} label="Insert table">
          <TableIcon />
        </Tool>
        <Tool onClick={() => editor.chain().focus().addColumnAfter().run()} label="Add column" disabled={!editor.can().addColumnAfter()}>
          <span className="toolbar-text">+Col</span>
        </Tool>
        <Tool onClick={() => editor.chain().focus().addRowAfter().run()} label="Add row" disabled={!editor.can().addRowAfter()}>
          <span className="toolbar-text">+Row</span>
        </Tool>
        <Tool onClick={() => editor.chain().focus().deleteTable().run()} label="Delete table" disabled={!editor.can().deleteTable()}>
          <Eraser />
        </Tool>
        <Tool onClick={insertImage} label="Insert image">
          <ImageIcon />
        </Tool>
        <Tool onClick={insertVideo} label="Insert video">
          <Video />
        </Tool>
        <Tool onClick={() => editor.chain().focus().setHorizontalRule().run()} label="Horizontal rule">
          <Minus />
        </Tool>
        <Tool active={editor.isActive('codeBlock')} onClick={() => editor.chain().focus().toggleCodeBlock().run()} label="Code block">
          <Code2 />
        </Tool>
        <Tool active={editor.isActive('blockquote')} onClick={() => editor.chain().focus().toggleBlockquote().run()} label="Quote">
          <span className="toolbar-text">“”</span>
        </Tool>
      </ToolGroup>

      <ToolGroup>
        <Tool onClick={() => editor.chain().focus().undo().run()} label="Undo" disabled={!editor.can().undo()}>
          <Undo2 />
        </Tool>
        <Tool onClick={() => editor.chain().focus().redo().run()} label="Redo" disabled={!editor.can().redo()}>
          <Redo2 />
        </Tool>
      </ToolGroup>
    </div>
  )
}

function ToolGroup({ children }: { children: ReactNode }) {
  return (
    <>
      <div className="editor-toolbar-group">{children}</div>
      <span className="editor-toolbar-divider" aria-hidden />
    </>
  )
}

function Tool({
  active,
  onClick,
  label,
  children,
  disabled,
}: {
  active?: boolean
  onClick: () => void
  label: string
  children: ReactNode
  disabled?: boolean
}) {
  return (
    <button type="button" className={active ? 'is-active' : ''} onClick={onClick} aria-label={label} title={label} disabled={disabled}>
      {children}
    </button>
  )
}

function ColorTool({
  label,
  colors,
  value,
  onPick,
  onClear,
  icon,
}: {
  label: string
  colors: string[]
  value: string
  onPick: (color: string) => void
  onClear: () => void
  icon?: ReactNode
}) {
  const [open, setOpen] = useState(false)
  const pick = (color: string) => {
    onPick(color)
    setOpen(false)
  }

  return (
    <Popover
      open={open}
      onOpenChange={setOpen}
      side="bottom"
      className="editor-color-popover"
      contentClassName="editor-color-panel"
      trigger={
        <span className="editor-color-tool" title={label} aria-label={label}>
          <span className="editor-color-swatch" style={{ background: value }} aria-hidden>
            {icon ?? <span>A</span>}
          </span>
          <span className="editor-color-underline" style={{ background: value }} aria-hidden />
        </span>
      }
    >
      <div className="editor-color-panel__header">{label}</div>
      <div className="editor-color-grid" role="listbox" aria-label={label}>
        {colors.map((color) => (
          <button
            key={color}
            type="button"
            role="option"
            aria-selected={value.toLowerCase() === color.toLowerCase()}
            className={value.toLowerCase() === color.toLowerCase() ? 'is-active' : undefined}
            style={{ background: color }}
            title={color}
            onClick={() => pick(color)}
          />
        ))}
      </div>
      <div className="editor-color-panel__row">
        <label className="editor-color-custom">
          <span>Custom</span>
          <input
            type="color"
            value={/^#[0-9a-fA-F]{6}$/.test(value) ? value : '#12344d'}
            onChange={(event) => pick(event.target.value)}
          />
        </label>
        <button
          type="button"
          className="editor-color-clear"
          onClick={() => {
            onClear()
            setOpen(false)
          }}
        >
          <Eraser size={12} aria-hidden />
          Clear
        </button>
      </div>
    </Popover>
  )
}
