'use client'

import { useEffect, useState } from 'react'
import { motion } from 'framer-motion'
import { Card, Badge, Skeleton } from '@/components/ui'
import {
  LineChart, Line, BarChart, Bar, AreaChart, Area,
  XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer,
  Legend
} from 'recharts'

interface TimeSeriesPoint {
  timestamp: string
  request_count: number
  error_count: number
  avg_latency_ms: number
  total_cost_usd: number
}

interface AnalyticsSummary {
  total_requests: number
  successful_requests: number
  failed_requests: number
  total_cost_usd: number
  avg_latency_ms: number
  p95_latency_ms: number
}

export default function AnalyticsPage() {
  const [summary, setSummary] = useState<AnalyticsSummary | null>(null)
  const [timeSeries, setTimeSeries] = useState<TimeSeriesPoint[]>([])
  const [topModels, setTopModels] = useState<any[]>([])
  const [loading, setLoading] = useState(true)
  const [range, setRange] = useState('30d') // 24h, 7d, 30d

  useEffect(() => {
    loadData()
  }, [range])

  const loadData = async () => {
    setLoading(true)
    try {
      // 1. Fetch Summary
      const resSummary = await fetch('/v1/analytics/summary', { credentials: 'include' })
      if (resSummary.ok) setSummary(await resSummary.json())

      // 2. Fetch TimeSeries
      const resSeries = await fetch(`/v1/analytics/time-series?range=${range}`, { credentials: 'include' })
      if (resSeries.ok) {
        const data = await resSeries.json()
        // Format timestamps for display
        const formatted = data.map((d: any) => ({
          ...d,
          displayTime: range === '24h'
            ? new Date(d.timestamp).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })
            : new Date(d.timestamp).toLocaleDateString([], { month: 'short', day: 'numeric' })
        }))
        setTimeSeries(formatted)
      }

      // 3. Fetch Top Models
      const resModels = await fetch('/v1/analytics/top-models', { credentials: 'include' })
      if (resModels.ok) setTopModels(await resModels.json())

    } catch (error) {
      console.error('Analytics load failed:', error)
    } finally {
      setLoading(false)
    }
  }

  // Custom Tooltip for Charts
  const CustomTooltip = ({ active, payload, label }: any) => {
    if (active && payload && payload.length) {
      return (
        <div className="glass p-3 rounded-lg border border-white/10 shadow-xl">
          <p className="text-slate-300 text-xs mb-2">{label}</p>
          {payload.map((p: any) => (
            <div key={p.name} className="flex items-center gap-2 text-sm">
              <div className="w-2 h-2 rounded-full" style={{ backgroundColor: p.color }} />
              <span className="text-slate-400 capitalize">{p.name.replace(/_/g, ' ')}:</span>
              <span className="text-white font-mono font-bold">
                {p.value.toLocaleString(undefined, { maximumFractionDigits: 4 })}
              </span>
            </div>
          ))}
        </div>
      )
    }
    return null
  }

  const statCards = [
    {
      label: 'Total Requests',
      value: summary?.total_requests?.toLocaleString() || '0',
      icon: '📊',
      glow: 'cyan',
      sub: `${summary?.failed_requests || 0} errors`
    },
    {
      label: 'Total Cost (USD)',
      value: `$${(summary?.total_cost_usd || 0).toFixed(4)}`, // Precise Cost
      icon: '💰',
      glow: 'green',
      sub: 'Estimated spend'
    },
    {
      label: 'Avg Latency',
      value: `${(summary?.avg_latency_ms || 0).toFixed(0)}ms`,
      icon: '⚡',
      glow: 'purple',
      sub: `P95: ${(summary?.p95_latency_ms || 0).toFixed(0)}ms`
    },
    {
      label: 'Success Rate',
      value: summary?.total_requests ? `${((summary.successful_requests / summary.total_requests) * 100).toFixed(1)}%` : '100%',
      icon: '✅',
      glow: 'cyan',
      sub: 'Reliability'
    }
  ]

  return (
    <div className="p-8 max-w-[1600px] mx-auto space-y-8">

      {/* Header */}
      <div className="flex items-center justify-between">
        <motion.div initial={{ opacity: 0, x: -20 }} animate={{ opacity: 1, x: 0 }}>
          <h1 className="text-3xl font-bold gradient-text-white mb-1">Analytics Dashboard</h1>
          <p className="text-slate-400">Real-time performance and financial insights</p>
        </motion.div>

        {/* Range Selector */}
        <div className="flex bg-[#1a1a1a] p-1 rounded-lg border border-white/5">
          {['24h', '7d', '30d'].map((r) => (
            <button
              key={r}
              onClick={() => setRange(r)}
              className={`px-4 py-1.5 rounded-md text-sm font-medium transition-all ${range === r
                ? 'bg-gradient-to-r from-cyan-500/20 to-blue-500/20 text-cyan-400 border border-cyan-500/30 shadow-lg shadow-cyan-900/20'
                : 'text-slate-400 hover:text-white hover:bg-white/5'
                }`}
            >
              Last {r}
            </button>
          ))}
        </div>
      </div>

      {/* KPI Cards */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
        {statCards.map((stat, i) => (
          <motion.div
            key={i}
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ delay: i * 0.05 }}
          >
            <Card hover glow={stat.glow as any} className="relative overflow-hidden">
              <div className="flex justify-between items-start mb-2">
                <div className="text-3xl p-2 bg-white/5 rounded-xl">{stat.icon}</div>
                {i === 1 && <Badge variant="success" size="sm">Precise</Badge>}
              </div>
              <div className="mt-2">
                <div className="text-3xl font-bold text-white tracking-tight">{stat.value}</div>
                <div className="text-slate-400 text-sm font-medium">{stat.label}</div>
                <div className="text-slate-600 text-xs mt-1 border-t border-white/5 pt-2">{stat.sub}</div>
              </div>
            </Card>
          </motion.div>
        ))}
      </div>

      {/* Charts Row 1: Requests & Latency */}
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6 h-[400px]">
        {/* Main Traffic Chart */}
        <Card className="col-span-2 flex flex-col">
          <h3 className="text-lg font-bold text-white mb-6 flex items-center gap-2">
            <span className="w-2 h-2 rounded-full bg-cyan-400" />
            Request Volume
          </h3>
          <div className="flex-1 min-h-0">
            <ResponsiveContainer width="100%" height="100%">
              <AreaChart data={timeSeries}>
                <defs>
                  <linearGradient id="colorRequests" x1="0" y1="0" x2="0" y2="1">
                    <stop offset="5%" stopColor="#22d3ee" stopOpacity={0.3} />
                    <stop offset="95%" stopColor="#22d3ee" stopOpacity={0} />
                  </linearGradient>
                </defs>
                <CartesianGrid strokeDasharray="3 3" stroke="#ffffff10" vertical={false} />
                <XAxis
                  dataKey="displayTime"
                  stroke="#666"
                  fontSize={12}
                  tickLine={false}
                  axisLine={false}
                />
                <YAxis
                  stroke="#666"
                  fontSize={12}
                  tickLine={false}
                  axisLine={false}
                  tickFormatter={(value) => value >= 1000 ? `${value / 1000}k` : value}
                />
                <Tooltip content={<CustomTooltip />} cursor={{ stroke: '#ffffff20' }} />
                <Area
                  type="monotone"
                  dataKey="request_count"
                  name="Requests"
                  stroke="#22d3ee"
                  strokeWidth={2}
                  fillOpacity={1}
                  fill="url(#colorRequests)"
                />
                <Area
                  type="monotone"
                  dataKey="error_count"
                  name="Errors"
                  stroke="#ef4444"
                  strokeWidth={2}
                  fill="transparent"
                />
              </AreaChart>
            </ResponsiveContainer>
          </div>
        </Card>

        {/* Latency Distribution */}
        <Card className="flex flex-col">
          <h3 className="text-lg font-bold text-white mb-6 flex items-center gap-2">
            <span className="w-2 h-2 rounded-full bg-purple-400" />
            Latency (ms)
          </h3>
          <div className="flex-1 min-h-0">
            <ResponsiveContainer width="100%" height="100%">
              <LineChart data={timeSeries}>
                <CartesianGrid strokeDasharray="3 3" stroke="#ffffff10" vertical={false} />
                <XAxis dataKey="displayTime" hide />
                <YAxis stroke="#666" fontSize={12} tickLine={false} axisLine={false} />
                <Tooltip content={<CustomTooltip />} />
                <Line
                  type="monotone"
                  dataKey="avg_latency_ms"
                  name="Avg Latency"
                  stroke="#a855f7"
                  strokeWidth={2}
                  dot={false}
                />
              </LineChart>
            </ResponsiveContainer>
          </div>
        </Card>
      </div>

      {/* Row 2: Cost & Top Models */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">

        {/* Cost Analysis */}
        <Card>
          <h3 className="text-lg font-bold text-white mb-6 flex items-center gap-2">
            <span className="w-2 h-2 rounded-full bg-green-400" />
            Cost Accumulation (USD)
          </h3>
          <div className="h-[300px]">
            <ResponsiveContainer width="100%" height="100%">
              <BarChart data={timeSeries}>
                <CartesianGrid strokeDasharray="3 3" stroke="#ffffff10" vertical={false} />
                <XAxis
                  dataKey="displayTime"
                  stroke="#666"
                  fontSize={12}
                  tickLine={false}
                  axisLine={false}
                />
                <YAxis stroke="#666" fontSize={12} tickLine={false} axisLine={false} />
                <Tooltip content={<CustomTooltip />} cursor={{ fill: '#ffffff05' }} />
                <Bar
                  dataKey="total_cost_usd"
                  name="Cost"
                  fill="#4ade80"
                  radius={[4, 4, 0, 0]}
                />
              </BarChart>
            </ResponsiveContainer>
          </div>
        </Card>

        {/* Top Models Table */}
        <Card>
          <h3 className="text-lg font-bold text-white mb-4">Top Models by Spend</h3>
          <div className="space-y-3 overflow-y-auto max-h-[300px] pr-2 custom-scrollbar">
            {topModels.map((model, i) => (
              <div key={model.model_id} className="flex items-center justify-between p-3 rounded-lg bg-white/5 border border-white/5 hover:bg-white/10 transition-colors">
                <div className="flex items-center gap-3">
                  <div className="w-8 h-8 rounded bg-gradient-to-br from-slate-700 to-slate-800 flex items-center justify-center text-xs font-bold text-slate-300">
                    #{i + 1}
                  </div>
                  <div>
                    <div className="text-sm font-medium text-white">{model.model_name}</div>
                    <div className="text-xs text-slate-500 font-mono">{model.model_id}</div>
                  </div>
                </div>
                <div className="text-right">
                  <div className="text-sm font-bold text-green-400">${model.total_cost.toFixed(4)}</div>
                  <div className="text-xs text-slate-500">{model.request_count} reqs</div>
                </div>
              </div>
            ))}
          </div>
        </Card>
      </div>

    </div>
  )
}
