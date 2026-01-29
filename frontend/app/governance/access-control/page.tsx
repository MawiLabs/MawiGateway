'use client'

import { useEffect, useState } from 'react'
import { motion, AnimatePresence } from 'framer-motion'
import { Card, Badge, Button, Input, Modal } from '@/components/ui'
import { toast } from 'sonner'

type Tab = 'users' | 'org' | 'policies' | 'api_keys' | 'human_in_loop'

interface ApiKey {
    id: string
    name: string
    prefix: string
    created_at: string
    expires_at?: string
    last_used_at?: string
}

interface NewApiKey {
    id: string
    name: string
    prefix: string
    raw_key: string
    created_at: string
}

interface UserProfile {
    id: string
    email: string
    name?: string
    tier: string
    monthly_quota_usd: number
    current_usage_usd: number
    is_free_tier: boolean
}

interface QuotaStatus {
    personal_quota: number
    personal_used: number
    personal_remaining: number
    personal_percentage: number
    org_quota_available: number
    org_percentage: number
    total_available: number
}

export default function AccessControlPage() {
    const [activeTab, setActiveTab] = useState<Tab>('users')
    const [searchQuery, setSearchQuery] = useState('')
    const [hitlUsers, setHitlUsers] = useState<string[]>([])
    const [newHitlEmail, setNewHitlEmail] = useState('')

    const handleAddHitlUser = () => {
        if (newHitlEmail && !hitlUsers.includes(newHitlEmail)) {
            setHitlUsers([...hitlUsers, newHitlEmail])
            setNewHitlEmail('')
            toast.success('User added to notification list')
        }
    }

    const removeHitlUser = (email: string) => {
        setHitlUsers(hitlUsers.filter(u => u !== email))
    }

    // Data & State
    const [apiKeys, setApiKeys] = useState<ApiKey[]>([])
    const [isCreateKeyModalOpen, setIsCreateKeyModalOpen] = useState(false)
    const [isSuccessModalOpen, setIsSuccessModalOpen] = useState(false)
    const [newCreatedKey, setNewCreatedKey] = useState<NewApiKey | null>(null)
    const [createForm, setCreateForm] = useState({ name: '', expires_in_days: 90 })

    // Real Data State
    const [userProfile, setUserProfile] = useState<UserProfile | null>(null)
    const [quotaStatus, setQuotaStatus] = useState<QuotaStatus | null>(null)
    const [loadingData, setLoadingData] = useState(false)
    const [loadingKeys, setLoadingKeys] = useState(false)

    // Revoke Modal
    const [showRevokeModal, setShowRevokeModal] = useState(false)
    const [keyToRevoke, setKeyToRevoke] = useState<ApiKey | null>(null)



    // Initial Data Fetch
    useEffect(() => {
        fetchData()
    }, [])

    // Load keys when tab changes
    useEffect(() => {
        if (activeTab === 'api_keys') {
            loadKeys()
        }
    }, [activeTab])

    const fetchData = async () => {
        setLoadingData(true)
        try {
            // Fetch User Profile
            const userRes = await fetch('/v1/user/me', { credentials: 'include' })
            if (userRes.ok) setUserProfile(await userRes.json())

            // Fetch Quota Status
            const quotaRes = await fetch('/v1/user/quota', { credentials: 'include' })
            if (quotaRes.ok) setQuotaStatus(await quotaRes.json())
        } catch (e) {
            console.error(e)
            toast.error('Failed to load access data')
        } finally {
            setLoadingData(false)
        }
    }

    const loadKeys = async () => {
        setLoadingKeys(true)
        try {
            const res = await fetch('/v1/user/api-keys', {
                credentials: 'include'
            })
            if (res.ok) {
                setApiKeys(await res.json())
            }
        } catch (e) {
            console.error(e)
            toast.error('Failed to load API keys')
        } finally {
            setLoadingKeys(false)
        }
    }

    // Create Key
    const handleCreateKey = async () => {
        try {
            const res = await fetch('/v1/user/api-keys', {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                },
                credentials: 'include',
                body: JSON.stringify({
                    name: createForm.name,
                    expires_in_days: createForm.expires_in_days === -1 ? null : createForm.expires_in_days
                })
            })
            if (res.ok) {
                const data = await res.json()
                setNewCreatedKey(data)
                setIsCreateKeyModalOpen(false)
                setIsSuccessModalOpen(true)
                loadKeys()
                toast.success('API Key generated')
            } else {
                toast.error('Failed to generate key')
            }
        } catch (e) {
            console.error(e)
            toast.error('Error generating key')
        }
    }

    // Delete Key
    const handleDeleteKey = (key: ApiKey) => {
        setKeyToRevoke(key)
        setShowRevokeModal(true)
    }

    const confirmRevoke = async () => {
        if (!keyToRevoke) return
        try {
            const res = await fetch(`/v1/user/api-keys/${keyToRevoke.id}`, {
                method: 'DELETE',
                credentials: 'include'
            })
            if (res.ok) {
                toast.success('API Key revoked')
                loadKeys()
                setShowRevokeModal(false)
            } else {
                toast.error('Failed to revoke key')
            }
        } catch (e) {
            console.error(e)
            toast.error('Error revoking key')
        }
    }

    // load keys when tab changes to api_keys
    // Simplification: just call loadKeys() when clicking tab? or useEffect

    const users = [
        { id: '1', name: 'Alice Smith', email: 'alice@example.com', role: 'admin', status: 'active', lastActive: '2 mins ago' },
        { id: '2', name: 'Bob Jones', email: 'bob@example.com', role: 'editor', status: 'active', lastActive: '1 hour ago' },
        { id: '3', name: 'Charlie Day', email: 'charlie@example.com', role: 'viewer', status: 'inactive', lastActive: '3 days ago' },
    ]

    const organization = {
        name: 'Acme Corp AI',
        tier: 'Enterprise',
        quota_usage: 1250.50,
        quota_limit: 5000.00,
        members_count: 12,
        api_keys_active: 8
    }

    const policies = [
        { id: '1', name: 'Global Rate Limit', description: 'Limit all users to 1000 requests/min', scope: 'Global', status: 'active' },
        { id: '2', name: 'PII Scrubbing', description: 'Remove detected PII from prompts', scope: 'Organization', status: 'active' },
        { id: '3', name: 'Cost Cap', description: 'Stop service when quota reaches 90%', scope: 'Organization', status: 'inactive' },
    ]

    return (
        <div className="p-8">
            <div className="max-w-7xl mx-auto space-y-6">
                {/* Header */}
                <motion.div
                    initial={{ opacity: 0, y: 20 }}
                    animate={{ opacity: 1, y: 0 }}>
                    <h1 className="text-3xl font-bold gradient-text-white mb-2">
                        Access Control
                    </h1>
                    <p className="text-slate-400">
                        Manage users, organization settings, and governance policies.
                    </p>
                </motion.div>

                {/* Tabs */}
                <div className="flex space-x-1 bg-white/5 p-1 rounded-xl w-fit">
                    {(['users', 'org', 'policies', 'api_keys', 'human_in_loop'] as Tab[]).map((tab) => (
                        <button
                            key={tab}
                            onClick={() => setActiveTab(tab)}
                            className={`
                                relative px-6 py-2.5 rounded-lg text-sm font-medium transition-all duration-200
                                ${activeTab === tab ? 'text-white' : 'text-slate-400 hover:text-white'}
                            `}
                        >
                            {activeTab === tab && (
                                <motion.div
                                    layoutId="active-tab-bg"
                                    className="absolute inset-0 bg-white/10 rounded-lg"
                                    transition={{ type: 'spring', bounce: 0.2, duration: 0.6 }}
                                />
                            )}
                            <span className="relative capitalize">
                                {tab === 'org' ? 'Organization' :
                                    tab === 'api_keys' ? 'API Keys' :
                                        tab === 'human_in_loop' ? 'Human-in-the-loop' : tab}
                            </span>
                        </button>
                    ))}
                </div>

                {/* Content Area */}
                <AnimatePresence mode="wait">
                    <motion.div
                        key={activeTab}
                        initial={{ opacity: 0, x: 10 }}
                        animate={{ opacity: 1, x: 0 }}
                        exit={{ opacity: 0, x: -10 }}
                        transition={{ duration: 0.2 }}
                    >
                        {activeTab === 'users' && (
                            // Users Tab Content
                            <Card className="overflow-hidden">
                                <div className="p-4 border-b border-white/10 flex justify-between items-center">
                                    <Input
                                        placeholder="Search users..."
                                        value={searchQuery}
                                        onChange={(e) => setSearchQuery(e.target.value)}
                                        className="max-w-xs"
                                    />
                                    <Button variant="primary" size="sm" disabled>Invite User (Pro)</Button>
                                </div>
                                <table className="w-full text-sm text-left">
                                    <thead className="bg-white/5 text-slate-400 uppercase text-xs">
                                        <tr>
                                            <th className="px-6 py-3">Name</th>
                                            <th className="px-6 py-3">Role / Tier</th>
                                            <th className="px-6 py-3">Status</th>
                                            <th className="px-6 py-3">Usage</th>
                                            <th className="px-6 py-3"></th>
                                        </tr>
                                    </thead>
                                    <tbody className="divide-y divide-white/10">
                                        {userProfile ? (
                                            <tr key={userProfile.id} className="hover:bg-white/5 transition-colors">
                                                <td className="px-6 py-4">
                                                    <div className="font-medium text-white">{userProfile.name || 'User'}</div>
                                                    <div className="text-slate-500 text-xs">{userProfile.email}</div>
                                                </td>
                                                <td className="px-6 py-4">
                                                    <Badge size="sm" variant="success">
                                                        🌱 Community
                                                    </Badge>
                                                </td>
                                                <td className="px-6 py-4">
                                                    <div className="flex items-center gap-2">
                                                        <div className="w-1.5 h-1.5 rounded-full bg-emerald-400" />
                                                        <span className="capitalize text-slate-300">Active</span>
                                                    </div>
                                                </td>
                                                <td className="px-6 py-4 text-slate-400">
                                                    Free
                                                </td>
                                                <td className="px-6 py-4 text-right">
                                                    <Button variant="ghost" size="sm">Edit</Button>
                                                </td>
                                            </tr>
                                        ) : (
                                            <tr><td colSpan={5} className="p-4 text-center text-slate-500">Loading profile...</td></tr>
                                        )}
                                    </tbody>
                                </table>
                            </Card>
                        )}

                        {activeTab === 'org' && (
                            // Organization Tab Content
                            <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
                                <Card className="p-6">
                                    <h3 className="text-lg font-bold text-white mb-4">Workspace Statistics</h3>
                                    <div className="space-y-4">
                                        <div className="flex justify-between items-center py-2 border-b border-white/5">
                                            <span className="text-slate-400">Workspace Name</span>
                                            <span className="text-white font-medium">{(userProfile?.name || 'My') + " Workspace"}</span>
                                        </div>
                                        <div className="flex justify-between items-center py-2 border-b border-white/5">
                                            <span className="text-slate-400">Edition</span>
                                            <Badge variant="success">🌱 Community (Free)</Badge>
                                        </div>
                                        <div className="flex justify-between items-center py-2 border-b border-white/5">
                                            <span className="text-slate-400">Members</span>
                                            <span className="text-white font-medium">1 (Personal)</span>
                                        </div>
                                        <div className="flex justify-between items-center py-2 border-b border-white/5">
                                            <span className="text-slate-400">Active API Keys</span>
                                            <span className="text-white font-medium">{apiKeys.length}</span>
                                        </div>
                                    </div>
                                    <div className="mt-6">
                                        <Button variant="secondary" className="w-full" disabled>Edit Organization (Pro)</Button>
                                    </div>
                                </Card>

                                <Card className="p-6">
                                    <h3 className="text-lg font-bold text-white mb-4">Community Edition</h3>
                                    <div className="space-y-4">
                                        <div className="p-4 rounded-xl bg-gradient-to-r from-emerald-400/10 to-emerald-600/10 border border-emerald-400/30">
                                            <div className="flex items-center gap-3 mb-3">
                                                <div className="text-3xl">🌱</div>
                                                <div>
                                                    <div className="text-white font-semibold">Free & Open Source</div>
                                                    <div className="text-sm text-slate-300">No usage limits or quotas</div>
                                                </div>
                                            </div>
                                            <div className="text-sm text-slate-400">
                                                Full access to all current features. Always free, always open source.
                                            </div>
                                        </div>
                                        <div className="p-4 rounded-xl bg-gradient-to-r from-purple-400/10 to-purple-600/10 border border-purple-400/30">
                                            <div className="flex items-start gap-3">
                                                <div className="text-2xl">🚀</div>
                                                <div>
                                                    <div className="text-white font-semibold mb-1">Team & Enterprise Coming Soon</div>
                                                    <ul className="text-xs text-slate-400 space-y-1">
                                                        <li>• Multi-user collaboration</li>
                                                        <li>• Usage quotas & billing</li>
                                                        <li>• Advanced governance</li>
                                                        <li>• Priority support</li>
                                                    </ul>
                                                </div>
                                            </div>
                                        </div>
                                    </div>
                                </Card>
                            </div>
                        )}

                        {activeTab === 'policies' && (
                            // Policies Tab Content
                            <Card className="overflow-hidden relative pointer-events-none opacity-80 select-none">
                                <div className="p-4 border-b border-white/10 flex justify-between items-center">
                                    <div className='text-sm text-slate-400'>
                                        Policies within the Agentic gateway and the user access.
                                    </div>
                                    <Button variant="primary" size="sm" disabled>+ New Policy</Button>
                                </div>
                                <div className="p-8 text-center">
                                    <div className="mb-4">
                                        <Badge variant="cyan" size="sm">Coming Soon</Badge>
                                    </div>
                                    <h3 className="text-white font-bold mb-2">Policies Management</h3>
                                </div>
                            </Card>
                        )}

                        {activeTab === 'api_keys' && (
                            <div className="space-y-6">
                                <div className="flex justify-between items-center">
                                    <div>
                                        <h2 className="text-xl font-bold text-white">API Keys</h2>
                                        <p className="text-slate-400 text-sm">Manage programmatic access keys.</p>
                                    </div>
                                    <Button onClick={() => {
                                        setCreateForm({ name: '', expires_in_days: 90 })
                                        setIsCreateKeyModalOpen(true)
                                    }} variant="primary">
                                        Generate New Key
                                    </Button>
                                </div>

                                <Card className="overflow-hidden">
                                    <div className="p-1">
                                        <Button variant="ghost" size="sm" onClick={loadKeys} className="mb-2">↻ Refresh List</Button>
                                    </div>
                                    <table className="w-full text-sm text-left">
                                        <thead className="bg-white/5 text-slate-400 uppercase text-xs">
                                            <tr>
                                                <th className="px-6 py-3">Name</th>
                                                <th className="px-6 py-3">Key Prefix</th>
                                                <th className="px-6 py-3">Created</th>
                                                <th className="px-6 py-3">Expires</th>
                                                <th className="px-6 py-3">Last Used</th>
                                                <th className="px-6 py-3"></th>
                                            </tr>
                                        </thead>
                                        <tbody className="divide-y divide-white/10">
                                            {apiKeys.length === 0 && (
                                                <tr>
                                                    <td colSpan={6} className="px-6 py-8 text-center text-slate-500">
                                                        No API keys found. Generate one to get started.
                                                    </td>
                                                </tr>
                                            )}
                                            {apiKeys.map((key) => (
                                                <tr key={key.id} className="group hover:bg-white/5 transition-colors">
                                                    <td className="px-6 py-4 font-medium text-white">{key.name}</td>
                                                    <td className="px-6 py-4 font-mono text-slate-400">{key.prefix}</td>
                                                    <td className="px-6 py-4 text-slate-400">{key.created_at}</td>
                                                    <td className="px-6 py-4 text-slate-400">{key.expires_at || 'Never'}</td>
                                                    <td className="px-6 py-4 text-slate-400">{key.last_used_at || 'Never'}</td>
                                                    <td className="px-6 py-4 text-right">
                                                        <Button
                                                            onClick={() => handleDeleteKey(key)}
                                                            variant="danger"
                                                            size="sm"
                                                            className="opacity-0 group-hover:opacity-100 transition-opacity"
                                                        >
                                                            Revoke
                                                        </Button>
                                                    </td>
                                                </tr>
                                            ))}
                                        </tbody>
                                    </table>
                                </Card>
                            </div>
                        )}

                        {activeTab === 'human_in_loop' && (
                            // Human in Loop Tab Content
                            <Card className="overflow-hidden relative pointer-events-none opacity-80 select-none">
                                <div className="p-4 border-b border-white/10 flex justify-between items-center">
                                    <div className='text-sm text-slate-400'>
                                        Manage human review and approval workflows.
                                    </div>
                                    <Button variant="primary" size="sm" disabled>+ New Workflow</Button>
                                </div>
                                <div className="p-8 text-center">
                                    <div className="mb-4">
                                        <Badge variant="cyan" size="sm">Coming Soon</Badge>
                                    </div>
                                    <h3 className="text-white font-bold mb-2">Human-in-the-loop Workflows</h3>

                                </div>
                            </Card>
                        )}

                    </motion.div>
                </AnimatePresence>

                {/* Create Key Modal */}
                {isCreateKeyModalOpen && (
                    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/80 backdrop-blur-sm">
                        <Card className="w-full max-w-md p-6 bg-slate-900 border-slate-700">
                            <h2 className="text-xl font-bold text-white mb-4">Generate API Key</h2>
                            <div className="space-y-4">
                                <div>
                                    <label className="block text-sm font-medium text-slate-400 mb-1">Key Name</label>
                                    <Input
                                        placeholder="e.g. CI/CD Pipeline"
                                        value={createForm.name}
                                        onChange={e => setCreateForm({ ...createForm, name: e.target.value })}
                                    />
                                </div>
                                <div>
                                    <label className="block text-sm font-medium text-slate-400 mb-1">Expiration</label>
                                    <select
                                        className="w-full bg-black/50 border border-white/10 rounded-lg px-3 py-2 text-white text-sm focus:outline-none focus:ring-2 focus:ring-cyan-500"
                                        value={createForm.expires_in_days}
                                        onChange={e => setCreateForm({ ...createForm, expires_in_days: parseInt(e.target.value) })}
                                    >
                                        <option value={30}>30 Days</option>
                                        <option value={90}>90 Days</option>
                                        <option value={365}>1 Year</option>
                                        <option value={-1}>Never</option>
                                    </select>
                                </div>
                                <div className="flex gap-3 justify-end mt-6">
                                    <Button variant="ghost" onClick={() => setIsCreateKeyModalOpen(false)}>Cancel</Button>
                                    <Button variant="primary" onClick={handleCreateKey} disabled={!createForm.name}>Generate Key</Button>
                                </div>
                            </div>
                        </Card>
                    </div>
                )}

                {/* Success Modal */}
                {isSuccessModalOpen && newCreatedKey && (
                    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/80 backdrop-blur-sm">
                        <Card className="w-full max-w-lg p-6 bg-slate-900 border-slate-700 border-l-4 border-l-emerald-500">
                            <h2 className="text-xl font-bold text-white mb-2">API Key Generated</h2>
                            <p className="text-slate-400 text-sm mb-4">
                                Please copy this key now. You will not be able to see it again.
                            </p>

                            <div className="bg-black/50 border border-white/10 rounded-lg p-3 flex items-center justify-between gap-4 mb-6">
                                <code className="text-emerald-400 font-mono text-sm break-all">
                                    {newCreatedKey.raw_key}
                                </code>
                                <Button
                                    size="sm"
                                    variant="secondary"
                                    onClick={() => {
                                        navigator.clipboard.writeText(newCreatedKey.raw_key)
                                    }}
                                >
                                    Copy
                                </Button>
                            </div>

                            <div className="flex justify-end">
                                <Button variant="primary" onClick={() => setIsSuccessModalOpen(false)}>Done</Button>
                            </div>
                        </Card>
                    </div>
                )}
            </div>
            {/* Revoke Confirmation Modal */}
            <Modal
                isOpen={showRevokeModal}
                onClose={() => setShowRevokeModal(false)}
                title="Revoke API Key"
                description="Are you sure you want to revoke this key?"
            >
                <div>
                    <div className="p-3 bg-red-500/10 border border-red-500/20 rounded-lg text-sm text-red-200 mb-6">
                        ⚠️ This will immediately stop any applications using this key.
                        <div className="mt-2 font-mono text-white">{keyToRevoke?.name} ({keyToRevoke?.prefix}...)</div>
                    </div>
                    <div className="flex justify-end gap-3">
                        <Button variant="ghost" onClick={() => setShowRevokeModal(false)}>Cancel</Button>
                        <Button variant="danger" onClick={confirmRevoke}>
                            Revoke Key
                        </Button>
                    </div>
                </div>
            </Modal>
        </div>
    )
}
