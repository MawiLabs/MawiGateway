'use client'

import Link from 'next/link'
import { usePathname, useRouter } from 'next/navigation'
import { motion, AnimatePresence } from 'framer-motion'
import { useAuth } from '@/contexts/AuthContext'
import { useState } from 'react'
import {
    LayoutGrid,
    Plug,
    Link2,
    Settings,
    Play,
    BarChart3,
    FileText,
    Lock,
    Shield,
    User,
    LogOut,
    MoreVertical,
    Sprout,
    type LucideIcon,
} from 'lucide-react'

type NavLink = { name: string; href: string; icon: LucideIcon }
type NavSpacer = { type: 'spacer' }
type NavHeader = { type: 'header'; name: string }
type NavDivider = { type: 'divider' }
type NavItem = NavLink | NavSpacer | NavHeader | NavDivider

const navigation: NavItem[] = [
    { name: 'Overview', href: '/', icon: LayoutGrid },
    { name: 'Providers', href: '/providers', icon: Plug },
    { name: 'MCP Servers', href: '/providers/mcp', icon: Link2 },
    { name: 'Services', href: '/services', icon: Settings },
    { name: 'Playground', href: '/playground', icon: Play },
    { name: 'Analytics', href: '/analytics', icon: BarChart3 },
    { name: 'Logs', href: '/logs', icon: FileText },
    { type: 'spacer' },
    { type: 'header', name: 'Governance' },
    { name: 'Access Control', href: '/governance/access-control', icon: Lock },
    { name: 'Guardrails', href: '/governance/guardrails', icon: Shield },
]

export default function Sidebar() {
    const pathname = usePathname()
    const router = useRouter()
    const { user, logout } = useAuth()
    const [showUserMenu, setShowUserMenu] = useState(false)

    // Don't show sidebar on auth pages
    if (pathname?.startsWith('/auth')) {
        return null
    }

    const handleLogout = async () => {
        await logout()
        router.push('/auth/login')
    }

    return (
        <aside
            className="fixed left-0 top-0 h-screen w-64 bg-black border-r border-white/10 backdrop-blur-xl flex flex-col z-[100]">

            {/* Logo Section */}
            <div className="p-6 border-b border-white/10">
                <Link href="/">
                    <div className="cursor-pointer hover:scale-105 transition-transform">
                        <h1 className="text-2xl font-bold bg-gradient-to-r from-cyan-400 to-cyan-600 bg-clip-text text-transparent drop-shadow-[0_0_20px_rgba(34,211,238,0.3)]">
                            MaWi
                        </h1>
                        <p className="text-xs text-slate-400 mt-1">Agentic Gateway</p>
                    </div>
                </Link>
            </div>

            {/* Navigation Items */}
            <nav className="flex-1 px-3 py-4 space-y-1 overflow-y-auto custom-scrollbar flex flex-col">
                {navigation.map((item, index) => {
                    if ('type' in item && item.type === 'spacer') {
                        return <div key={index} className="flex-1" />
                    }
                    if ('type' in item && item.type === 'header') {
                        return (
                            <div key={index} className="px-3 py-2 mt-4 mb-2 text-xs font-semibold text-slate-500 uppercase tracking-wider">
                                {item.name}
                            </div>
                        )
                    }
                    if ('type' in item && item.type === 'divider') {
                        return <div key={index} className="my-2 border-t border-white/5" />
                    }

                    const link = item as NavLink
                    const Icon = link.icon
                    const isActive = pathname === link.href
                    return (
                        <Link key={link.name} href={link.href}>
                            <div
                                className={`
                  relative flex items-center gap-3 px-3 py-2.5 rounded-xl cursor-pointer
                  transition-all duration-200 group
                  ${isActive
                                        ? 'text-white bg-gradient-to-r from-cyan-400/20 to-cyan-600/20'
                                        : 'text-slate-400 hover:text-white hover:bg-white/5'
                                    }
                `}>
                                {isActive && (
                                    <div
                                        className="absolute left-0 top-0 bottom-0 w-1 bg-gradient-to-b from-cyan-400 to-cyan-600 rounded-r-full"
                                    />
                                )}
                                <Icon
                                    className={`w-4 h-4 shrink-0 transition-transform group-hover:scale-110 ${isActive ? 'scale-110' : ''}`}
                                    strokeWidth={1.75}
                                />
                                <span className="font-medium">{link.name}</span>
                                {isActive && (
                                    <div
                                        className="ml-auto w-1.5 h-1.5 rounded-full bg-cyan-400 animate-pulse"
                                    />
                                )}
                            </div>
                        </Link>
                    )
                })}
            </nav>

            {/* User Section */}
            {user ? (
                <div className="relative border-t border-white/10 p-3">
                    <button
                        onClick={() => setShowUserMenu(!showUserMenu)}
                        className="w-full px-3 py-3 rounded-xl hover:bg-white/5 transition-all duration-200 group">
                        <div className="flex items-center gap-3">
                            <div className="w-10 h-10 rounded-full bg-gradient-to-br from-cyan-400/20 to-cyan-600/20 border border-cyan-400/50 flex items-center justify-center text-cyan-300">
                                <User className="w-4 h-4" strokeWidth={2} />
                            </div>
                            <div className="flex-1 text-left">
                                <div className="text-sm font-medium text-white">{user.name || 'User'}</div>
                                <div className="text-xs text-slate-400 truncate">{user.email}</div>
                                <div className="text-xs text-emerald-400 mt-0.5 inline-flex items-center gap-1">
                                    <Sprout className="w-3 h-3" strokeWidth={2} />
                                    Community
                                </div>
                            </div>
                            <MoreVertical className="w-4 h-4 text-slate-400 group-hover:text-white" strokeWidth={2} />
                        </div>
                    </button>

                    {/* User Menu Dropdown */}
                    <AnimatePresence>
                        {showUserMenu && (
                            <motion.div
                                initial={{ opacity: 0, y: 10 }}
                                animate={{ opacity: 1, y: 0 }}
                                exit={{ opacity: 0, y: 10 }}
                                className="absolute bottom-full left-3 right-3 mb-2 bg-[#0f0f0f] border border-white/10 rounded-xl overflow-hidden shadow-2xl">
                                <Link href="/profile">
                                    <div className="px-4 py-3 hover:bg-white/5 transition-colors cursor-pointer flex items-center gap-2 text-sm text-white">
                                        <User className="w-4 h-4" strokeWidth={2} /> Profile
                                    </div>
                                </Link>
                                <div
                                    onClick={handleLogout}
                                    className="px-4 py-3 hover:bg-white/5 transition-colors cursor-pointer flex items-center gap-2 text-sm text-red-400">
                                    <LogOut className="w-4 h-4" strokeWidth={2} /> Sign Out
                                </div>
                            </motion.div>
                        )}
                    </AnimatePresence>
                </div>
            ) : (
                <div className="border-t border-white/10 p-3">
                    <Link href="/auth/login">
                        <button className="w-full px-4 py-2 rounded-xl bg-gradient-to-r from-cyan-400 to-cyan-600 text-white font-medium hover:shadow-lg hover:shadow-cyan-400/40 transition-all">
                            Sign In
                        </button>
                    </Link>
                </div>
            )}
        </aside>
    )
}
