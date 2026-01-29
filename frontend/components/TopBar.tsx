'use client'

import { useAuth } from '@/contexts/AuthContext'
import { motion } from 'framer-motion'
import { usePathname } from 'next/navigation'

export default function TopBar() {
    const { user } = useAuth()
    const pathname = usePathname()

    if (!user) return null

    const usage = user.current_usage_usd || 0
    const limit = user.monthly_quota_usd || 10 // safe default
    const percentage = Math.min((usage / limit) * 100, 100)

    // Format path for breadcrumb
    const pathSegments = pathname?.split('/').filter(p => p) || []
    const breadcrumb = pathSegments.length > 0
        ? pathSegments.map(s => s.charAt(0).toUpperCase() + s.slice(1)).join(' / ')
        : 'Dashboard'

    return (
        <div className="sticky top-0 z-40 w-full h-16 border-b border-white/5 bg-[#050505]/80 backdrop-blur-md flex items-center justify-between px-8 transition-all">
            {/* Left: Breadcrumbs */}
            <div className="flex items-center gap-2 text-sm">
                <span className="text-slate-600">/</span>
                <span className="text-slate-400 font-medium capitalize">{breadcrumb}</span>
            </div>

            {/* Right: Quota & User */}
            <div className="flex items-center gap-6">

                {/* Quota Widget - Phase 1 Priority */}
                <div className="group flex items-center gap-3 bg-white/5 hover:bg-white/10 rounded-full pl-4 pr-1.5 py-1.5 border border-white/5 transition-colors cursor-default">
                    <div className="flex flex-col items-end mr-1">
                        <div className="text-[10px] text-slate-500 group-hover:text-slate-400 uppercase tracking-widest font-semibold transition-colors">
                            Monthly Budget
                        </div>
                        <div className="flex items-baseline gap-1.5 text-xs font-mono">
                            <span className={`font-bold ${percentage > 90 ? 'text-red-400' : 'text-white'}`}>
                                ${usage.toFixed(2)}
                            </span>
                            <span className="text-slate-600">/ ${limit.toFixed(0)}</span>
                        </div>
                    </div>

                    {/* Ring Chart */}
                    <div className="w-9 h-9 relative flex items-center justify-center">
                        <svg className="w-full h-full -rotate-90 transform" viewBox="0 0 36 36">
                            {/* Background Circle */}
                            <path className="text-white/5" d="M18 2.0845 a 15.9155 15.9155 0 0 1 0 31.831 a 15.9155 15.9155 0 0 1 0 -31.831" fill="none" stroke="currentColor" strokeWidth="3" />
                            {/* Progress Circle */}
                            <motion.path
                                className={`${percentage > 90 ? 'text-red-500 drop-shadow-[0_0_8px_rgba(239,68,68,0.5)]' : 'text-cyan-400 drop-shadow-[0_0_8px_rgba(34,211,238,0.5)]'}`}
                                strokeDasharray={`${percentage}, 100`}
                                d="M18 2.0845 a 15.9155 15.9155 0 0 1 0 31.831 a 15.9155 15.9155 0 0 1 0 -31.831"
                                fill="none"
                                stroke="currentColor"
                                strokeWidth="3"
                                strokeLinecap="round"
                                initial={{ strokeDasharray: "0, 100" }}
                                animate={{ strokeDasharray: `${percentage}, 100` }}
                                transition={{ duration: 1.5, ease: "easeOut" }}
                            />
                        </svg>
                        <div className="absolute inset-0 flex items-center justify-center text-[8px] font-bold text-slate-500">
                            {percentage.toFixed(0)}%
                        </div>
                    </div>
                </div>

                <div className="h-6 w-px bg-white/10" />

                {/* Simple User Profile */}
                <div className="flex items-center gap-3">
                    <div className="w-8 h-8 rounded-full bg-gradient-to-tr from-cyan-400 to-purple-600 flex items-center justify-center text-xs font-bold text-white shadow-lg shadow-purple-500/20">
                        {user.name?.[0]?.toUpperCase() || user.email[0].toUpperCase()}
                    </div>
                </div>

            </div>
        </div>
    )
}
