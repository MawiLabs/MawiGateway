'use client'

import React, { useState, useEffect } from 'react'
import { motion, AnimatePresence } from 'framer-motion'
import { Card, Badge, Button, Input, Select } from '@/components/ui'
import { Search, RefreshCw, FileText, ChevronRight } from 'lucide-react'

interface RequestLog {
    id: string
    service_name: string
    model_id: string
    provider_type: string
    latency_ms: number
    latency_us?: number  //Microseconds for better precision
    status: string
    created_at: string
    tokens_prompt?: number
    tokens_completion?: number
    tokens_total?: number
    cost_usd?: number
    error_message?: string
    failover_count: number
}

export default function LogsPage() {
    const [logs, setLogs] = useState<RequestLog[]>([])
    const [loading, setLoading] = useState(true)
    const [searchQuery, setSearchQuery] = useState('')
    const [statusFilter, setStatusFilter] = useState<'all' | 'success' | 'error'>('all')
    const [liveTail, setLiveTail] = useState(false)
    const [showFilters, setShowFilters] = useState(false)
    const [autoRefresh, setAutoRefresh] = useState(true)
    const [expandedLogId, setExpandedLogId] = useState<string | null>(null)

    useEffect(() => {
        fetchLogs()

        if (autoRefresh || liveTail) {
            const interval = setInterval(fetchLogs, liveTail ? 5000 : 15000)
            return () => clearInterval(interval)
        }
    }, [autoRefresh, liveTail])

    const fetchLogs = async () => {
        try {
            const res = await fetch('/v1/user/logs?limit=100', { credentials: 'include' })
            if (res.ok) {
                setLogs(await res.json())
            }
            setLoading(false)
        } catch (error) {
            console.error('Failed to fetch logs:', error)
            setLoading(false)
        }
    }

    const filteredLogs = logs.filter(log => {
        if (statusFilter !== 'all' && log.status !== statusFilter) return false

        if (searchQuery) {
            const query = searchQuery.toLowerCase()
            return (
                log.id.toLowerCase().includes(query) ||
                log.service_name.toLowerCase().includes(query) ||
                log.model_id.toLowerCase().includes(query)
            )
        }

        return true
    })

    const resetFilters = () => {
        setSearchQuery('')
        setStatusFilter('all')
    }

    // Format latency with microseconds for better precision
    const formatLatency = (log: RequestLog) => {
        // Use microseconds if available
        if (log.latency_us !== undefined && log.latency_us !== null) {
            if (log.latency_us < 1000) {
                return `${log.latency_us}μs`
            }
            const ms = (log.latency_us / 1000).toFixed(2)
            return `${ms}ms`
        }
        // Fallback to latency_ms
        return `${log.latency_ms.toLocaleString()}ms`
    }

    return (
        <div className="relative h-screen overflow-y-auto bg-black">
            <div className="px-10 pt-10 pb-8 max-w-7xl mx-auto">
                <div className="text-[11px] text-slate-500 mb-3 tracking-[0.12em] uppercase font-semibold">
                    Workspace · Logs
                </div>
                <h1 className="text-4xl font-bold bg-gradient-to-br from-white to-slate-400 bg-clip-text text-transparent leading-[1.1]">
                    Request Logs
                </h1>
                <p className="text-slate-400 mt-2 text-sm">
                    Monitor and analyze all gateway requests
                    {logs.length > 0 && (
                        <>
                            <span className="mx-2 text-slate-700">·</span>
                            <span className="font-mono tabular-nums">{logs.length}</span> entr{logs.length === 1 ? 'y' : 'ies'} loaded
                        </>
                    )}
                </p>
            </div>

            <div className="px-10 pb-16 max-w-7xl mx-auto space-y-6">

                {/* Filters Bar */}
                <Card className="p-4">
                    <div className="flex items-center gap-3 flex-wrap">
                        <Button
                            variant="secondary"
                            size="sm"
                            icon={<Search className="w-4 h-4" strokeWidth={2} />}
                            onClick={() => setShowFilters(!showFilters)}>
                            Filters
                        </Button>

                        <Button
                            variant="ghost"
                            size="sm"
                            onClick={resetFilters}>
                            Reset
                        </Button>

                        <div className="h-6 w-px bg-white/10" />

                        {/* Search */}
                        <Input
                            value={searchQuery}
                            onChange={(e) => setSearchQuery(e.target.value)}
                            placeholder="Search by Request ID, Service, or Model"
                            className="flex-1 min-w-[300px]"
                        />

                        {/* Live Tail Toggle */}
                        <label className="flex items-center gap-2 cursor-pointer">
                            <span className="text-sm text-slate-400">Live Tail</span>
                            <div className="relative">
                                <input
                                    type="checkbox"
                                    checked={liveTail}
                                    onChange={(e) => setLiveTail(e.target.checked)}
                                    className="sr-only peer"
                                />
                                <div className="w-11 h-6 bg-[#1a1a1a] peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-cyan-400" />
                            </div>
                        </label>

                        <Button
                            variant="primary"
                            size="sm"
                            icon={<RefreshCw className="w-4 h-4" strokeWidth={2} />}
                            onClick={fetchLogs}>
                            Refresh
                        </Button>

                        <div className="ml-auto text-sm text-slate-400">
                            {filteredLogs.length} results
                        </div>
                    </div>

                    {/* Extended Filters */}
                    <AnimatePresence>
                        {showFilters && (
                            <motion.div
                                initial={{ opacity: 0, height: 0 }}
                                animate={{ opacity: 1, height: 'auto' }}
                                exit={{ opacity: 0, height: 0 }}
                                className="mt-4 pt-4 border-t border-white/10">
                                <Select
                                    label="Status"
                                    value={statusFilter}
                                    onChange={(e) => setStatusFilter(e.target.value as any)}
                                    options={[
                                        { value: 'all', label: 'All statuses' },
                                        { value: 'success', label: 'Success' },
                                        { value: 'error', label: 'Error' },
                                    ]}
                                />
                            </motion.div>
                        )}
                    </AnimatePresence>
                </Card>

                {/* Auto-refresh banner */}
                {(autoRefresh || liveTail) && (
                    <motion.div
                        initial={{ opacity: 0, y: -10 }}
                        animate={{ opacity: 1, y: 0 }}
                        className="glass border border-cyan-400/30 rounded-xl p-3 flex items-center justify-between">
                        <span className="text-sm text-cyan-400 flex items-center gap-2">
                            <RefreshCw className="w-4 h-4 animate-spin" strokeWidth={2} />
                            Auto-refreshing every {liveTail ? '5' : '15'} seconds
                        </span>
                        <Button
                            variant="ghost"
                            size="sm"
                            onClick={() => {
                                setAutoRefresh(false)
                                setLiveTail(false)
                            }}
                            className="text-cyan-400">
                            Stop
                        </Button>
                    </motion.div>
                )}

                {/* Logs Table */}
                <Card className="overflow-hidden">
                    <div className="overflow-x-auto">
                        <table className="w-full">
                            <thead className="bg-[#0f0f0f] border-b border-white/10">
                                <tr>
                                    <th className="px-6 py-3 text-left text-xs font-medium text-slate-400 uppercase tracking-wider w-12" />
                                    <th className="px-6 py-3 text-left text-xs font-medium text-slate-400 uppercase tracking-wider">Time</th>
                                    <th className="px-6 py-3 text-left text-xs font-medium text-slate-400 uppercase tracking-wider">Status</th>
                                    <th className="px-6 py-3 text-left text-xs font-medium text-slate-400 uppercase tracking-wider">Service</th>
                                    <th className="px-6 py-3 text-left text-xs font-medium text-slate-400 uppercase tracking-wider">Request ID</th>
                                    <th className="px-6 py-3 text-left text-xs font-medium text-slate-400 uppercase tracking-wider">Model ID</th>
                                    <th className="px-6 py-3 text-left text-xs font-medium text-slate-400 uppercase tracking-wider">Duration</th>
                                </tr>
                            </thead>
                            <tbody className="divide-y divide-white/10">
                                {loading ? (
                                    <tr>
                                        <td colSpan={7} className="px-6 py-8 text-center text-slate-400">
                                            Loading logs...
                                        </td>
                                    </tr>
                                ) : filteredLogs.length === 0 ? (
                                    <tr>
                                        <td colSpan={7} className="p-0">
                                            <div className="rounded-2xl border border-white/10 bg-gradient-to-b from-[#0a0a0a] to-[#050505] py-16 text-center">
                                                <div className="w-14 h-14 mx-auto mb-4 rounded-2xl bg-cyan-400/10 border border-cyan-400/20 flex items-center justify-center shadow-[0_0_24px_rgba(34,211,238,0.15)]">
                                                    <FileText className="w-7 h-7 text-cyan-400" strokeWidth={2} />
                                                </div>
                                                <div className="text-slate-300 font-medium mb-1">No logs found</div>
                                                <div className="text-slate-500 text-sm mb-5">Logs will appear here as requests flow through the gateway.</div>
                                                <div className="flex justify-center">
                                                    <Button
                                                        variant="secondary"
                                                        size="sm"
                                                        icon={<RefreshCw className="w-4 h-4" strokeWidth={2} />}
                                                        onClick={fetchLogs}>
                                                        Refresh logs
                                                    </Button>
                                                </div>
                                            </div>
                                        </td>
                                    </tr>
                                ) : (
                                    filteredLogs.map((log, i) => (
                                        <React.Fragment key={log.id}>
                                            <motion.tr
                                                initial={{ opacity: 0 }}
                                                animate={{ opacity: 1 }}
                                                transition={{ delay: i * 0.01 }}
                                                className="hover:bg-white/5 transition-colors cursor-pointer"
                                                onClick={() => setExpandedLogId(expandedLogId === log.id ? null : log.id)}>
                                                <td className="px-6 py-4 whitespace-nowrap">
                                                    <button className="text-slate-400 hover:text-white transition-colors">
                                                        <motion.span
                                                            animate={{ rotate: expandedLogId === log.id ? 90 : 0 }}
                                                            className="inline-block">
                                                            <ChevronRight className="w-4 h-4" strokeWidth={2} />
                                                        </motion.span>
                                                    </button>
                                                </td>
                                                <td className="px-6 py-4 whitespace-nowrap text-sm text-slate-300">
                                                    {new Date(log.created_at).toLocaleString()}
                                                </td>
                                                <td className="px-6 py-4 whitespace-nowrap">
                                                    <Badge variant={log.status === 'success' ? 'success' : 'danger'} size="sm">
                                                        {log.status.charAt(0).toUpperCase() + log.status.slice(1)}
                                                    </Badge>
                                                </td>
                                                <td className="px-6 py-4 whitespace-nowrap text-sm text-white font-medium">
                                                    {log.service_name}
                                                </td>
                                                <td className="px-6 py-4 whitespace-nowrap text-sm text-cyan-400 font-mono">
                                                    {log.id.substring(0, 13)}...
                                                </td>
                                                <td className="px-6 py-4 whitespace-nowrap text-sm text-slate-400 font-mono">
                                                    {log.model_id.substring(0, 13)}...
                                                </td>
                                                <td className="px-6 py-4 whitespace-nowrap text-sm text-slate-300">
                                                    {formatLatency(log)}
                                                </td>
                                            </motion.tr>

                                            {/* Expanded Details */}
                                            <AnimatePresence>
                                                {expandedLogId === log.id && (
                                                    <motion.tr
                                                        initial={{ opacity: 0 }}
                                                        animate={{ opacity: 1 }}
                                                        exit={{ opacity: 0 }}>
                                                        <td colSpan={7} className="px-6 py-4 bg-[#0f0f0f]">
                                                            <div className="grid grid-cols-2 gap-6">
                                                                {/* Left Column */}
                                                                <div className="space-y-4">
                                                                    <div>
                                                                        <h4 className="text-xs font-semibold text-slate-400 uppercase mb-2">Request Details</h4>
                                                                        <div className="glass rounded-lg p-3 space-y-2 text-sm">
                                                                            <div className="flex justify-between">
                                                                                <span className="text-slate-500">Request ID:</span>
                                                                                <span className="text-cyan-400 font-mono">{log.id}</span>
                                                                            </div>
                                                                            <div className="flex justify-between">
                                                                                <span className="text-slate-500">Service:</span>
                                                                                <span className="text-white">{log.service_name}</span>
                                                                            </div>
                                                                            <div className="flex justify-between">
                                                                                <span className="text-slate-500">Model ID:</span>
                                                                                <span className="text-slate-400 font-mono">{log.model_id}</span>
                                                                            </div>
                                                                            <div className="flex justify-between">
                                                                                <span className="text-slate-500">Status:</span>
                                                                                <span className={log.status === 'success' ? 'text-emerald-400' : 'text-red-400'}>
                                                                                    {log.status}
                                                                                </span>
                                                                            </div>
                                                                        </div>
                                                                    </div>

                                                                    <div>
                                                                        <h4 className="text-xs font-semibold text-slate-400 uppercase mb-2">Performance</h4>
                                                                        <div className="glass rounded-lg p-3 space-y-2 text-sm">
                                                                            <div className="flex justify-between">
                                                                                <span className="text-slate-500">Latency:</span>
                                                                                <span className="text-white">{formatLatency(log)}</span>
                                                                            </div>
                                                                            <div className="flex justify-between">
                                                                                <span className="text-slate-500">Timestamp:</span>
                                                                                <span className="text-slate-400">{new Date(log.created_at).toISOString()}</span>
                                                                            </div>
                                                                        </div>
                                                                    </div>
                                                                </div>

                                                                {/* Right Column */}
                                                                <div className="space-y-4">
                                                                    <div>
                                                                        <h4 className="text-xs font-semibold text-slate-400 uppercase mb-2">Provider & Model</h4>
                                                                        <div className="glass rounded-lg p-3 space-y-2 text-sm">
                                                                            <div className="flex justify-between">
                                                                                <span className="text-slate-500">Provider:</span>
                                                                                <span className="text-white">{log.provider_type}</span>
                                                                            </div>
                                                                            <div className="flex justify-between">
                                                                                <span className="text-slate-500">Failover Count:</span>
                                                                                <span className={log.failover_count > 0 ? 'text-amber-400' : 'text-slate-400'}>
                                                                                    {log.failover_count}
                                                                                </span>
                                                                            </div>
                                                                        </div>
                                                                    </div>

                                                                    {(log.tokens_prompt || log.tokens_completion || log.cost_usd) && (
                                                                        <div>
                                                                            <h4 className="text-xs font-semibold text-slate-400 uppercase mb-2">Usage & Cost</h4>
                                                                            <div className="glass rounded-lg p-3 space-y-2 text-sm">
                                                                                {log.tokens_prompt && (
                                                                                    <div className="flex justify-between">
                                                                                        <span className="text-slate-500">Prompt Tokens:</span>
                                                                                        <span className="text-white">{log.tokens_prompt.toLocaleString()}</span>
                                                                                    </div>
                                                                                )}
                                                                                {log.tokens_completion && (
                                                                                    <div className="flex justify-between">
                                                                                        <span className="text-slate-500">Completion Tokens:</span>
                                                                                        <span className="text-white">{log.tokens_completion.toLocaleString()}</span>
                                                                                    </div>
                                                                                )}
                                                                                {log.tokens_total && (
                                                                                    <div className="flex justify-between">
                                                                                        <span className="text-slate-500">Total Tokens:</span>
                                                                                        <span className="text-white font-medium">{log.tokens_total.toLocaleString()}</span>
                                                                                    </div>
                                                                                )}
                                                                                {log.cost_usd && (
                                                                                    <div className="flex justify-between">
                                                                                        <span className="text-cyan-400">Cost:</span>
                                                                                        <span className="text-emerald-400 font-medium">${log.cost_usd.toFixed(6)}</span>
                                                                                    </div>
                                                                                )}
                                                                            </div>
                                                                        </div>
                                                                    )}

                                                                    {log.error_message && (
                                                                        <div>
                                                                            <h4 className="text-xs font-semibold text-slate-400 uppercase mb-2">Error Details</h4>
                                                                            <div className="bg-red-500/10 border border-red-500/30 rounded-lg p-3">
                                                                                <p className="text-sm text-red-400 font-mono">{log.error_message}</p>
                                                                            </div>
                                                                        </div>
                                                                    )}
                                                                </div>
                                                            </div>
                                                        </td>
                                                    </motion.tr>
                                                )}
                                            </AnimatePresence>
                                        </React.Fragment>
                                    ))
                                )}
                            </tbody>
                        </table>
                    </div>
                </Card>
            </div>
        </div>
    )
}
