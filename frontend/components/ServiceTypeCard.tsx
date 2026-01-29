'use client'

import { motion } from 'framer-motion'
import Image from 'next/image'

interface ServiceTypeCardProps {
    icon: string | React.ReactNode
    title: string
    description: string
    selected: boolean
    onClick: () => void
    color?: 'cyan' | 'purple' | 'green'
    badge?: string
}

export function ServiceTypeCard({
    icon,
    title,
    description,
    selected,
    onClick,
    color = 'cyan',
    badge
}: ServiceTypeCardProps) {
    const colorClasses = {
        cyan: {
            border: 'border-cyan-400',
            bg: 'bg-cyan-400/10',
            shadow: 'shadow-cyan-400/20',
            check: 'bg-cyan-400'
        },
        purple: {
            border: 'border-purple-400',
            bg: 'bg-purple-400/10',
            shadow: 'shadow-purple-400/20',
            check: 'bg-purple-400'
        },
        green: {
            border: 'border-green-400',
            bg: 'bg-green-400/10',
            shadow: 'shadow-green-400/20',
            check: 'bg-green-400'
        }
    }

    const colors = colorClasses[color]

    return (
        <motion.button
            type="button"
            whileHover={{ scale: 1.02 }}
            whileTap={{ scale: 0.98 }}
            onClick={onClick}
            className={`relative p-4 rounded-xl border-2 transition-all text-left ${selected
                ? `${colors.border} ${colors.bg} shadow-lg ${colors.shadow}`
                : 'border-white/10 bg-white/5 hover:border-white/20'
                }`}>
            {selected && (
                <div className="absolute top-2 right-2">
                    <div className={`w-5 h-5 rounded-full ${colors.check} flex items-center justify-center text-black text-xs font-bold`}>
                        ✓
                    </div>
                </div>
            )}
            {badge && (
                <div className="absolute top-2 left-2">
                    <span className="px-2 py-0.5 bg-amber-400/20 text-amber-400 text-[10px] rounded-md font-bold">
                        {badge}
                    </span>
                </div>
            )}
            <div className={`mb-2 flex items-center justify-center overflow-hidden bg-white rounded-full ${typeof icon === 'string' && icon.startsWith('/')
                ? 'h-24 w-24'
                : 'text-3xl h-12 w-12 bg-transparent'}`}>
                {typeof icon === 'string' && icon.startsWith('/') ? (
                    <div className="relative w-full h-full p-10">
                        <Image src={icon} alt={title} fill className="object-contain" />
                    </div>
                ) : (
                    icon
                )}
            </div>
            <div className="font-bold text-white mb-1">{title}</div>
            <div className="text-xs text-slate-400">{description}</div>
        </motion.button>
    )
}
