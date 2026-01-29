'use client'

import { memo } from 'react'
import { Handle, Position } from '@xyflow/react'
import { useRouter } from 'next/navigation'

export default memo(function UserNode({ data }: any) {
    const router = useRouter()

    const handleClick = () => {
        router.push('/logs')
    }

    return (
        <div className="relative group cursor-pointer" onClick={handleClick}>
            {/* Glow effect */}
            <div className="absolute -inset-2 bg-gradient-to-r from-sky-400/20 to-blue-400/20 rounded-3xl opacity-0 group-hover:opacity-100 blur-2xl transition-all duration-500" />

            <div className="relative px-8 py-6 min-w-[180px] rounded-2xl backdrop-blur-xl bg-gradient-to-br from-sky-500/20 via-[#0a1520] to-[#0f0f0f] border border-sky-400/30 shadow-xl shadow-sky-500/10 transition-all duration-300 group-hover:border-sky-400/50 group-hover:shadow-sky-500/20">

                {/* Badge */}
                <div className="absolute -top-3 left-1/2 -translate-x-1/2 px-3 py-1 text-[10px] font-bold uppercase tracking-widest bg-sky-500 text-white rounded-full shadow-lg shadow-sky-500/30">
                    Client
                </div>

                <div className="text-center pt-2">
                    {/* User avatar */}
                    {data?.user?.profile_image ? (
                        <div className="w-14 h-14 mx-auto mb-3 rounded-2xl overflow-hidden border border-sky-400/30 shadow-lg shadow-sky-500/20">
                            <img src={data.user.profile_image} alt="User" className="w-full h-full object-cover" />
                        </div>
                    ) : (
                        <div className="w-14 h-14 mx-auto mb-3 rounded-2xl bg-gradient-to-br from-sky-400/30 to-blue-500/20 flex items-center justify-center border border-sky-400/30 shadow-lg shadow-sky-500/20">
                            <span className="text-3xl">{data?.user?.name ? data.user.name.charAt(0).toUpperCase() : '👤'}</span>
                        </div>
                    )}

                    <div className="text-base font-semibold text-white">Incoming Traffic</div>
                    <div className="text-[10px] text-sky-400/70 font-mono mt-1">http://localhost:8030</div>
                </div>

                {/* Pulse indicator */}
                <div className="flex items-center justify-center gap-1.5 mt-4">
                    <div className="w-1.5 h-1.5 rounded-full bg-sky-400 animate-pulse" />
                    <span className="text-[10px] text-slate-500">Click to view logs</span>
                </div>
            </div>

            <Handle type="source" position={Position.Right} className="w-3 h-3 !bg-sky-400/50 !border-2 !border-sky-400/30" />
        </div>
    )
})
