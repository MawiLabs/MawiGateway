'use client'

import { useEffect, useState } from 'react'
import { useRouter } from 'next/navigation'
import { motion } from 'framer-motion'
import { Button, Card, Badge, Input, Modal } from '@/components/ui'
import { useAuth, fetchModels } from '@/contexts/AuthContext'
import { toast } from 'sonner'

interface Service {
    id: string
    name: string
    service_type: string
}

interface ModelInfo {
    id: string
    name: string
    tier: string
    worker_type: string
}

export default function ProfilePage() {
    const router = useRouter()
    const { user, loading, refreshUser } = useAuth()

    const [services, setServices] = useState<Service[]>([])
    const [models, setModels] = useState<ModelInfo[]>([])
    const [editing, setEditing] = useState(false)
    const [name, setName] = useState('')
    const [showDeleteModal, setShowDeleteModal] = useState(false)
    const [deleteConfirmation, setDeleteConfirmation] = useState('')
    const [isDeleting, setIsDeleting] = useState(false)

    // Initial auth checks
    useEffect(() => {
        if (!loading && !user) {
            router.push('/auth/login')
        }
        if (user) {
            setName(user.name || '')
        }
    }, [user, loading, router])

    // Refresh user on mount
    useEffect(() => {
        refreshUser()
    }, [])

    // Fetch services & models once user is loaded
    useEffect(() => {
        if (user) {
            fetch('/v1/user/services', { credentials: 'include' })
                .then(async res => {
                    if (res.ok) setServices(await res.json())
                })
                .catch(err => console.error('Failed to load services:', err))

            fetchModels()
                .then(data => setModels(data.models || []))
                .catch(err => console.error('Failed to load models:', err))
        }
    }, [user])

    if (loading || !user) {
        return (
            <div className="min-h-screen bg-black flex items-center justify-center">
                <div className="text-slate-400 animate-pulse">Loading profile...</div>
            </div>
        )
    }

    // Calculations for usage bar
    const usagePercent = Math.min((user.current_usage_usd / user.monthly_quota_usd) * 100, 100) || 0;
    const isOverLimit = user.current_usage_usd >= user.monthly_quota_usd;

    return (
        <div className="p-8 pb-20">
            <div className="max-w-7xl mx-auto space-y-8">
                {/* Header */}
                <motion.div
                    initial={{ opacity: 0, y: 20 }}
                    animate={{ opacity: 1, y: 0 }}
                    className="flex justify-between items-end">
                    <div>
                        <h1 className="text-3xl font-bold gradient-text-white mb-2">My Profile</h1>
                        <p className="text-slate-400">Manage your account and settings</p>
                    </div>
                </motion.div>

                <div className="grid grid-cols-1 lg:grid-cols-3 gap-8">
                    {/* Left Column: Profile Card */}
                    <motion.div
                        initial={{ opacity: 0, x: -20 }}
                        animate={{ opacity: 1, x: 0 }}
                        transition={{ delay: 0.1 }}
                        className="space-y-6 lg:col-span-1"
                    >
                        <Card className="p-0 overflow-hidden relative group">
                            {/* Banner Background */}
                            <div className="h-32 bg-gradient-to-r from-cyan-400/10 to-blue-600/10 relative">
                                <div className="absolute inset-0 bg-black/20" />
                            </div>

                            {/* Avatar & Basic Info */}
                            <div className="px-6 relative -mt-12 flex flex-col items-center">
                                <div className="w-24 h-24 rounded-full bg-black p-1 mb-4 relative z-10">
                                    <div className="w-full h-full rounded-full bg-[#1a1a1a] flex items-center justify-center text-4xl overflow-hidden border border-white/10">
                                        <div className="bg-gradient-to-br from-slate-700 to-slate-900 w-full h-full flex items-center justify-center">
                                            {user.name ? user.name.charAt(0).toUpperCase() : 'U'}
                                        </div>
                                    </div>
                                    <div className="absolute bottom-1 right-1 w-6 h-6 bg-emerald-500 rounded-full border-2 border-black flex items-center justify-center" title="Online">
                                        <div className="w-2 h-2 bg-white rounded-full animate-pulse" />
                                    </div>
                                </div>

                                <div className="text-center w-full mb-6">
                                    {editing ? (
                                        <div className="mb-2">
                                            <Input
                                                value={name}
                                                onChange={(e) => setName(e.target.value)}
                                                className="text-center"
                                                autoFocus
                                                onBlur={() => setEditing(false)}
                                            />
                                        </div>
                                    ) : (
                                        <h2 className="text-xl font-bold text-white mb-1 flex items-center justify-center gap-2 cursor-pointer hover:text-cyan-400 transition-colors" onClick={() => setEditing(true)}>
                                            {user.name || 'Set your name'}
                                            <span className="text-xs text-slate-500 opacity-0 group-hover:opacity-100 transition-opacity">✎</span>
                                        </h2>
                                    )}
                                    <p className="text-slate-400 text-sm mb-3">{user.email}</p>
                                    <Badge size="sm" variant={user.tier === 'community' ? 'success' : 'primary'}>
                                        {user.tier.charAt(0).toUpperCase() + user.tier.slice(1)} Plan
                                    </Badge>
                                </div>

                                {/* Stats Grid */}
                                <div className="grid grid-cols-2 gap-px bg-white/5 w-full border-t border-white/5">
                                    <div className="p-4 text-center cursor-default hover:bg-white/5 transition-colors">
                                        <div className="text-2xl font-bold text-white">{services.length}</div>
                                        <div className="text-xs text-slate-500 uppercase tracking-wider">Services</div>
                                    </div>
                                    <div className="p-4 text-center cursor-default hover:bg-white/5 transition-colors">
                                        <div className="text-2xl font-bold text-white">{models.length}</div>
                                        <div className="text-xs text-slate-500 uppercase tracking-wider">Models</div>
                                    </div>
                                </div>
                            </div>
                        </Card>

                        {/* Account Details */}
                        <Card className="p-6">
                            <h3 className="text-sm font-semibold text-slate-400 uppercase tracking-wider mb-4">Account Security</h3>
                            <div className="space-y-4">
                                <div className="flex items-center justify-between p-3 rounded-lg hover:bg-white/5 transition-colors group cursor-pointer">
                                    <div className="flex items-center gap-3">
                                        <div className="w-8 h-8 rounded-full bg-white/5 flex items-center justify-center text-slate-400 group-hover:text-white transition-colors">🔐</div>
                                        <div>
                                            <div className="text-white text-sm font-medium">Password</div>
                                            <div className="text-xs text-slate-500">Last changed 3 months ago</div>
                                        </div>
                                    </div>
                                    <Badge variant="cyan" size="sm">Update</Badge>
                                </div>
                                <div className="flex items-center justify-between p-3 rounded-lg hover:bg-white/5 transition-colors group cursor-pointer">
                                    <div className="flex items-center gap-3">
                                        <div className="w-8 h-8 rounded-full bg-white/5 flex items-center justify-center text-slate-400 group-hover:text-white transition-colors">🛡️</div>
                                        <div>
                                            <div className="text-white text-sm font-medium">2FA Authentication</div>
                                            <div className="text-xs text-slate-500">Not enabled</div>
                                        </div>
                                    </div>
                                    <Badge variant="warning" size="sm">Enable</Badge>
                                </div>
                            </div>
                        </Card>

                        {/* System Information */}
                        <Card className="p-6">
                            <h3 className="text-lg font-bold text-white mb-4">System Information</h3>
                            <div className="space-y-4">
                                <div className="p-4 rounded-xl bg-black/40 border border-white/5">
                                    <div className="flex justify-between items-center mb-2">
                                        <div className="text-sm text-slate-400">Gateway Endpoint</div>
                                        <Badge variant="success" size="sm">Online</Badge>
                                    </div>
                                    <div className="flex items-center gap-2">
                                        <code className="flex-1 bg-black/50 p-2.5 rounded text-sm text-cyan-400 font-mono border border-white/5 truncate">
                                            http://localhost:8030/v1
                                        </code>
                                        <Button variant="secondary" size="sm" className="shrink-0" onClick={() => {
                                            navigator.clipboard.writeText('http://localhost:8030/v1')
                                            toast.success('Copied to clipboard')
                                        }}>
                                            Copy
                                        </Button>
                                    </div>
                                    <p className="text-xs text-slate-500 mt-2">
                                        Use this endpoint to connect your SDKs and Agentic clients.
                                    </p>
                                </div>
                            </div>
                        </Card>

                        {/* Danger Zone */}
                        <Card className="p-6 border-red-500/20 bg-red-500/5">
                            <h3 className="text-lg font-bold text-red-400 mb-2">Danger Zone</h3>
                            <p className="text-slate-400 text-sm mb-6">
                                Irreversible actions regarding your organization and data.
                            </p>

                            <div className="flex items-center justify-between p-4 bg-red-500/10 border border-red-500/20 rounded-xl">
                                <div>
                                    <div className="text-white font-medium">Delete Organization</div>
                                    <div className="text-xs text-red-300/70">
                                        Permanently delete this organization and all associated data.
                                    </div>
                                </div>
                                <Button variant="danger" size="sm" onClick={() => setShowDeleteModal(true)}>
                                    Delete
                                </Button>
                            </div>
                        </Card>
                    </motion.div>

                    {/* Right Column: Usage & Settings */}
                    <motion.div
                        initial={{ opacity: 0, x: 20 }}
                        animate={{ opacity: 1, x: 0 }}
                        transition={{ delay: 0.2 }}
                        className="space-y-6 lg:col-span-2"
                    >
                        {/* Subscription & Usage Card - Hidded for Community Tier (BYOK) */}
                        {user.tier !== 'community' && (
                            <Card className="p-6 border-cyan-400/20 relative overflow-hidden">
                                <div className="absolute inset-0 bg-gradient-to-r from-cyan-400/5 to-blue-600/5" />
                                <div className="relative z-10">
                                    <div className="flex justify-between items-start mb-6">
                                        <div>
                                            <h3 className="text-lg font-bold text-white mb-1">Subscription & Usage</h3>
                                            <p className="text-slate-400 text-sm">
                                                Your current plan usage and limits.
                                            </p>
                                        </div>
                                        <Badge variant={isOverLimit ? 'danger' : 'primary'}>
                                            {isOverLimit ? 'Quota Exceeded' : 'Active'}
                                        </Badge>
                                    </div>

                                    <div className="space-y-6">
                                        {/* Progress Bar */}
                                        <div>
                                            {user.monthly_quota_usd > 10000 ? (
                                                <div>
                                                    <div className="flex justify-between text-sm mb-2">
                                                        <span className="text-slate-300">Monthly Usage</span>
                                                        <span className="text-white font-medium">
                                                            ${user.current_usage_usd.toFixed(2)} / <span className="text-emerald-400">Unlimited</span>
                                                        </span>
                                                    </div>
                                                    <div className="p-3 rounded-lg bg-emerald-500/10 border border-emerald-500/20 text-sm text-emerald-200">
                                                        ✨ You have processed <strong>${user.current_usage_usd.toFixed(2)}</strong> worth of AI tasks this month with no hard limits.
                                                    </div>
                                                </div>
                                            ) : (
                                                <>
                                                    <div className="flex justify-between text-sm mb-2">
                                                        <span className="text-slate-300">Monthly Usage</span>
                                                        <span className="text-white font-medium">
                                                            ${user.current_usage_usd.toFixed(2)} / ${user.monthly_quota_usd.toFixed(2)}
                                                        </span>
                                                    </div>
                                                    <div className="h-2 bg-white/10 rounded-full overflow-hidden">
                                                        <motion.div
                                                            initial={{ width: 0 }}
                                                            animate={{ width: `${usagePercent}%` }}
                                                            className={`h-full rounded-full ${isOverLimit ? 'bg-red-500' : 'bg-cyan-400'}`}
                                                        />
                                                    </div>
                                                    <p className="text-xs text-slate-500 mt-2">
                                                        Resets in {Math.ceil(((user.quota_remaining_usd || user.monthly_quota_usd) / (user.monthly_quota_usd || 1)) * 30)} days
                                                    </p>
                                                </>
                                            )}
                                        </div>

                                        <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
                                            <div className="p-4 rounded-xl bg-black/40 border border-white/5">
                                                <div className="text-sm text-slate-400 mb-1">Current Tier</div>
                                                <div className="text-xl font-bold text-white capitalize">{user.tier}</div>
                                            </div>
                                            <div className="p-4 rounded-xl bg-black/40 border border-white/5">
                                                <div className="text-sm text-slate-400 mb-1">Remaining Balance</div>
                                                <div className="text-xl font-bold text-white">
                                                    ${(user.monthly_quota_usd - user.current_usage_usd).toFixed(2)}
                                                </div>
                                            </div>
                                            <div className="p-4 rounded-xl bg-black/40 border border-white/5">
                                                <div className="text-sm text-slate-400 mb-1">Organization</div>
                                                <div className="text-xl font-bold text-white truncate" title={user.organization_id || 'None'}>
                                                    {user.organization_id ? 'Active' : 'None'}
                                                </div>
                                            </div>
                                        </div>
                                    </div>
                                </div>
                            </Card>
                        )}

                        {/* Active Services */}
                        <Card className="p-6">
                            <h3 className="text-lg font-bold text-white mb-4">Active Services</h3>
                            {services.length === 0 ? (
                                <p className="text-slate-400">No active services.</p>
                            ) : (
                                <div className="space-y-3">
                                    {services.map(s => (
                                        <div key={s.id} className="flex items-center justify-between p-4 rounded-xl bg-white/5 border border-white/5 hover:border-white/10 transition-colors">
                                            <div className="flex items-center gap-4">
                                                <div className="w-10 h-10 rounded-lg bg-black flex items-center justify-center text-xl">
                                                    {s.service_type === 'AGENT' ? '🤖' : '🎱'}
                                                </div>
                                                <div>
                                                    <div className="font-medium text-white">{s.name}</div>
                                                    <div className="text-xs text-slate-500 capitalize">{s.service_type.toLowerCase().replace('_', ' ')} Service</div>
                                                </div>
                                            </div>
                                            <div className="flex items-center gap-2">
                                                <div className="flex items-center gap-1.5 px-2 py-1 bg-emerald-500/10 rounded text-xs text-emerald-400 font-medium">
                                                    <div className="w-1.5 h-1.5 rounded-full bg-emerald-400 animate-pulse" />
                                                    Active
                                                </div>
                                                <Button variant="ghost" size="sm" className="opacity-50 hover:opacity-100" onClick={() => router.push('/services')}>Manage</Button>
                                            </div>
                                        </div>
                                    ))}
                                    <Button variant="secondary" className="w-full mt-2" onClick={() => router.push('/services')}>
                                        + Create New Service
                                    </Button>
                                </div>
                            )}
                        </Card>



                    </motion.div>
                </div>
            </div>
            {/* Delete Organization Modal */}
            <Modal
                isOpen={showDeleteModal}
                onClose={() => setShowDeleteModal(false)}
                title="Delete Organization"
                description="This action is irreversible. Please confirm."
            >
                <div className="space-y-4">
                    <div className="p-3 bg-red-500/10 border border-red-500/20 rounded-lg text-sm text-red-200">
                        ⚠️ <strong>Warning:</strong> You are about to permanently delete your organization.
                        All data will be lost, and you will be logged out immediately.
                    </div>
                    <div>
                        <label className="block text-sm text-slate-400 mb-2">
                            Type <span className="text-white font-mono font-bold">DELETE</span> to confirm
                        </label>
                        <Input
                            value={deleteConfirmation}
                            onChange={(e) => setDeleteConfirmation(e.target.value)}
                            placeholder="DELETE"
                            className="font-mono text-center uppercase"
                        />
                    </div>
                    <div className="flex justify-end gap-3 mt-6">
                        <Button variant="ghost" onClick={() => setShowDeleteModal(false)} disabled={isDeleting}>
                            Cancel
                        </Button>
                        <Button
                            variant="danger"
                            onClick={async () => {
                                if (deleteConfirmation !== 'DELETE') return;
                                setIsDeleting(true);
                                try {
                                    const orgId = user?.organization_id || 'default-org';
                                    const res = await fetch(`/v1/organizations/${orgId}`, {
                                        method: 'DELETE',
                                        credentials: 'include'
                                    });

                                    if (res.ok) {
                                        toast.success('Organization deleted. Logging out...');
                                        await fetch('/v1/auth/logout', { method: 'POST', credentials: 'include' });
                                        router.push('/auth/login');
                                    } else {
                                        const error = await res.text();
                                        toast.error(`Failed to delete organization: ${error}`);
                                        setIsDeleting(false);
                                    }
                                } catch (e) {
                                    console.error(e);
                                    toast.error('Failed to delete organization');
                                    setIsDeleting(false);
                                }
                            }}
                            disabled={deleteConfirmation !== 'DELETE' || isDeleting}
                        >
                            {isDeleting ? 'Deleting...' : 'Delete Organization'}
                        </Button>
                    </div>
                </div>
            </Modal>
        </div >
    )
}
