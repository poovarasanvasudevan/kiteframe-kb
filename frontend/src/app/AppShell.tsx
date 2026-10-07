import { useCallback, useMemo, useState } from 'react'
import { Link, NavLink, Outlet, useLocation, useNavigate } from 'react-router'
import {
  AppLayout,
  Avatar,
  Button,
  CommandK,
  SearchTrigger,
  Sidebar,
  SidebarFooterButton,
  SidebarItem,
  TopBar,
  useCommandKShortcut,
  type CommandItem,
} from '@kiteframe/ui'
import {
  BarChart3,
  Bell,
  BookOpen,
  ClipboardList,
  FileText,
  Gauge,
  HelpCircle,
  LayoutGrid,
  Menu as MenuIcon,
  Plus,
  PlusCircle,
  Settings,
  Users,
  X,
} from 'lucide-react'

const navigation = [
  { to: '/dashboard', label: 'Overview', icon: Gauge },
  { to: '/knowledge', label: 'Knowledge', icon: BookOpen },
  { to: '/articles', label: 'Articles', icon: FileText },
  { to: '/reviews', label: 'Review queue', icon: ClipboardList },
  { to: '/analytics', label: 'Analytics', icon: BarChart3 },
  { to: '/people', label: 'People & teams', icon: Users },
  { to: '/settings', label: 'Settings', icon: Settings },
]

const titles: Record<string, string> = {
  '/dashboard': 'Overview',
  '/knowledge': 'Knowledge base',
  '/articles': 'Articles',
  '/reviews': 'Review queue',
  '/analytics': 'Analytics',
  '/people': 'People & teams',
  '/settings': 'Settings',
}

function pageTitle(pathname: string) {
  if (pathname.startsWith('/articles/') && pathname.endsWith('/edit')) return 'Edit article'
  if (pathname === '/articles/new') return 'New article'
  if (pathname.includes('/categories')) return 'Categories'
  const match = Object.keys(titles).find((key) => pathname === key || pathname.startsWith(`${key}/`))
  return match ? titles[match] : 'Knowledge base'
}

export function AppShell() {
  const [navOpen, setNavOpen] = useState(false)
  const [commandOpen, setCommandOpen] = useState(false)
  const location = useLocation()
  const navigate = useNavigate()
  const title = pageTitle(location.pathname)

  const toggleCommand = useCallback(() => setCommandOpen((open) => !open), [])
  useCommandKShortcut(toggleCommand)

  const commandItems = useMemo<CommandItem[]>(
    () => [
      ...navigation.map(({ to, label, icon: Glyph }) => ({
        id: to,
        label,
        group: 'Navigate',
        icon: <Glyph size={14} aria-hidden />,
        onSelect: () => navigate(to),
      })),
      {
        id: 'new-article',
        label: 'Create article',
        group: 'Actions',
        icon: <Plus size={14} aria-hidden />,
        onSelect: () => navigate('/articles/new'),
      },
      {
        id: 'new-kb',
        label: 'Create knowledge base',
        group: 'Actions',
        icon: <BookOpen size={14} aria-hidden />,
        onSelect: () => navigate('/knowledge'),
      },
    ],
    [navigate],
  )

  return (
    <>
      <AppLayout
        className="app-shell"
        sidebar={
          <Sidebar
            open={navOpen}
            brandLabel="KiteFrame"
            brand={
              <>
                <span className="kf-sidebar__mark" aria-hidden>
                  k
                </span>
                <button type="button" className="mobile-close" onClick={() => setNavOpen(false)} aria-label="Close navigation">
                  <X />
                </button>
              </>
            }
            footer={<SidebarFooterButton label="Apps" icon={<LayoutGrid aria-hidden />} />}
          >
            {navigation.map(({ to, label, icon: Glyph }) => (
              <SidebarItem
                key={to}
                as={NavLink}
                to={to}
                end={to === '/dashboard'}
                label={label}
                icon={<Glyph aria-hidden />}
                onClick={() => setNavOpen(false)}
              />
            ))}
          </Sidebar>
        }
        topbar={
          <TopBar
            className="app-topbar"
            leading={
              <Button
                variant="ghost"
                iconOnly
                className="menu-button"
                aria-label="Open navigation"
                onClick={() => setNavOpen(true)}
              >
                <MenuIcon />
              </Button>
            }
            title={title}
            actions={
              <>
                <SearchTrigger
                  className="command-search"
                  placeholder="Search articles"
                  shortcut="⌘ K"
                  onClick={() => setCommandOpen(true)}
                />
                <Link to="/articles/new" className="app-topbar__create">
                  <Button size="sm" variant="primary" leftIcon={<PlusCircle size={13} aria-hidden />}>
                    Create
                  </Button>
                </Link>
                <Button variant="ghost" iconOnly aria-label="Help">
                  <HelpCircle aria-hidden />
                </Button>
                <Button variant="ghost" iconOnly aria-label="Notifications" className="icon-button">
                  <Bell aria-hidden />
                  <span className="notification-dot" />
                </Button>
              </>
            }
            trailing={<Avatar name="Ramesh Kumar" fallback="RK" aria-label="Open user menu" />}
          />
        }
      >
        <Outlet />
      </AppLayout>

      {navOpen ? (
        <button type="button" aria-label="Close navigation" className="scrim" onClick={() => setNavOpen(false)} />
      ) : null}

      <CommandK
        open={commandOpen}
        onOpenChange={setCommandOpen}
        items={commandItems}
        placeholder="Search articles, categories, and settings"
        emptyText="Start typing to find a published article or workspace action."
      />
    </>
  )
}
