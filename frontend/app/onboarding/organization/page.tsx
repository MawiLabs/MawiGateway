'use client'

import { useState, useEffect } from 'react'
import { useRouter } from 'next/navigation'
import { motion } from 'framer-motion'
import { Card, Button, Input } from '@/components/ui'
import { toast } from 'sonner'
import { useAuth } from '@/contexts/AuthContext'

export default function OrganizationSetupPage() {
    const router = useRouter()
    const { user, loading: authLoading } = useAuth()
    const [loading, setLoading] = useState(false)

    // Redirect if already has organization
    useEffect(() => {
        if (!authLoading && user?.organization_id) {
            toast.success('Organization already exists')
            router.push('/')
        }
    }, [user, authLoading, router])
    const [formData, setFormData] = useState({
        name: '',
        type: 'personal',
        industry: ''
    })

    const handleSubmit = async (e: React.FormEvent) => {
        e.preventDefault()

        if (!formData.name.trim()) {
            toast.error('Organization name is required')
            return
        }

        setLoading(true)

        try {
            const res = await fetch('/v1/organizations', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                credentials: 'include',
                body: JSON.stringify({
                    name: formData.name,
                    org_type: formData.type,
                    industry: formData.industry || null
                })
            })

            if (res.ok) {
                toast.success('Organization created successfully!')
                // Redirect to dashboard
                router.push('/')
            } else {
                const error = await res.text()
                toast.error(`Failed to create organization: ${error}`)
            }
        } catch (e) {
            console.error(e)
            toast.error('Failed to create organization')
        } finally {
            setLoading(false)
        }
    }

    return (
        <div className="min-h-screen bg-black flex items-center justify-center p-8">
            <motion.div
                initial={{ opacity: 0, y: 20 }}
                animate={{ opacity: 1, y: 0 }}
                className="w-full max-w-md"
            >
                <Card className="p-8">
                    {/* Header */}
                    <div className="text-center mb-8">
                        <div className="text-4xl mb-4">🏢</div>
                        <h1 className="text-2xl font-bold text-white mb-2">
                            Create Your Organization
                        </h1>
                        <p className="text-slate-400 text-sm">
                            Set up your workspace to get started with MaWi Gateway
                        </p>
                    </div>

                    {/* Form */}
                    <form onSubmit={handleSubmit} className="space-y-6">
                        {/* Organization Name */}
                        <div>
                            <label className="block text-sm font-medium text-slate-300 mb-2">
                                Organization Name *
                            </label>
                            <Input
                                type="text"
                                placeholder="e.g. Acme Corporation"
                                value={formData.name}
                                onChange={(e) => setFormData({ ...formData, name: e.target.value })}
                                required
                                autoFocus
                            />
                        </div>

                        {/* Organization Type */}
                        <div>
                            <label className="block text-sm font-medium text-slate-300 mb-2">
                                Organization Type
                            </label>
                            <select
                                className="w-full bg-black/50 border border-white/10 rounded-lg px-3 py-2 text-white text-sm focus:outline-none focus:ring-2 focus:ring-cyan-500"
                                value={formData.type}
                                onChange={(e) => setFormData({ ...formData, type: e.target.value })}
                            >
                                <option value="personal">Personal</option>
                                <option value="team">Team</option>
                                <option value="company">Company</option>
                            </select>
                        </div>

                        {/* Industry (Optional) */}
                        <div>
                            <label className="block text-sm font-medium text-slate-300 mb-2">
                                Industry <span className="text-slate-500">(optional)</span>
                            </label>
                            <Input
                                type="text"
                                placeholder="e.g. Technology, Healthcare, Finance"
                                value={formData.industry}
                                onChange={(e) => setFormData({ ...formData, industry: e.target.value })}
                            />
                        </div>

                        {/* Submit Button */}
                        <Button
                            type="submit"
                            variant="primary"
                            className="w-full"
                            disabled={loading}
                        >
                            {loading ? 'Creating...' : 'Create Organization'}
                        </Button>
                    </form>

                    {/* Info */}
                    <div className="mt-6 p-4 rounded-lg bg-emerald-400/10 border border-emerald-400/30">
                        <div className="flex items-start gap-3">
                            <div className="text-xl">🌱</div>
                            <div>
                                <div className="text-sm font-semibold text-emerald-400 mb-1">
                                    Community Edition
                                </div>
                                <div className="text-xs text-slate-400">
                                    Free forever • No credit card required • Full access to all current features
                                </div>
                            </div>
                        </div>
                    </div>
                </Card>
            </motion.div>
        </div>
    )
}
