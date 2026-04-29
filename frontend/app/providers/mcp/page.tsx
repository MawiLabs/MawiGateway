'use client'

import { useEffect, useState } from 'react'
import { motion, AnimatePresence } from 'framer-motion'
import { Card, Badge, Button, Input, Modal, Skeleton } from '@/components/ui'
import { toast } from 'sonner'
import {
    Plus,
    Link2,
    Container,
    Terminal,
    Globe,
    Trash2,
    Wrench,
    Sparkles,
    type LucideIcon,
} from 'lucide-react'

interface McpServer {
    id: string
    name: string
    server_type: string
    image_or_command: string
    status: string
    error_message?: string
    created_at: string
    args?: string[]
    env_vars?: Record<string, string>
}

interface McpTool {
    id: string
    server_id: string
    name: string
    description?: string
    input_schema?: object
}

const SERVER_TYPE_INFO: Record<string, {
    icon: LucideIcon
    label: string
    placeholder: string
    description: string
}> = {
    docker: {
        icon: Container,
        label: 'Docker Container',
        placeholder: 'ghcr.io/github/github-mcp-server',
        description: 'Run MCP server as a Docker container'
    },
    stdio: {
        icon: Terminal,
        label: 'Local Process',
        placeholder: '/usr/local/bin/mcp-server',
        description: 'Run a local executable via stdio'
    },
    sse: {
        icon: Globe,
        label: 'Remote SSE',
        placeholder: 'https://mcp.example.com/sse',
        description: 'Connect to a remote SSE endpoint'
    }
}

export default function McpServersPage() {
    const [servers, setServers] = useState<McpServer[]>([])
    const [selectedServer, setSelectedServer] = useState<McpServer | null>(null)
    const [tools, setTools] = useState<McpTool[]>([])
    const [loading, setLoading] = useState(true)
    const [connecting, setConnecting] = useState<string | null>(null)

    // Add/Edit Server Modal
    const [isAddModalOpen, setIsAddModalOpen] = useState(false)
    const [isEditMode, setIsEditMode] = useState(false)
    const [editingServerId, setEditingServerId] = useState<string | null>(null)

    // Delete Modal
    const [showDeleteModal, setShowDeleteModal] = useState(false)
    const [serverToDelete, setServerToDelete] = useState<McpServer | null>(null)

    const [newServer, setNewServer] = useState({
        name: '',
        server_type: 'docker',
        image_or_command: '',
        env_vars: {} as Record<string, string>
    })
    const [newEnvKey, setNewEnvKey] = useState('')
    const [newEnvValue, setNewEnvValue] = useState('')

    useEffect(() => {
        loadServers()
    }, [])

    const loadServers = async () => {
        try {
            const res = await fetch('/v1/mcp/servers', { credentials: 'include' })
            if (res.ok) {
                const data = await res.json()
                setServers(data)
            } else {
                console.error('Failed to load servers:', res.status, res.statusText)
                toast.error(`Failed to load servers: ${res.status} ${res.statusText}`)
            }
        } catch (e) {
            console.error(e)
            toast.error('Error connecting to server')
        } finally {
            setLoading(false)
        }
    }

    const loadTools = async (serverId: string) => {
        console.log('Loading tools for server:', serverId)
        try {
            const res = await fetch(`/v1/mcp/servers/${serverId}/tools`, { credentials: 'include' })
            console.log('Tools fetch response:', res.status)
            if (res.ok) {
                const data = await res.json()
                console.log('Tools loaded:', data)
                setTools(data)
            } else {
                console.error('Failed to load tools:', await res.text())
            }
        } catch (e) {
            console.error('Error loading tools:', e)
        }
    }

    const openAddModal = () => {
        setIsEditMode(false)
        setEditingServerId(null)
        setNewServer({ name: '', server_type: 'docker', image_or_command: '', env_vars: {} })
        setIsAddModalOpen(true)
    }

    const openEditModal = (server: McpServer) => {
        setIsEditMode(true)
        setEditingServerId(server.id)
        setNewServer({
            name: server.name,
            server_type: server.server_type,
            image_or_command: server.image_or_command,
            env_vars: server.env_vars || {}
        })
        setIsAddModalOpen(true)
    }

    const handleSaveServer = async () => {
        if (!newServer.name || !newServer.image_or_command) {
            toast.error('Please fill in all required fields')
            return
        }

        try {
            const url = isEditMode && editingServerId
                ? `/v1/mcp/servers/${editingServerId}`
                : '/v1/mcp/servers'

            const method = isEditMode ? 'PATCH' : 'POST'

            const res = await fetch(url, {
                method,
                headers: { 'Content-Type': 'application/json' },
                credentials: 'include',
                body: JSON.stringify(newServer)
            })

            if (res.ok) {
                toast.success(isEditMode ? 'Server updated successfully' : 'MCP Server added successfully')
                setIsAddModalOpen(false)
                loadServers()
            } else {
                const error = await res.text()
                toast.error(`Failed to ${isEditMode ? 'update' : 'add'} server: ${error}`)
            }
        } catch (e) {
            toast.error(`Error ${isEditMode ? 'updating' : 'adding'} server`)
        }
    }

    const handleConnect = async (server: McpServer) => {
        setConnecting(server.id)
        try {
            const res = await fetch(`/v1/mcp/servers/${server.id}/connect`, {
                method: 'POST',
                credentials: 'include'
            })

            if (res.ok) {
                const data = await res.json()
                toast.success(`Connected! Discovered ${data.tools_discovered} tools`)
                loadServers()
                if (selectedServer?.id === server.id) {
                    loadTools(server.id)
                }
            } else {
                const error = await res.text()
                toast.error(`Connection failed: ${error}`)
            }
        } catch (e) {
            toast.error('Error connecting to server')
        } finally {
            setConnecting(null)
        }
    }

    const handleDisconnect = async (server: McpServer) => {
        try {
            const res = await fetch(`/v1/mcp/servers/${server.id}/disconnect`, {
                method: 'POST',
                credentials: 'include'
            })

            if (res.ok) {
                toast.success('Disconnected')
                loadServers()
                setTools([])
            }
        } catch (e) {
            toast.error('Error disconnecting')
        }
    }

    const handleDelete = (server: McpServer) => {
        setServerToDelete(server)
        setShowDeleteModal(true)
    }

    const confirmDelete = async () => {
        if (!serverToDelete) return

        try {
            const res = await fetch(`/v1/mcp/servers/${serverToDelete.id}`, {
                method: 'DELETE',
                credentials: 'include'
            })

            if (res.ok) {
                toast.success('Server deleted')
                loadServers()
                if (selectedServer?.id === serverToDelete.id) {
                    setSelectedServer(null)
                    setTools([])
                }
                setShowDeleteModal(false)
            } else {
                toast.error('Error deleting server')
            }
        } catch (e) {
            toast.error('Error deleting server')
        }
    }

    const addEnvVar = () => {
        if (newEnvKey && newEnvValue) {
            setNewServer({
                ...newServer,
                env_vars: { ...newServer.env_vars, [newEnvKey]: newEnvValue }
            })
            setNewEnvKey('')
            setNewEnvValue('')
        }
    }

    const getStatusConfig = (status: string) => {
        switch (status) {
            case 'connected': return { color: 'emerald', icon: '●', text: 'Connected' }
            case 'connecting': return { color: 'amber', icon: '◐', text: 'Connecting' }
            case 'error': return { color: 'rose', icon: '✕', text: 'Error' }
            default: return { color: 'slate', icon: '○', text: 'Disconnected' }
        }
    }

    const serverTypeConfig = SERVER_TYPE_INFO[newServer.server_type as keyof typeof SERVER_TYPE_INFO]

    const connectedCount = servers.filter(s => s.status === 'connected').length

    return (
        <div className="relative h-screen overflow-y-auto bg-black">
            {/* Topbar — same shape as /providers and /services. */}
            <div className="px-10 pt-10 pb-8 max-w-7xl mx-auto">
                <div className="text-[11px] text-slate-500 mb-3 tracking-[0.12em] uppercase font-semibold">
                    Workspace · Providers · MCP Servers
                </div>

                <div className="flex items-end justify-between gap-4">
                    <div>
                        <h1 className="text-4xl font-bold bg-gradient-to-br from-white to-slate-400 bg-clip-text text-transparent leading-[1.1]">
                            MCP Servers
                        </h1>
                        <p className="text-slate-400 mt-2 text-sm">
                            <span className="font-mono tabular-nums">{servers.length}</span> registered
                            {connectedCount > 0 && (
                                <>
                                    <span className="mx-2 text-slate-700">·</span>
                                    <span className="font-mono tabular-nums text-emerald-300">{connectedCount}</span> connected
                                </>
                            )}
                            {tools.length > 0 && (
                                <>
                                    <span className="mx-2 text-slate-700">·</span>
                                    <span className="font-mono tabular-nums">{tools.length}</span> capabilit{tools.length === 1 ? 'y' : 'ies'} discovered
                                </>
                            )}
                        </p>
                    </div>

                    <button
                        onClick={openAddModal}
                        className="group inline-flex shrink-0 items-center gap-2 px-4 py-2.5 rounded-xl
                                   text-sm font-semibold text-black
                                   bg-gradient-to-br from-cyan-300 to-cyan-600
                                   shadow-[0_0_24px_rgba(34,211,238,0.32)]
                                   hover:shadow-[0_0_32px_rgba(34,211,238,0.5)]
                                   hover:-translate-y-px active:translate-y-0
                                   transition-all"
                    >
                        <Plus className="w-4 h-4" strokeWidth={2.75} />
                        Add Server
                    </button>
                </div>
            </div>

            <div className="px-10 pb-16 max-w-7xl mx-auto">
                {/* Two-column workspace: server list + tools side panel. */}
                <div className="grid grid-cols-1 lg:grid-cols-3 gap-5">
                    {/* Server list */}
                    <div className="lg:col-span-2">
                        <AnimatePresence mode="wait">
                            {loading ? (
                                <motion.div
                                    key="loading"
                                    initial={{ opacity: 0 }}
                                    animate={{ opacity: 1 }}
                                    exit={{ opacity: 0 }}
                                    className="space-y-3"
                                >
                                    <Skeleton className="h-24 rounded-2xl" />
                                    <Skeleton className="h-24 rounded-2xl" />
                                    <Skeleton className="h-24 rounded-2xl" />
                                </motion.div>
                            ) : servers.length === 0 ? (
                                <motion.div
                                    key="empty"
                                    initial={{ opacity: 0, scale: 0.98 }}
                                    animate={{ opacity: 1, scale: 1 }}
                                    exit={{ opacity: 0 }}
                                    className="rounded-2xl border border-white/10 bg-gradient-to-b from-[#0a0a0a] to-[#050505] py-16 px-8 text-center"
                                >
                                    <div className="inline-flex items-center justify-center w-16 h-16 rounded-2xl bg-cyan-400/10 border border-cyan-400/20 mb-5 shadow-[0_0_24px_rgba(34,211,238,0.2)]">
                                        <Link2 className="w-8 h-8 text-cyan-400" strokeWidth={1.75} />
                                    </div>
                                    <h3 className="text-xl font-semibold text-white mb-2">No MCP servers yet</h3>
                                    <p className="text-slate-400 mb-6 max-w-sm mx-auto text-sm leading-relaxed">
                                        Connect external tools and data sources — GitHub, Notion, Slack, Postgres, anything that speaks the Model Context Protocol.
                                    </p>
                                    <div className="flex flex-wrap justify-center gap-2 mb-7">
                                        {['GitHub', 'Notion', 'Slack', 'PostgreSQL'].map((name) => (
                                            <span
                                                key={name}
                                                className="px-3 py-1 rounded-full bg-white/[0.04] border border-white/10 text-[11px] text-slate-400 font-medium"
                                            >
                                                {name}
                                            </span>
                                        ))}
                                    </div>
                                    <button
                                        onClick={openAddModal}
                                        className="group inline-flex items-center gap-2 px-4 py-2.5 rounded-xl
                                                   text-sm font-semibold text-black
                                                   bg-gradient-to-br from-cyan-300 to-cyan-600
                                                   shadow-[0_0_24px_rgba(34,211,238,0.32)]
                                                   hover:shadow-[0_0_32px_rgba(34,211,238,0.5)]
                                                   hover:-translate-y-px transition-all"
                                    >
                                        <Plus className="w-4 h-4" strokeWidth={2.75} />
                                        Add your first server
                                    </button>
                                </motion.div>
                            ) : (
                                <motion.div
                                    key="list"
                                    initial={{ opacity: 0 }}
                                    animate={{ opacity: 1 }}
                                    className="space-y-3"
                                >
                                    {servers.map((server, index) => {
                                        const statusConfig = getStatusConfig(server.status)
                                        const typeInfo = SERVER_TYPE_INFO[server.server_type as keyof typeof SERVER_TYPE_INFO]
                                        const isSelected = selectedServer?.id === server.id
                                        const isConnected = server.status === 'connected'
                                        const isError = server.status === 'error'
                                        const TypeIcon = typeInfo?.icon || Link2

                                        return (
                                            <motion.div
                                                key={server.id}
                                                initial={{ opacity: 0, y: 12 }}
                                                animate={{ opacity: 1, y: 0 }}
                                                transition={{ delay: index * 0.04 }}
                                                onClick={() => {
                                                    setSelectedServer(server)
                                                    if (isConnected) {
                                                        loadTools(server.id)
                                                    } else {
                                                        setTools([])
                                                    }
                                                }}
                                                className={`group relative overflow-hidden cursor-pointer rounded-2xl border
                                                            transition-all duration-200
                                                            ${isSelected
                                                                ? 'border-cyan-400/50 bg-gradient-to-br from-cyan-400/[0.06] to-[#080808] shadow-[0_0_0_1px_rgba(34,211,238,0.35),0_0_28px_rgba(34,211,238,0.18)]'
                                                                : 'border-white/10 bg-gradient-to-br from-[#0f0f0f] to-[#080808] hover:-translate-y-0.5 hover:border-cyan-400/40 hover:shadow-[0_8px_24px_rgba(0,0,0,0.4),0_0_24px_rgba(34,211,238,0.15)]'
                                                            }`}
                                            >
                                                <span className="pointer-events-none absolute inset-0 bg-[radial-gradient(180px_110px_at_85%_-10%,rgba(34,211,238,0.12),transparent_70%)] opacity-0 group-hover:opacity-100 transition-opacity" />

                                                <div className="relative p-4 flex items-center gap-4">
                                                    {/* Type icon tile — emerald when live, slate otherwise. */}
                                                    <div
                                                        className={`w-12 h-12 shrink-0 rounded-xl flex items-center justify-center border
                                                                    ${isConnected
                                                                        ? 'bg-emerald-400/10 border-emerald-400/30 shadow-[0_0_18px_rgba(52,211,153,0.18)]'
                                                                        : isError
                                                                            ? 'bg-red-500/10 border-red-500/30'
                                                                            : 'bg-white/[0.04] border-white/10'
                                                                    }`}
                                                    >
                                                        <TypeIcon
                                                            className={`w-5 h-5 ${
                                                                isConnected ? 'text-emerald-300' : isError ? 'text-red-300' : 'text-slate-400'
                                                            }`}
                                                            strokeWidth={1.75}
                                                        />
                                                    </div>

                                                    {/* Identity column. */}
                                                    <div className="flex-1 min-w-0">
                                                        <div className="flex items-center gap-2 flex-wrap">
                                                            <span className="font-semibold text-white text-base truncate">
                                                                {server.name}
                                                            </span>
                                                            <span
                                                                className={`inline-flex items-center gap-1.5 px-2 py-0.5 rounded-full text-[10px] font-semibold uppercase tracking-wider border
                                                                            ${isConnected
                                                                                ? 'bg-emerald-400/10 text-emerald-300 border-emerald-400/30'
                                                                                : isError
                                                                                    ? 'bg-red-500/10 text-red-300 border-red-500/30'
                                                                                    : 'bg-white/[0.04] text-slate-400 border-white/10'
                                                                            }`}
                                                            >
                                                                <span className={server.status === 'connecting' ? 'animate-spin' : ''}>
                                                                    {statusConfig.icon}
                                                                </span>
                                                                {statusConfig.text}
                                                            </span>
                                                        </div>
                                                        <div className="flex items-center gap-2 mt-1 text-[11px] text-slate-500 min-w-0">
                                                            <span className="text-cyan-300/80 font-semibold uppercase tracking-wider text-[10px]">
                                                                {typeInfo?.label}
                                                            </span>
                                                            <span className="text-slate-700">·</span>
                                                            <code className="font-mono truncate text-slate-500">
                                                                {server.image_or_command}
                                                            </code>
                                                        </div>
                                                        {server.error_message && (
                                                            <div className="mt-2 text-[11px] text-red-300/90 flex items-start gap-1.5 bg-red-500/[0.04] border border-red-500/15 rounded-lg px-2.5 py-1.5 max-h-24 overflow-y-auto">
                                                                <span className="shrink-0 mt-px text-red-400">⚠</span>
                                                                <span className="whitespace-pre-wrap break-words font-mono">
                                                                    {server.error_message}
                                                                </span>
                                                            </div>
                                                        )}
                                                    </div>

                                                    {/* Action buttons — surface on hover, prevent row click. */}
                                                    <div
                                                        className="flex items-center gap-1.5 shrink-0 opacity-0 group-hover:opacity-100 transition-opacity"
                                                        onClick={e => e.stopPropagation()}
                                                    >
                                                        {isConnected ? (
                                                            <button
                                                                onClick={() => handleDisconnect(server)}
                                                                className="px-3 py-1.5 rounded-lg text-[12px] font-medium text-slate-300 hover:text-white bg-white/[0.04] hover:bg-white/[0.08] border border-white/10 transition-colors"
                                                            >
                                                                Disconnect
                                                            </button>
                                                        ) : (
                                                            <button
                                                                onClick={() => handleConnect(server)}
                                                                disabled={connecting === server.id}
                                                                className="px-3 py-1.5 rounded-lg text-[12px] font-semibold text-cyan-200 hover:text-white bg-cyan-400/10 hover:bg-cyan-400/15 border border-cyan-400/30 hover:border-cyan-400/50 disabled:opacity-50 transition-colors min-w-[88px] inline-flex items-center justify-center gap-1.5"
                                                            >
                                                                {connecting === server.id ? (
                                                                    <>
                                                                        <span className="animate-spin">◐</span>
                                                                        Connecting
                                                                    </>
                                                                ) : (
                                                                    'Connect'
                                                                )}
                                                            </button>
                                                        )}
                                                        <button
                                                            onClick={() => openEditModal(server)}
                                                            className="px-3 py-1.5 rounded-lg text-[12px] font-medium text-slate-300 hover:text-white bg-white/[0.02] hover:bg-white/[0.06] border border-white/10 transition-colors"
                                                        >
                                                            Edit
                                                        </button>
                                                        <button
                                                            onClick={() => handleDelete(server)}
                                                            className="px-3 py-1.5 rounded-lg text-[12px] font-medium text-red-400 hover:text-red-300 bg-red-500/[0.04] hover:bg-red-500/10 border border-red-500/15 hover:border-red-500/30 transition-colors"
                                                        >
                                                            Delete
                                                        </button>
                                                    </div>
                                                </div>
                                            </motion.div>
                                        )
                                    })}
                                </motion.div>
                            )}
                        </AnimatePresence>
                    </div>

                    {/* Tools side panel — sticky, dark gradient, premium accents. */}
                    <div>
                        <div className="sticky top-8 overflow-hidden h-[calc(100vh-8rem)] flex flex-col rounded-2xl border border-white/10 bg-gradient-to-br from-[#0f0f0f] to-[#080808]">
                            <div className="px-4 py-3 border-b border-white/10 bg-white/[0.015] shrink-0 flex items-center gap-2">
                                <div className="w-7 h-7 rounded-lg bg-cyan-400/10 border border-cyan-400/25 flex items-center justify-center shrink-0">
                                    <Wrench className="w-3.5 h-3.5 text-cyan-300" strokeWidth={2} />
                                </div>
                                <h3 className="font-semibold text-white text-[13px] tracking-tight flex-1">
                                    Discovered capabilities
                                </h3>
                                {selectedServer && selectedServer.status === 'connected' && tools.length > 0 && (
                                    <span className="font-mono tabular-nums text-[11px] font-semibold px-2 py-0.5 rounded-md bg-cyan-400/10 text-cyan-200 border border-cyan-400/25">
                                        {tools.length}
                                    </span>
                                )}
                            </div>
                            <div className="flex-1 overflow-y-auto p-2 custom-scrollbar">
                                {selectedServer ? (
                                    selectedServer.status === 'connected' ? (
                                        tools.length > 0 ? (
                                            <div className="space-y-2">
                                                {tools.map((tool, i) => (
                                                    <motion.div
                                                        key={tool.id}
                                                        initial={{ opacity: 0, y: 10 }}
                                                        animate={{ opacity: 1, y: 0 }}
                                                        transition={{ delay: i * 0.05 }}
                                                        className="group"
                                                    >
                                                        <div className="p-3 rounded-lg bg-white/[0.02] border border-white/5 hover:border-cyan-500/30 hover:bg-white/[0.04] transition-all">
                                                            {/* Tool Header */}
                                                            <div className="flex items-start justify-between mb-2">
                                                                <div className="flex items-center gap-2">
                                                                    <div className="p-1.5 rounded-lg bg-cyan-900/30 text-cyan-400">
                                                                        <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                                                                            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10 20l4-16m4 4l4 4-4 4M6 16l-4-4 4-4" />
                                                                        </svg>
                                                                    </div>
                                                                    <h4 className="font-bold text-white font-mono text-sm tracking-tight">{tool.name}</h4>
                                                                </div>
                                                            </div>

                                                            {/* Description */}
                                                            {tool.description && (
                                                                <p className="text-xs text-slate-400 mb-3 leading-relaxed pl-1">
                                                                    {tool.description.split(/(\*\*[^*]+\*\*)/).map((part, idx) => {
                                                                        if (part.startsWith('**') && part.endsWith('**')) {
                                                                            return <strong key={idx} className="text-slate-300 font-semibold">{part.slice(2, -2)}</strong>
                                                                        }
                                                                        // Truncate long parts
                                                                        return part.length > 120 ? part.slice(0, 120) + '...' : part
                                                                    })}
                                                                </p>
                                                            )}

                                                            {/* Parameters Schema */}
                                                            {tool.input_schema && (tool.input_schema as any).properties && Object.keys((tool.input_schema as any).properties).length > 0 && (
                                                                <div className="mt-3 bg-black/30 rounded-lg overflow-hidden border border-white/5">
                                                                    <div className="px-3 py-1.5 border-b border-white/5 bg-white/[0.02] text-[10px] uppercase font-bold text-slate-500 tracking-wider">
                                                                        Parameters
                                                                    </div>
                                                                    <div className="divide-y divide-white/5">
                                                                        {Object.entries((tool.input_schema as any).properties).map(([key, prop]: [string, any]) => {
                                                                            const isRequired = (tool.input_schema as any).required?.includes(key)
                                                                            return (
                                                                                <div key={key} className="px-3 py-2.5 text-xs hover:bg-white/[0.02] transition-colors">
                                                                                    <div className="flex items-baseline gap-2 mb-1">
                                                                                        <span className="font-mono text-cyan-300 font-semibold">{key}</span>
                                                                                        {isRequired && (
                                                                                            <span className="px-1 py-0.5 rounded-[4px] bg-red-500/20 text-red-300 text-[9px] font-bold uppercase tracking-wide">
                                                                                                Required
                                                                                            </span>
                                                                                        )}
                                                                                        <span className="text-slate-500 font-mono text-[10px]">
                                                                                            {prop.type}
                                                                                        </span>
                                                                                    </div>
                                                                                    {prop.description && (
                                                                                        <div className="text-slate-400 pl-4 border-l-2 border-white/5 text-[11px] leading-relaxed">
                                                                                            {prop.description}
                                                                                        </div>
                                                                                    )}
                                                                                </div>
                                                                            )
                                                                        })}
                                                                    </div>
                                                                </div>
                                                            )}
                                                        </div>
                                                    </motion.div>
                                                ))}
                                            </div>
                                        ) : (
                                            <div className="text-center py-12">
                                                <div className="text-3xl mb-2 opacity-40">📦</div>
                                                <div className="text-slate-500 text-sm">No tools</div>
                                            </div>
                                        )
                                    ) : (
                                        <div className="h-full flex flex-col items-center justify-center text-center p-4">
                                            <div className="text-3xl mb-3">⚡️</div>
                                            <p className="text-slate-400 text-xs mb-4">Connect to discover tools</p>
                                            <Button
                                                variant="primary"
                                                size="sm"
                                                onClick={() => handleConnect(selectedServer)}
                                                disabled={connecting === selectedServer.id}
                                            >
                                                {connecting === selectedServer.id ? 'Connecting...' : 'Connect'}
                                            </Button>
                                        </div>
                                    )
                                ) : (
                                    <div className="h-full flex flex-col items-center justify-center text-center p-6 opacity-60">
                                        <div className="w-10 h-10 rounded-xl bg-white/[0.04] border border-white/10 flex items-center justify-center mb-3">
                                            <Sparkles className="w-4 h-4 text-slate-500" strokeWidth={1.75} />
                                        </div>
                                        <div className="text-slate-500 text-[11px] font-medium tracking-wide">
                                            Select a server to inspect its tools
                                        </div>
                                    </div>
                                )}
                            </div>
                        </div>
                    </div>
                </div>

                {/* Add/Edit Server Modal — uses shared <Modal> primitive
                    so it inherits the cyan accent bar, focus trap, ESC,
                    aria-modal, and the same gloss as every other dialog. */}
                <Modal
                    isOpen={isAddModalOpen}
                    onClose={() => setIsAddModalOpen(false)}
                    title={isEditMode ? 'Edit MCP Server' : 'Add MCP Server'}
                    description={isEditMode ? 'Update server configuration' : 'Connect an external tool or data source via MCP'}
                    size="md"
                    footer={
                        <>
                            <Button variant="ghost" onClick={() => setIsAddModalOpen(false)}>
                                Cancel
                            </Button>
                            <Button
                                variant="primary"
                                onClick={handleSaveServer}
                                disabled={!newServer.name || !newServer.image_or_command}
                            >
                                {isEditMode ? 'Update Server' : 'Add Server'}
                            </Button>
                        </>
                    }
                >
                    <div className="space-y-5">
                        {/* Server Name */}
                        <Input
                            label="Server Name"
                            placeholder="e.g., GitHub MCP, Notion API"
                            value={newServer.name}
                            onChange={e => setNewServer({ ...newServer, name: e.target.value })}
                        />

                        {/* Connection Type — Lucide-iconed radio cards */}
                        <div>
                            <label className="text-sm font-medium text-slate-300 block mb-2">
                                Connection Type
                            </label>
                            <div className="grid grid-cols-3 gap-2">
                                {Object.entries(SERVER_TYPE_INFO).map(([key, info]) => {
                                    const TypeIcon = info.icon
                                    const active = newServer.server_type === key
                                    return (
                                        <button
                                            key={key}
                                            type="button"
                                            onClick={() => setNewServer({ ...newServer, server_type: key, image_or_command: '' })}
                                            className={`p-3 rounded-xl border text-center transition-all ${
                                                active
                                                    ? 'bg-cyan-400/10 border-cyan-400/50 text-white shadow-[0_0_16px_rgba(34,211,238,0.18)]'
                                                    : 'bg-white/[0.04] border-white/10 text-slate-400 hover:border-white/20 hover:bg-white/[0.07]'
                                            }`}
                                        >
                                            <TypeIcon
                                                className={`w-6 h-6 mx-auto mb-1.5 ${active ? 'text-cyan-300' : 'text-slate-400'}`}
                                                strokeWidth={1.75}
                                            />
                                            <div className="text-xs font-semibold">{info.label}</div>
                                        </button>
                                    )
                                })}
                            </div>
                            <p className="text-xs text-slate-500 mt-2">{serverTypeConfig?.description}</p>
                        </div>

                        {/* Image / Command / URL */}
                        <Input
                            label={
                                newServer.server_type === 'docker' ? 'Docker Image' :
                                newServer.server_type === 'sse' ? 'SSE Endpoint URL' : 'Command Path'
                            }
                            placeholder={serverTypeConfig?.placeholder}
                            value={newServer.image_or_command}
                            onChange={e => setNewServer({ ...newServer, image_or_command: e.target.value })}
                            className="font-mono text-sm"
                        />

                        {/* Environment Variables */}
                        <div>
                            <label className="text-sm font-medium text-slate-300 block mb-2">
                                Environment Variables
                                <span className="text-slate-500 font-normal ml-2">(optional)</span>
                            </label>
                            <div className="flex gap-2 mb-3">
                                <Input
                                    placeholder="KEY"
                                    value={newEnvKey}
                                    onChange={e => setNewEnvKey(e.target.value.toUpperCase())}
                                    className="flex-1 font-mono text-sm uppercase"
                                />
                                <Input
                                    placeholder="value"
                                    value={newEnvValue}
                                    onChange={e => setNewEnvValue(e.target.value)}
                                    className="flex-1 font-mono text-sm"
                                    type="password"
                                />
                                <Button
                                    variant="secondary"
                                    onClick={addEnvVar}
                                    disabled={!newEnvKey || !newEnvValue}
                                >
                                    Add
                                </Button>
                            </div>
                            {Object.entries(newServer.env_vars || {}).length > 0 && (
                                <div className="space-y-2 p-3 rounded-xl bg-black/30 border border-white/5">
                                    {Object.entries(newServer.env_vars).map(([key, value]) => (
                                        <div key={key} className="flex items-center justify-between">
                                            <div className="flex items-center gap-2 min-w-0">
                                                <span className="font-mono text-sm text-cyan-400 truncate">{key}</span>
                                                <span className="text-slate-600">=</span>
                                                <span className="text-slate-500 text-sm font-mono truncate">{value.length > 20 ? value.substring(0, 15) + '…' : value}</span>
                                            </div>
                                            <button
                                                type="button"
                                                aria-label={`Remove ${key}`}
                                                className="shrink-0 p-1.5 rounded-md text-slate-500 hover:text-red-400 hover:bg-red-500/10 transition-colors"
                                                onClick={() => {
                                                    const { [key]: _, ...rest } = newServer.env_vars
                                                    setNewServer({ ...newServer, env_vars: rest })
                                                }}
                                            >
                                                <Trash2 className="w-3.5 h-3.5" strokeWidth={2} />
                                            </button>
                                        </div>
                                    ))}
                                </div>
                            )}
                        </div>
                    </div>
                </Modal>

                {/* Delete Confirmation Modal */}
                <Modal
                    isOpen={showDeleteModal}
                    onClose={() => setShowDeleteModal(false)}
                    title="Delete MCP Server"
                    description="Are you sure you want to remove this server?"
                >
                    <div>
                        <p className="text-slate-300 mb-6">
                            This will permanently delete the server <strong className="text-white">{serverToDelete?.name}</strong> and remove all its tools.
                        </p>
                        <div className="flex justify-end gap-3">
                            <Button variant="ghost" onClick={() => setShowDeleteModal(false)}>Cancel</Button>
                            <Button variant="danger" onClick={confirmDelete}>Delete Server</Button>
                        </div>
                    </div>
                </Modal>

            </div>
        </div>
    )
}
