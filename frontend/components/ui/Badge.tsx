'use client'

import { ReactNode } from 'react'

interface BadgeProps {
    children: ReactNode
    /**
     * Visual flavour. `neutral` is for "no opinion yet" states (e.g.
     * health not checked, value not set) — distinct from `danger`
     * which actively claims something is wrong.
     */
    variant?: 'primary' | 'success' | 'warning' | 'danger' | 'purple' | 'cyan' | 'neutral'
    size?: 'sm' | 'md'
    glow?: boolean
}

export function Badge({
    children,
    variant = 'primary',
    size = 'md',
    glow = false
}: BadgeProps) {
    const variants = {
        primary: 'bg-gradient-primary text-white',
        success: 'bg-gradient-green text-white',
        warning: 'bg-gradient-amber text-white',
        danger: 'bg-gradient-to-r from-red-500 to-rose-600 text-white',
        purple: 'bg-gradient-purple text-white',
        cyan: 'bg-cyan-500/20 text-cyan-400 border border-cyan-500/50',
        neutral: 'bg-slate-500/15 text-slate-300 border border-slate-500/30',
    }

    const sizes = {
        sm: 'px-2 py-0.5 text-xs',
        md: 'px-3 py-1 text-sm',
    }

    const glowEffect = glow ? 'shadow-lg shadow-cyan-500/50' : ''

    return (
        <span className={`
      inline-flex items-center rounded-full font-semibold
      ${variants[variant]} ${sizes[size]} ${glowEffect}
    `}>
            {children}
        </span>
    )
}
