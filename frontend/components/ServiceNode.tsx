'use client'

import { memo } from 'react'
import { Handle, Position } from '@xyflow/react'
import { useRouter } from 'next/navigation'
import {
    Plus, Cpu, Shield, Brain, Image as ImageIcon, Music, Film, Sparkles,
    MessageSquare, Scale, DollarSign, Zap, HeartPulse, type LucideIcon,
} from 'lucide-react'

export default memo(function ServiceNode({ data }: any) {
    const modelCount = data.models?.length || 0
    const router = useRouter()

    const handleAddModel = (e: React.MouseEvent) => {
        e.stopPropagation()
        // Navigate to services page - models are managed there
        router.push('/services')
    }

    return (
        <div className="relative group">
            <Handle type="target" position={Position.Left} className="w-3 h-3 !bg-purple-400/50 !border-2 !border-purple-400/30" />

            {/* Glow effect */}
            <div className="absolute -inset-2 bg-gradient-to-r from-purple-400/20 via-cyan-400/10 to-purple-400/20 rounded-3xl opacity-0 group-hover:opacity-100 blur-2xl transition-all duration-500" />

            <div className="relative px-6 py-5 min-w-[200px] rounded-2xl backdrop-blur-2xl bg-gradient-to-br from-purple-500/20 via-[#1a1025] to-[#0f0f0f] border border-purple-400/30 shadow-2xl shadow-purple-500/10">

                {/* Accent bar */}
                <div className={`absolute top-0 left-4 right-4 h-0.5 bg-gradient-to-r from-transparent ${data.isAgentic ? 'via-indigo-500' : 'via-purple-400'} to-transparent`} />

                {/* Type badge - Restored professional look */}
                <div className={`absolute -top-3 left-1/2 -translate-x-1/2 px-3 py-1 text-[10px] font-bold uppercase tracking-widest text-white rounded-full shadow-lg ${data.isAgentic ? 'bg-indigo-500 shadow-indigo-500/30' : 'bg-purple-500 shadow-purple-500/30'}`}>
                    Service
                </div>

                {/* Add Model Button */}
                <button
                    onClick={handleAddModel}
                    className={`absolute top-1/2 -right-3 -translate-y-1/2 w-6 h-6 rounded-full text-white flex items-center justify-center transition-all hover:scale-110 z-50 border-4 border-black box-content
                        ${data.isAgentic
                            ? 'bg-indigo-500 hover:bg-indigo-400 shadow-lg shadow-indigo-500/50'
                            : 'bg-emerald-500 hover:bg-emerald-400 shadow-lg shadow-emerald-500/50'}`}
                    title="Assign Models"
                    aria-label="Assign Models"
                >
                    <Plus className="w-3.5 h-3.5" strokeWidth={3} />
                </button>

                {/* Icon & Label - Clean layout. Icon picked from modality
                    via a Lucide map; agentic services always get Brain. */}
                <div className="flex items-center gap-3 mt-2 mb-3">
                    <div className={`w-10 h-10 rounded-xl flex items-center justify-center shadow-inner
                        ${data.isAgentic
                            ? 'bg-gradient-to-br from-indigo-500/30 to-purple-600/20 text-indigo-300'
                            : 'bg-gradient-to-br from-purple-400/30 to-purple-600/20 text-purple-200'}`}>
                        {(() => {
                            const Icon: LucideIcon = data.isAgentic
                                ? Brain
                                : (data.modality === 'image' ? ImageIcon
                                    : data.modality === 'audio' ? Music
                                        : data.modality === 'video' ? Film
                                            : data.modality === 'multi-modal' ? Sparkles
                                                : MessageSquare)
                            return <Icon className="w-5 h-5" strokeWidth={1.75} />
                        })()}
                    </div>
                    <div>
                        <div className="text-base font-bold text-white">{data.label}</div>
                        <div className={`text-[10px] uppercase tracking-wide opacity-70 ${data.isAgentic ? 'text-indigo-300' : 'text-purple-300'}`}>
                            {data.modality} • {data.isAgentic ? 'Agentic' : (data.type === 'POOL' ? 'Pool' : data.type)}
                        </div>
                    </div>
                </div>

                {/* Stats row - restoring minimal look */}
                <div className="flex items-center gap-3 mt-3 pt-3 border-t border-white/10">
                    <div className="flex items-center gap-1.5 text-xs">
                        <Cpu className="w-3.5 h-3.5 text-emerald-400" strokeWidth={2} />
                        <span className="text-white font-semibold">{modelCount}</span>
                        <span className="text-slate-500">models</span>
                    </div>
                    {data.guardrails && (
                        <div className="flex items-center gap-1.5 text-xs text-amber-400">
                            <Shield className="w-3.5 h-3.5" strokeWidth={2} />
                            <span>Protected</span>
                        </div>
                    )}
                </div>

                {/* Strategy indicator — Lucide icon + label per strategy. */}
                <div className="mt-3 px-2 py-1 bg-white/5 rounded-lg text-[10px] text-slate-400 text-center flex items-center justify-center gap-1.5">
                    {(() => {
                        const STRATEGY: Record<string, { icon: LucideIcon; label: string }> = {
                            planner: { icon: Brain, label: 'Planner' },
                            weighted_random: { icon: Scale, label: 'Weighted' },
                            least_cost: { icon: DollarSign, label: 'Cost' },
                            least_latency: { icon: Zap, label: 'Speed' },
                            health: { icon: HeartPulse, label: 'Health' },
                            none: { icon: Scale, label: 'None' },
                        }
                        const key = data.isAgentic ? 'planner' : ((data.strategy as string) || 'weighted_random')
                        const entry = STRATEGY[key]
                        if (!entry) {
                            return <span className="capitalize">{(data.strategy as string)?.replace('_', ' ') || 'Weighted'}</span>
                        }
                        const Icon = entry.icon
                        return (
                            <>
                                <Icon className="w-3 h-3" strokeWidth={2} />
                                <span>{entry.label}</span>
                            </>
                        )
                    })()}
                </div>
            </div>

            <Handle type="source" position={Position.Right} className="w-3 h-3 !bg-transparent !border-transparent opacity-0" />
        </div>
    )
})
