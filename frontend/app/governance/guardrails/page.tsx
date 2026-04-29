'use client'

import { Badge } from '@/components/ui'
import { Shield } from 'lucide-react'

export default function GuardrailsPage() {
    return (
        <div className="relative h-screen overflow-y-auto bg-black">
            <div className="px-10 pt-10 pb-8 max-w-7xl mx-auto">
                <div className="text-[11px] text-slate-500 mb-3 tracking-[0.12em] uppercase font-semibold">
                    Workspace · Governance · Guardrails
                </div>
                <div className="flex items-end justify-between gap-4">
                    <div>
                        <h1 className="text-4xl font-bold bg-gradient-to-br from-white to-slate-400 bg-clip-text text-transparent leading-[1.1]">
                            Guardrails
                        </h1>
                        <p className="text-slate-400 mt-2 text-sm">
                            Policy controls for model access, PII filtering, content moderation, and compliance.
                        </p>
                    </div>
                    <Badge variant="cyan" size="sm">Coming Soon</Badge>
                </div>
            </div>

            <div className="px-10 pb-16 max-w-7xl mx-auto">
                <div className="rounded-2xl border border-white/10 bg-gradient-to-b from-[#0a0a0a] to-[#050505] py-24 text-center relative overflow-hidden pointer-events-none opacity-90 select-none">
                    <span className="pointer-events-none absolute inset-0 bg-[radial-gradient(420px_220px_at_50%_-10%,rgba(34,211,238,0.08),transparent_70%)]" />
                    <div className="relative inline-flex items-center justify-center w-16 h-16 rounded-2xl bg-cyan-400/10 border border-cyan-400/20 mb-5 shadow-[0_0_28px_rgba(34,211,238,0.22)]">
                        <Shield className="w-8 h-8 text-cyan-400" strokeWidth={1.75} />
                    </div>
                    <h2 className="relative text-xl font-semibold text-white mb-2">Safety Controls</h2>
                    <p className="relative text-slate-400 max-w-md mx-auto mb-3 text-sm leading-relaxed">
                        Guardrails let you set policies on what models can be called, by whom, and what content gets blocked. We&apos;re building this — check back soon.
                    </p>
                    <p className="relative text-slate-500 max-w-md mx-auto text-sm leading-relaxed">
                        Configure PII filtering, content moderation strategies, and regulatory compliance rules.
                    </p>
                </div>
            </div>
        </div>
    )
}
