'use client'

import { motion } from 'framer-motion'
import { Card, Badge } from '@/components/ui'

export default function GuardrailsPage() {
    return (
        <div className="p-8">
            <div className="max-w-7xl mx-auto">
                <motion.div
                    initial={{ opacity: 0, y: 20 }}
                    animate={{ opacity: 1, y: 0 }}
                    className="mb-8">
                    <h1 className="text-3xl font-bold gradient-text-white mb-2">
                        Guardrails
                    </h1>

                </motion.div>

                <Card>
                    <div className="py-24 text-center relative pointer-events-none opacity-80 select-none">
                        <div className="absolute top-4 right-4">
                            <Badge variant="cyan" size="sm">Coming Soon</Badge>
                        </div>
                        <div className="text-6xl mb-6">🛡️</div>
                        <h2 className="text-xl font-semibold text-white mb-2">Safety Controls</h2>
                        <p className="text-slate-500 max-w-md mx-auto">
                            Configure PII filtering, content moderation strategies, and regulatory compliance rules.
                        </p>
                    </div>
                </Card>
            </div>
        </div>
    )
}
