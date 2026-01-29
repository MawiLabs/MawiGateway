'use client'

import { memo } from 'react'
import { Handle, Position } from '@xyflow/react'
import { useRouter } from 'next/navigation'

import Image from 'next/image'

const LOGO_MAP: Record<string, string> = {
    'openai': '/providers/openai.png',
    'azure': '/providers/azure.png',
    'google': '/providers/gemini.png',
    'anthropic': '/providers/anthropic.png',
    'xai': '/providers/xai.png',
    'mistral': '/providers/mistral.png',
    'elevenlabs': '/providers/elevenlabs.png',
    'perplexity': '/providers/perplexity.png',
    'selfhosted': '/providers/self-hosted.png',
    'deepseek': '/providers/deepseek.png',
}

export default memo(function GatewayNode({ data }: any) {
    const providers = data.providers || []
    const router = useRouter()

    const handleAddProvider = (e: React.MouseEvent) => {
        e.stopPropagation()
        router.push('/providers')
    }

    const handleAddService = (e: React.MouseEvent) => {
        e.stopPropagation()
        router.push('/services')
    }

    return (
        <div className="relative group">
            <Handle type="target" position={Position.Left} className="w-3 h-3 !bg-cyan-400/50 !border-2 !border-cyan-400/30" />

            {/* Massive glow effect */}
            <div className="absolute -inset-4 bg-gradient-to-r from-cyan-400/30 via-blue-500/20 to-cyan-400/30 rounded-3xl opacity-60 blur-3xl animate-pulse" />
            <div className="absolute -inset-2 bg-gradient-to-br from-cyan-400/20 to-blue-600/20 rounded-3xl opacity-0 group-hover:opacity-100 blur-2xl transition-all duration-500" />

            <div className="relative px-8 py-6 min-w-[300px] rounded-3xl backdrop-blur-2xl bg-gradient-to-br from-[#0a1520] via-[#0f1a25] to-[#051015] border-2 border-cyan-400/40 shadow-2xl shadow-cyan-500/20">

                {/* Accent bars */}
                <div className="absolute top-0 left-6 right-6 h-1 bg-gradient-to-r from-transparent via-cyan-400 to-transparent rounded-full" />
                <div className="absolute bottom-0 left-8 right-8 h-0.5 bg-gradient-to-r from-transparent via-cyan-400/50 to-transparent" />

                {/* Central badge */}
                <div className="absolute -top-4 left-1/2 -translate-x-1/2 px-4 py-1.5 text-xs font-bold uppercase tracking-widest bg-gradient-to-r from-cyan-400 to-blue-500 text-white rounded-full shadow-lg shadow-cyan-500/40">
                    AI Gateway
                </div>

                {/* Provider chips */}
                <div className="space-y-2">
                    {providers.map((provider: any, i: number) => {
                        const logo = provider.icon_url || LOGO_MAP[provider.provider_type]
                        return (
                            <div
                                key={i}
                                onClick={(e) => {
                                    e.stopPropagation()
                                    router.push(`/providers?id=${provider.id}`)
                                }}
                                className="nodrag group/provider relative px-4 py-2.5 rounded-xl bg-gradient-to-r from-white/5 to-white/[0.02] border border-white/10 hover:border-cyan-400/40 hover:bg-white/5 transition-all duration-300 cursor-pointer"
                            >
                                <div className="flex items-center gap-3">
                                    {logo ? (
                                        <div className="relative w-8 h-8 rounded-lg overflow-hidden bg-white p-0.5">
                                            <Image
                                                src={logo}
                                                alt={provider.name}
                                                fill
                                                className="object-contain"
                                            />
                                        </div>
                                    ) : (
                                        <div className="w-8 h-8 rounded-lg flex items-center justify-center text-lg bg-purple-400/20 text-purple-400">
                                            ꩜
                                        </div>
                                    )}
                                    <div>
                                        <div className="text-sm font-semibold text-white">
                                            {provider.name.replace(/\s*provider$/i, '')}
                                        </div>
                                        <div className="text-[10px] text-slate-500 uppercase tracking-wide">
                                            {provider.provider_type.replace(/[-_]/g, ' ').replace(/\s*provider$/i, '')}
                                        </div>
                                    </div>
                                    {provider.has_api_key && (
                                        <div className="ml-auto w-2 h-2 rounded-full bg-emerald-400 shadow shadow-emerald-400/50" title="API Key Set" />
                                    )}
                                </div>
                            </div>
                        )
                    })}

                    {/* Add Provider Button - Inline */}
                    <button
                        onClick={handleAddProvider}
                        className="w-full group/add relative px-4 py-2.5 rounded-xl bg-gradient-to-r from-cyan-500/10 to-blue-500/10 border border-cyan-500/30 hover:border-cyan-400 hover:bg-cyan-500/20 transition-all duration-300 cursor-pointer text-left"
                    >
                        <div className="flex items-center gap-3">
                            <div className="w-8 h-8 rounded-lg flex items-center justify-center text-lg bg-cyan-400/20 text-cyan-400 group-hover/add:bg-cyan-400 group-hover/add:text-black transition-colors">
                                +
                            </div>
                            <div>
                                <div className="text-sm font-bold text-cyan-100 group-hover/add:text-white">Add Provider</div>
                                <div className="text-[10px] text-cyan-500/70 group-hover/add:text-cyan-300 uppercase tracking-wide">Connect New AI</div>
                            </div>
                        </div>
                    </button>
                </div>

                {/* Stats footer */}
                <div className="flex items-center justify-center gap-4 mt-4 pt-3 border-t border-white/5 text-xs text-slate-500">
                    <span>{providers.length} providers</span>
                    <span className="w-1 h-1 rounded-full bg-slate-700" />
                    <span className="text-emerald-400">● Active</span>
                </div>
            </div>

            {/* Add Service Button - Right Edge */}
            <div className="absolute top-1/2 -right-0 -translate-y-1/2 translate-x-1/2 z-50">
                <button
                    onClick={handleAddService}
                    className="group/service flex items-center justify-center w-10 h-10 rounded-full bg-gradient-to-br from-purple-500 to-indigo-600 border-2 border-purple-400/50 shadow-lg shadow-purple-500/30 hover:scale-110 active:scale-95 transition-all duration-300"
                    title="Add Service"
                >
                    <span className="text-xl font-bold text-white group-hover/service:rotate-90 transition-transform duration-300">+</span>

                    {/* Tooltip */}
                    <div className="absolute left-full ml-3 px-3 py-1.5 bg-gray-900 border border-white/10 rounded-lg text-xs font-medium text-purple-200 whitespace-nowrap opacity-0 group-hover/service:opacity-100 -translate-x-2 group-hover/service:translate-x-0 transition-all pointer-events-none">
                        Add Service
                    </div>
                </button>
            </div>

            <Handle type="source" position={Position.Right} className="w-3 h-3 !bg-purple-400/50 !border-2 !border-purple-400/30" />
        </div>
    )
})
