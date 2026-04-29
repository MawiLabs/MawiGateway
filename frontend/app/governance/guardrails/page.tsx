'use client'

import { motion } from 'framer-motion'
import { Card, Badge } from '@/components/ui'
import { Shield } from 'lucide-react'

export default function GuardrailsPage() {
    return (
        <div className="p-8">
            <div className="max-w-7xl mx-auto">
                <motion.div
                    initial={{ opacity: 0, y: 20 }}
                    animate={{ opacity: 1, y: 0 }}
                    className="mb-8">
                    <h1 className="text-3xl font-bold bg-gradient-to-br from-white to-slate-400 bg-clip-text text-transparent leading-[1.1] mb-2">
                        Guardrails
                    </h1>

                </motion.div>

                <Card>
                    <div className="py-24 text-center relative pointer-events-none opacity-80 select-none">
                        <div className="absolute top-4 right-4">
                            <Badge variant="cyan" size="sm">Coming Soon</Badge>
                        </div>
                        <div className="inline-flex items-center justify-center w-14 h-14 rounded-2xl bg-cyan-400/10 border border-cyan-400/20 mb-6 shadow-[0_0_24px_rgba(34,211,238,0.18)]">
                            <Shield className="w-7 h-7 text-cyan-400" strokeWidth={1.75} />
                        </div>
                        <h2 className="text-xl font-semibold text-white mb-2">Safety Controls</h2>
                        <p className="text-slate-400 max-w-md mx-auto mb-4 text-sm leading-relaxed">
                            Guardrails let you set policies on what models can be called, by whom, and what content gets blocked. We&apos;re building this — check back soon.
                        </p>
                        <p className="text-slate-500 max-w-md mx-auto text-sm">
                            Configure PII filtering, content moderation strategies, and regulatory compliance rules.
                        </p>
                    </div>
                </Card>
            </div>
        </div>
    )
}
