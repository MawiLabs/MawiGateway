'use client'

import { memo } from 'react'
import { Handle, Position } from '@xyflow/react'

import Image from 'next/image'

interface ModelNodeData {
    name: string
    provider: string
    modality: string
    isLeader: boolean
    weight: number
    is_healthy?: boolean
    health_status?: 'healthy' | 'warning' | 'unhealthy'
    logo?: string
    isPlanner?: boolean
}

export default memo(function ModelNode({ data }: { data: ModelNodeData }) {
    return (
        <div className="relative group">
            <Handle type="target" position={Position.Left} className={`w-2 h-2 !border-0 ${data.isPlanner ? '!bg-indigo-500' : '!bg-emerald-400/50'}`} />

            {/* Glow effect on hover */}
            <div className="absolute -inset-1 bg-gradient-to-r from-emerald-400/20 to-cyan-400/20 rounded-2xl opacity-0 group-hover:opacity-100 blur-xl transition-all duration-500" />

            <div className={`relative px-4 py-3 w-[180px] rounded-xl backdrop-blur-xl border transition-all duration-300
                ${data.isLeader
                    ? 'bg-gradient-to-br from-emerald-400/20 to-emerald-600/10 border-emerald-400/40 shadow-lg shadow-emerald-400/10'
                    : 'bg-gradient-to-br from-white/10 to-white/5 border-white/20 hover:border-cyan-400/40'
                }`}>

                {/* Leader badge */}
                {(data.isLeader || data.isPlanner) && (
                    <div className={`absolute -top-2 -right-2 px-2 py-0.5 text-[10px] font-bold uppercase tracking-wider rounded-full
                        ${data.isPlanner
                            ? 'bg-indigo-500 text-white shadow-lg shadow-indigo-500/30'
                            : 'bg-emerald-400 text-black'}`}>
                        {data.isPlanner ? 'Planner' : 'Primary'}
                    </div>
                )}

                {/* Model icon and name */}
                <div className="flex items-center gap-2 mb-1">
                    {data.logo ? (
                        <div className="relative w-6 h-6 shrink-0 rounded-lg overflow-hidden bg-white p-0.5">
                            <Image
                                src={data.logo}
                                alt={data.provider}
                                fill
                                className="object-contain" // removed p-padding to maximize logo size in small container
                            />
                        </div>
                    ) : (
                        <div className={`w-6 h-6 shrink-0 rounded-lg flex items-center justify-center text-sm
                            ${data.isLeader
                                ? 'bg-emerald-400/30 text-emerald-400'
                                : 'bg-cyan-400/20 text-cyan-400'
                            }`}>
                            ꩜
                        </div>
                    )}
                    <span className="font-semibold text-white text-sm truncate flex-1 block">
                        {data.name}
                    </span>
                    {data.health_status === 'warning' && (
                        <span title="Warning: Rate Limited" className="text-sm">⚠️</span>
                    )}
                    {data.health_status === 'unhealthy' && (
                        <span title="Model Unhealthy" className="text-sm">⛔️</span>
                    )}
                    {!data.health_status && data.is_healthy === false && (
                        <span title="Model Unhealthy" className="text-sm">⛔️</span>
                    )}
                </div>

                {/* Provider and modality */}
                <div className="flex items-center gap-2 text-[10px]">
                    <span className="px-1.5 py-0.5 bg-white/10 rounded text-slate-400">
                        {data.provider}
                    </span>
                    <span className="px-1.5 py-0.5 bg-purple-400/20 rounded text-purple-300">
                        {data.modality}
                    </span>
                </div>

                {/* Weight indicator */}
                <div className="mt-2 flex items-center gap-1.5">
                    <div className="flex-1 h-1 bg-white/10 rounded-full overflow-hidden">
                        <div
                            className={`h-full rounded-full ${data.isLeader ? 'bg-emerald-400' : 'bg-cyan-400'}`}
                            style={{ width: `${Math.min(data.weight, 100)}%` }}
                        />
                    </div>
                    <span className="text-[9px] text-slate-500">{data.weight}%</span>
                </div>
            </div>

            <Handle type="source" position={Position.Right} className="w-2 h-2 !bg-cyan-400/50 !border-0" />
        </div>
    )
})
