'use client'

import { useEffect, useState } from 'react'
import { motion, AnimatePresence } from 'framer-motion'
import { Card, Badge, Button, Input, Modal } from '@/components/ui'
import { toast } from 'sonner'

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

const SERVER_TYPE_INFO = {
    docker: {
        icon: '🐳',
        label: 'Docker Container',
        placeholder: 'ghcr.io/github/github-mcp-server',
        description: 'Run MCP server as a Docker container'
    },
    stdio: {
        icon: '💻',
        label: 'Local Process',
        placeholder: '/usr/local/bin/mcp-server',
        description: 'Run a local executable via stdio'
    },
    sse: {
        icon: '🌐',
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

    return (
        <div className="p-8 min-h-screen">
            <div className="max-w-7xl mx-auto space-y-8">
                {/* Header */}
                <motion.div
                    initial={{ opacity: 0, y: 20 }}
                    animate={{ opacity: 1, y: 0 }}
                    className="flex items-center justify-between"
                >
                    <div>
                        <div className="flex items-center gap-3 mb-2">
                            <div className="w-10 h-10 rounded-xl bg-gradient-to-br from-cyan-500 to-blue-600 flex items-center justify-center text-xl shadow-lg shadow-cyan-500/25">
                                🖇
                            </div>
                            <h1 className="text-3xl font-bold gradient-text-white">MCP Servers</h1>
                        </div>
                        <p className="text-slate-400">
                            Connect external tools and data sources via Model Context Protocol
                        </p>
                    </div>
                    <Button
                        variant="primary"
                        onClick={openAddModal}
                        className="shadow-lg shadow-cyan-500/20"
                    >
                        <span className="mr-2">+</span> Add Server
                    </Button>
                </motion.div>

                {/* Stats Bar */}
                {servers.length > 0 && (
                    <motion.div
                        initial={{ opacity: 0, y: 10 }}
                        animate={{ opacity: 1, y: 0 }}
                        transition={{ delay: 0.1 }}
                        className="grid grid-cols-3 gap-4"
                    >
                        <Card className="p-4 bg-gradient-to-br from-slate-900/50 to-slate-800/30">
                            <div className="text-2xl font-bold text-white">{servers.length}</div>
                            <div className="text-sm text-slate-400">Total Servers</div>
                        </Card>
                        <Card className="p-4 bg-gradient-to-br from-emerald-900/20 to-emerald-800/10 border-emerald-500/20">
                            <div className="text-2xl font-bold text-emerald-400">
                                {servers.filter(s => s.status === 'connected').length}
                            </div>
                            <div className="text-sm text-slate-400">Connected</div>
                        </Card>
                        <Card className="p-4 bg-gradient-to-br from-cyan-900/20 to-cyan-800/10 border-cyan-500/20">
                            <div className="text-2xl font-bold text-cyan-400">{tools.length}</div>
                            <div className="text-sm text-slate-400">Available Capabilities</div>
                        </Card>
                    </motion.div>
                )}

                {/* Main Content */}
                <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
                    {/* Server List */}
                    <div className="lg:col-span-2 space-y-4">
                        <AnimatePresence mode="wait">
                            {loading ? (
                                <motion.div
                                    key="loading"
                                    initial={{ opacity: 0 }}
                                    animate={{ opacity: 1 }}
                                    exit={{ opacity: 0 }}
                                >
                                    <Card className="p-12 text-center">
                                        <div className="animate-pulse">
                                            <div className="w-16 h-16 mx-auto rounded-2xl bg-gradient-to-br from-cyan-500/20 to-blue-500/20 mb-4" />
                                            <div className="text-slate-400">Loading servers...</div>
                                        </div>
                                    </Card>
                                </motion.div>
                            ) : servers.length === 0 ? (
                                <motion.div
                                    key="empty"
                                    initial={{ opacity: 0, scale: 0.95 }}
                                    animate={{ opacity: 1, scale: 1 }}
                                    exit={{ opacity: 0, scale: 0.95 }}
                                >
                                    <Card className="relative overflow-hidden">
                                        {/* Decorative Background */}
                                        <div className="absolute inset-0 bg-gradient-to-br from-cyan-500/5 via-transparent to-blue-500/5" />
                                        <div className="absolute top-0 right-0 w-64 h-64 bg-cyan-500/10 blur-[100px] rounded-full" />
                                        <div className="absolute bottom-0 left-0 w-48 h-48 bg-blue-500/10 blur-[80px] rounded-full" />

                                        <div className="relative z-10 p-12 text-center">
                                            <motion.div
                                                initial={{ scale: 0.8, opacity: 0 }}
                                                animate={{ scale: 1, opacity: 1 }}
                                                transition={{ delay: 0.2 }}
                                                className="w-24 h-24 mx-auto rounded-3xl bg-gradient-to-br from-cyan-500/20 to-blue-600/20 flex items-center justify-center text-5xl mb-6 border border-cyan-500/30 shadow-xl shadow-cyan-500/10"
                                            >
                                                🖇
                                            </motion.div>

                                            <h3 className="text-2xl font-bold text-white mb-3">
                                                No MCP Servers Connected
                                            </h3>
                                            <p className="text-slate-400 max-w-md mx-auto mb-8 leading-relaxed">
                                                Connect your first MCP server to unlock powerful external tools.
                                                Integrate with GitHub, Notion, databases, and more.
                                            </p>

                                            <div className="flex flex-wrap justify-center gap-3 mb-8">
                                                {['GitHub', 'Notion', 'Slack', 'PostgreSQL'].map((name, i) => (
                                                    <motion.div
                                                        key={name}
                                                        initial={{ opacity: 0, y: 10 }}
                                                        animate={{ opacity: 1, y: 0 }}
                                                        transition={{ delay: 0.3 + i * 0.1 }}
                                                        className="px-4 py-2 rounded-full bg-white/5 border border-white/10 text-sm text-slate-400"
                                                    >
                                                        {name}
                                                    </motion.div>
                                                ))}
                                            </div>

                                            <Button
                                                variant="primary"
                                                onClick={openAddModal}
                                                className="px-8 py-3 shadow-lg shadow-cyan-500/30"
                                            >
                                                <span className="mr-2">+</span> Add Your First Server
                                            </Button>
                                        </div>
                                    </Card>
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

                                        return (
                                            <motion.div
                                                key={server.id}
                                                initial={{ opacity: 0, x: -20 }}
                                                animate={{ opacity: 1, x: 0 }}
                                                transition={{ delay: index * 0.05 }}
                                            >
                                                <Card
                                                    className={`group p-5 cursor-pointer transition-all duration-300 hover:shadow-lg ${selectedServer?.id === server.id
                                                        ? 'border-cyan-500/50 bg-cyan-500/5 shadow-lg shadow-cyan-500/10'
                                                        : 'hover:border-white/20 hover:bg-white/[0.02]'
                                                        }`}
                                                    onClick={() => {
                                                        setSelectedServer(server)
                                                        if (server.status === 'connected') {
                                                            loadTools(server.id)
                                                        } else {
                                                            setTools([])
                                                        }
                                                    }}
                                                >
                                                    <div className="flex items-center justify-between">
                                                        <div className="flex items-center gap-4">
                                                            <div className={`w-14 h-14 rounded-2xl flex items-center justify-center text-2xl border transition-all duration-300 ${server.status === 'connected'
                                                                ? 'bg-gradient-to-br from-emerald-500/20 to-emerald-600/10 border-emerald-500/30'
                                                                : 'bg-gradient-to-br from-slate-700/50 to-slate-800/30 border-white/10 group-hover:border-white/20'
                                                                }`}>
                                                                {typeInfo?.icon || '🔗'}
                                                            </div>
                                                            <div>
                                                                <div className="flex items-center gap-3">
                                                                    <span className="font-bold text-white text-lg">{server.name}</span>
                                                                    <span
                                                                        className={`flex items-center gap-1.5 px-2.5 py-1 rounded-full text-xs font-medium ${server.status === 'connected'
                                                                            ? 'bg-emerald-500/20 text-emerald-400 border border-emerald-500/30'
                                                                            : server.status === 'error'
                                                                                ? 'bg-red-500/20 text-red-400 border border-red-500/30'
                                                                                : 'bg-slate-700/50 text-slate-400 border border-white/10'
                                                                            }`}
                                                                    >
                                                                        <span className={server.status === 'connecting' ? 'animate-spin' : ''}>
                                                                            {statusConfig.icon}
                                                                        </span>
                                                                        {statusConfig.text}
                                                                    </span>
                                                                </div>
                                                                <div className="flex items-center gap-2 mt-1.5">
                                                                    <span className="text-xs text-slate-500">{typeInfo?.label}</span>
                                                                    <span className="text-slate-600">•</span>
                                                                    <code className="text-xs text-slate-500 font-mono truncate max-w-[300px]">
                                                                        {server.image_or_command}
                                                                    </code>
                                                                </div>
                                                                {server.error_message && (
                                                                    <div className="text-xs text-red-400 mt-2 flex items-start gap-1 max-h-32 overflow-y-auto scrollbar-thin scrollbar-thumb-red-500/20 scrollbar-track-transparent pr-2">
                                                                        <span className="shrink-0 mt-0.5">⚠</span>
                                                                        <span className="whitespace-pre-wrap break-words">{server.error_message}</span>
                                                                    </div>
                                                                )}
                                                            </div>
                                                        </div>
                                                        <div className="flex items-center gap-2 opacity-0 group-hover:opacity-100 transition-opacity" onClick={e => e.stopPropagation()}>
                                                            {server.status === 'connected' ? (
                                                                <Button
                                                                    variant="ghost"
                                                                    size="sm"
                                                                    onClick={() => handleDisconnect(server)}
                                                                    className="text-slate-400 hover:text-white"
                                                                >
                                                                    Disconnect
                                                                </Button>
                                                            ) : (
                                                                <Button
                                                                    variant="secondary"
                                                                    size="sm"
                                                                    onClick={() => handleConnect(server)}
                                                                    disabled={connecting === server.id}
                                                                    className="min-w-[100px]"
                                                                >
                                                                    {connecting === server.id ? (
                                                                        <span className="flex items-center gap-2">
                                                                            <span className="animate-spin">◐</span> Connecting
                                                                        </span>
                                                                    ) : 'Connect'}
                                                                </Button>
                                                            )}
                                                            <Button
                                                                variant="ghost"
                                                                size="sm"
                                                                className="text-cyan-400 hover:bg-cyan-500/10 hover:text-cyan-300"
                                                                onClick={() => openEditModal(server)}
                                                            >
                                                                Edit
                                                            </Button>
                                                            <Button
                                                                variant="ghost"
                                                                size="sm"
                                                                className="text-red-400 hover:bg-red-500/10 hover:text-red-300"
                                                                onClick={() => handleDelete(server)}
                                                            >
                                                                Delete
                                                            </Button>
                                                        </div>
                                                    </div>
                                                </Card>
                                            </motion.div>
                                        )
                                    })}
                                </motion.div>
                            )}
                        </AnimatePresence>
                    </div>

                    {/* Tools Panel */}
                    <div>
                        <Card className="sticky top-8 overflow-hidden h-[calc(100vh-8rem)] flex flex-col">
                            <div className="px-3 py-2 border-b border-white/10 bg-gradient-to-r from-cyan-500/5 to-transparent shrink-0">
                                <h3 className="font-semibold text-white flex items-center gap-2 text-sm">
                                    <span className="text-cyan-400">🔧</span>
                                    Discovered Capabilities
                                    {selectedServer && selectedServer.status === 'connected' && tools.length > 0 && (
                                        <span className="ml-auto">
                                            <Badge variant="cyan" size="sm">
                                                {tools.length}
                                            </Badge>
                                        </span>
                                    )}
                                </h3>
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
                                    <div className="h-full flex flex-col items-center justify-center text-center p-4 opacity-50">
                                        <div className="text-3xl mb-2">👈</div>
                                        <div className="text-slate-500 text-xs">Select a server</div>
                                    </div>
                                )}
                            </div>
                        </Card>
                    </div>
                </div>

                {/* Add/Edit Server Modal */}
                <AnimatePresence>
                    {isAddModalOpen && (
                        <motion.div
                            initial={{ opacity: 0 }}
                            animate={{ opacity: 1 }}
                            exit={{ opacity: 0 }}
                            className="fixed inset-0 z-50 flex items-center justify-center bg-black/80 backdrop-blur-md"
                            onClick={() => setIsAddModalOpen(false)}
                        >
                            <motion.div
                                initial={{ scale: 0.9, opacity: 0, y: 20 }}
                                animate={{ scale: 1, opacity: 1, y: 0 }}
                                exit={{ scale: 0.9, opacity: 0, y: 20 }}
                                transition={{ type: 'spring', damping: 25, stiffness: 300 }}
                                onClick={e => e.stopPropagation()}
                                className="w-full max-w-lg mx-4"
                            >
                                <Card className="overflow-hidden">
                                    {/* Modal Header */}
                                    <div className="p-6 border-b border-white/10 bg-gradient-to-r from-cyan-500/10 to-blue-500/5">
                                        <h2 className="text-xl font-bold text-white flex items-center gap-3">
                                            <span className="w-10 h-10 rounded-xl bg-gradient-to-br from-cyan-500 to-blue-600 flex items-center justify-center text-lg shadow-lg shadow-cyan-500/25">
                                                🖇
                                            </span>
                                            {isEditMode ? 'Edit MCP Server' : 'Add MCP Server'}
                                        </h2>
                                        <p className="text-slate-400 text-sm mt-2">
                                            {isEditMode ? 'Update server configuration' : 'Connect an external tool or data source via MCP'}
                                        </p>
                                    </div>

                                    <div className="p-6 space-y-5">
                                        {/* Server Name */}
                                        <div>
                                            <label className="text-sm font-medium text-slate-300 block mb-2">Server Name</label>
                                            <Input
                                                placeholder="e.g., GitHub MCP, Notion API"
                                                value={newServer.name}
                                                onChange={e => setNewServer({ ...newServer, name: e.target.value })}
                                            />
                                        </div>

                                        {/* Server Type */}
                                        <div>
                                            <label className="text-sm font-medium text-slate-300 block mb-2">Connection Type</label>
                                            <div className="grid grid-cols-3 gap-2">
                                                {Object.entries(SERVER_TYPE_INFO).map(([key, info]) => (
                                                    <button
                                                        key={key}
                                                        onClick={() => setNewServer({ ...newServer, server_type: key, image_or_command: '' })}
                                                        className={`p-3 rounded-xl border text-center transition-all ${newServer.server_type === key
                                                            ? 'bg-cyan-500/10 border-cyan-500/50 text-white'
                                                            : 'bg-white/5 border-white/10 text-slate-400 hover:border-white/20'
                                                            }`}
                                                    >
                                                        <div className="text-2xl mb-1">{info.icon}</div>
                                                        <div className="text-xs font-medium">{info.label}</div>
                                                    </button>
                                                ))}
                                            </div>
                                            <p className="text-xs text-slate-500 mt-2">{serverTypeConfig?.description}</p>
                                        </div>

                                        {/* Image/Command/URL */}
                                        <div>
                                            <label className="text-sm font-medium text-slate-300 block mb-2">
                                                {newServer.server_type === 'docker' ? 'Docker Image' :
                                                    newServer.server_type === 'sse' ? 'SSE Endpoint URL' : 'Command Path'}
                                            </label>
                                            <Input
                                                placeholder={serverTypeConfig?.placeholder}
                                                value={newServer.image_or_command}
                                                onChange={e => setNewServer({ ...newServer, image_or_command: e.target.value })}
                                                className="font-mono text-sm"
                                            />
                                        </div>

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
                                                            <div className="flex items-center gap-2">
                                                                <span className="font-mono text-sm text-cyan-400">{key}</span>
                                                                <span className="text-slate-600">=</span>
                                                                <span className="text-slate-500 text-sm font-mono">{value.length > 20 ? value.substring(0, 15) + '...' : value}</span>
                                                            </div>
                                                            <button
                                                                className="text-red-400 text-xs hover:text-red-300 transition-colors"
                                                                onClick={() => {
                                                                    const { [key]: _, ...rest } = newServer.env_vars
                                                                    setNewServer({ ...newServer, env_vars: rest })
                                                                }}
                                                            >
                                                                Remove
                                                            </button>
                                                        </div>
                                                    ))}
                                                </div>
                                            )}
                                        </div>
                                    </div>

                                    {/* Modal Footer */}
                                    <div className="flex justify-end gap-3 p-6 border-t border-white/10 bg-black/20">
                                        <Button variant="ghost" onClick={() => setIsAddModalOpen(false)}>
                                            Cancel
                                        </Button>
                                        <Button
                                            variant="primary"
                                            onClick={handleSaveServer}
                                            disabled={!newServer.name || !newServer.image_or_command}
                                            className="min-w-[120px] shadow-lg shadow-cyan-500/20"
                                        >
                                            {isEditMode ? 'Update Server' : 'Add Server'}
                                        </Button>
                                    </div>
                                </Card>
                            </motion.div>
                        </motion.div>
                    )}
                </AnimatePresence>

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
