'use client'

import { motion, HTMLMotionProps } from 'framer-motion'
import { ReactNode } from 'react'

interface CardProps extends HTMLMotionProps<"div"> {
    children: ReactNode
    className?: string
    hover?: boolean
    glass?: boolean
    glow?: 'cyan' | 'purple' | 'green' | 'none'
}

export function Card({
    children,
    className = '',
    hover = true,
    glass = true,
    glow = 'none',
    ...props
}: CardProps) {
    const glowStyles = {
        cyan: 'shadow-glow-cyan',
        purple: 'shadow-glow-purple',
        green: 'shadow-lg shadow-emerald-500/20',
        none: 'shadow-lg shadow-black/30',
    }

    return (
        <motion.div
            {...props}
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            className={`
        relative rounded-2xl p-6 border transition-all duration-300
        ${glass ? 'glass' : 'bg-[#0f0f0f]'}
        ${hover ? 'hover:border-white/20 hover:shadow-xl' : ''}
        ${glowStyles[glow]}
        ${className}
      `}
        >
            {children}
        </motion.div>
    )
}
