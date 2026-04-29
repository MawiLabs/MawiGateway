'use client'

import { useEffect, useMemo, useRef, useState } from 'react'
import { motion, AnimatePresence } from 'framer-motion'
import { Button, Card, Modal, Input, Badge, Skeleton } from '@/components/ui'
import Image from 'next/image'
import { toast } from 'sonner'
import { useSearchParams } from 'next/navigation'
import {
    Search,
    Tag,
    Calendar,
    Globe,
    Lock,
    Key,
    Plus,
    MessageSquare,
    Layers,
    Image as ImageIcon,
    Film,
    Mic,
    AudioLines,
    Repeat,
    ArrowRight,
    ArrowLeft,
    Sparkles,
    Settings2,
    RefreshCw,
    Cpu,
    AlertTriangle,
} from 'lucide-react'

type ProviderCategory = 'foundation' | 'hosted' | 'audio' | 'image' | 'selfhosted'

interface ProviderEntry {
    id: string
    name: string
    logo: string
    type: string
    color: string
    category: ProviderCategory
}

const PROVIDERS: ProviderEntry[] = [
    { id: 'openai', name: 'OpenAI', logo: '/providers/openai.png', type: 'openai', color: 'emerald', category: 'foundation' },
    { id: 'azure', name: 'Azure', logo: '/providers/azure.png', type: 'azure', color: 'cyan', category: 'hosted' },
    { id: 'gemini', name: 'Gemini', logo: '/providers/gemini.png', type: 'google', color: 'blue', category: 'foundation' },
    { id: 'anthropic', name: 'Anthropic', logo: '/providers/anthropic.png', type: 'anthropic', color: 'orange', category: 'foundation' },
    { id: 'xai', name: 'X.AI', logo: '/providers/xai.png', type: 'xai', color: 'slate', category: 'foundation' },
    { id: 'elevenlabs', name: 'ElevenLabs', logo: '/providers/elevenlabs.png', type: 'elevenlabs', color: 'slate', category: 'audio' },
    { id: 'mistral', name: 'Mistral', logo: '/providers/mistral.png', type: 'mistral', color: 'indigo', category: 'foundation' },
    { id: 'perplexity', name: 'Perplexity', logo: '/providers/perplexity.png', type: 'perplexity', color: 'violet', category: 'foundation' },
    { id: 'deepseek', name: 'DeepSeek', logo: '/providers/deepseek.png', type: 'deepseek', color: 'blue', category: 'foundation' },
    { id: 'selfhosted', name: 'Self-Hosted', logo: '/providers/self-hosted.png', type: 'selfhosted', color: 'gray', category: 'selfhosted' },
]

const CATEGORY_LABEL: Record<ProviderCategory, string> = {
    foundation: 'Foundation',
    hosted: 'Hosted',
    audio: 'Audio',
    image: 'Image',
    selfhosted: 'Self-hosted',
}

export default function ProvidersPage() {
  const searchParams = useSearchParams()
  const urlProviderId = searchParams.get('id')

  // null = show provider catalog grid (single-pane home view).
  // string = show detail view for that provider (models + key management).
  const [selectedProvider, setSelectedProvider] = useState<string | null>(null)
  const [configuredProviders, setConfiguredProviders] = useState<any[]>([])
  const [models, setModels] = useState<any[]>([])
  const [loading, setLoading] = useState(true)
  const [showModal, setShowModal] = useState(false)
  const [showApiKeyModal, setShowApiKeyModal] = useState(false)
  const [editingModel, setEditingModel] = useState<any>(null)

  // Confirmation Modals State
  const [showDeleteModelModal, setShowDeleteModelModal] = useState(false)
  const [modelToDelete, setModelToDelete] = useState<any>(null)

  const [showDeleteKeyModal, setShowDeleteKeyModal] = useState(false)
  const [isDeletingParams, setIsDeletingParams] = useState(false)

  const [isSubmitting, setIsSubmitting] = useState(false)

  // v3 "Add Provider" picker modal — the design centerpiece. Hero cards
  // for recently-used providers, tag chips for category filtering,
  // searchable 3-col grid for everything else, keyboard-navigable.
  const [showAddProviderModal, setShowAddProviderModal] = useState(false)
  const [pickerSearch, setPickerSearch] = useState('')
  const [pickerCategory, setPickerCategory] = useState<ProviderCategory | 'all'>('all')
  const [pickedProviderId, setPickedProviderId] = useState<string | null>(null)
  const [highlightedIndex, setHighlightedIndex] = useState<number>(-1)
  const [pendingProviderName, setPendingProviderName] = useState('')
  const pickerSearchRef = useRef<HTMLInputElement>(null)

  // Form fields
  const [modelName, setModelName] = useState('')
  const [modality, setModality] = useState<'text' | 'image' | 'video' | 'audio' | 'speech-to-text' | 'speech-to-speech' | 'multimodal'>('text')
  const [apiKey, setApiKey] = useState('')
  const [apiEndpoint, setApiEndpoint] = useState('')
  const [apiVersion, setApiVersion] = useState('2024-12-01-preview')

  const selectedProviderInfo = PROVIDERS.find(p => p.id === selectedProvider)
  const providerInstance = configuredProviders.find(p => p.provider_type === selectedProviderInfo?.type)
  const isConfigured = !!providerInstance

  // -- v3 picker memos --------------------------------------------------
  // Filter applies search + category to the FULL PROVIDERS list. Counts
  // ignore the active category (so the user can see how many would be in
  // a different bucket before clicking).
  const pickerFiltered = useMemo(() => {
    const q = pickerSearch.trim().toLowerCase()
    return PROVIDERS.filter(p => {
      if (pickerCategory !== 'all' && p.category !== pickerCategory) return false
      if (!q) return true
      return p.name.toLowerCase().includes(q) || p.type.toLowerCase().includes(q)
    })
  }, [pickerSearch, pickerCategory])

  const pickerCategoryCounts = useMemo(() => {
    const q = pickerSearch.trim().toLowerCase()
    const matchSearch = (p: ProviderEntry) =>
      !q || p.name.toLowerCase().includes(q) || p.type.toLowerCase().includes(q)
    const counts: Record<string, number> = { all: 0 }
    for (const p of PROVIDERS) {
      if (!matchSearch(p)) continue
      counts.all = (counts.all || 0) + 1
      counts[p.category] = (counts[p.category] || 0) + 1
    }
    return counts
  }, [pickerSearch])

  // "Recent" hero cards — providers the user actually has configured.
  // Capped at 2 to match the v3 mockup's 2-up grid; if they have more
  // they'll see them in the main grid below anyway.
  const recentProviders = useMemo(() => {
    return PROVIDERS
      .filter(p => configuredProviders.some(cp => cp.provider_type === p.type))
      .slice(0, 2)
  }, [configuredProviders])

  // Reset picker state every time the modal opens so it never lingers
  // mid-search from a previous session.
  useEffect(() => {
    if (!showAddProviderModal) return
    setPickerSearch('')
    setPickerCategory('all')
    setPickedProviderId(null)
    setHighlightedIndex(-1)
    setPendingProviderName('')
  }, [showAddProviderModal])

  // Keyboard nav inside the modal: ↑↓←→ to move highlight, Enter to
  // pick, "/" to focus search. Modal already handles ESC.
  useEffect(() => {
    if (!showAddProviderModal) return
    const handler = (e: KeyboardEvent) => {
      const target = e.target as HTMLElement
      const inInput = target?.tagName === 'INPUT' || target?.tagName === 'TEXTAREA'

      if (e.key === '/' && !inInput) {
        e.preventDefault()
        pickerSearchRef.current?.focus()
        return
      }
      if (!pickerFiltered.length) return
      if (e.key === 'ArrowDown' || e.key === 'ArrowRight') {
        e.preventDefault()
        const step = e.key === 'ArrowDown' ? 3 : 1
        setHighlightedIndex(prev => Math.min(pickerFiltered.length - 1, (prev < 0 ? -1 : prev) + step))
      } else if (e.key === 'ArrowUp' || e.key === 'ArrowLeft') {
        e.preventDefault()
        const step = e.key === 'ArrowUp' ? 3 : 1
        setHighlightedIndex(prev => Math.max(0, (prev < 0 ? 0 : prev) - step))
      } else if (e.key === 'Enter' && highlightedIndex >= 0) {
        e.preventDefault()
        const p = pickerFiltered[highlightedIndex]
        if (p) setPickedProviderId(p.id)
      }
    }
    window.addEventListener('keydown', handler)
    return () => window.removeEventListener('keydown', handler)
  }, [showAddProviderModal, pickerFiltered, highlightedIndex])

  const confirmPickedProvider = () => {
    if (!pickedProviderId) return
    setSelectedProvider(pickedProviderId)
    setShowAddProviderModal(false)
  }

  useEffect(() => {
    loadData()
  }, [])

  const loadData = async () => {
    setLoading(true)
    try {
      const [providersRes, modelsRes] = await Promise.all([
        fetch('/v1/user/providers', { credentials: 'include' }),
        fetch('/v1/user/models', { credentials: 'include' }),
      ])

      if (providersRes.ok) setConfiguredProviders(await providersRes.json())
      if (modelsRes.ok) setModels(await modelsRes.json())

      setLoading(false)
    } catch (error) {
      toast.error('Failed to load data')
      setLoading(false)
    }
  }

  // Filter models when provider changes, without network request
  const providerInstances = configuredProviders.filter(
    p => p.provider_type === selectedProviderInfo?.type
  )

  const filteredModels = models.filter((model: any) =>
    providerInstances.some(p => p.id === model.provider)
  )

  useEffect(() => {
    // Handle deep linking via ?id=<provider-uuid>. Once we resolve it
    // to one of our UI providers, drop the param so refreshes don't
    // keep re-selecting and bookmarks land on the catalog grid.
    if (!urlProviderId) return
    if (configuredProviders.length === 0) return
    const target = configuredProviders.find(p => p.id === urlProviderId)
    if (!target) return
    const uiProvider = PROVIDERS.find(p => p.type === target.provider_type)
    if (uiProvider && uiProvider.id !== selectedProvider) {
      setSelectedProvider(uiProvider.id)
      window.history.replaceState(null, '', '/providers')
    }
  }, [configuredProviders, urlProviderId, selectedProvider])


  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    if (isSubmitting) return
    setIsSubmitting(true)

    // Update existing model
    if (editingModel) {
      try {
        const updateData: any = { name: modelName }
        const provider = configuredProviders.find(p => p.id === editingModel.provider)

        if (provider?.provider_type === 'azure') {
          if (apiEndpoint) updateData.api_endpoint = apiEndpoint
          if (apiVersion) updateData.api_version = apiVersion
          if (apiKey) updateData.api_key = apiKey
        }

        const res = await fetch(`/v1/models/${editingModel.id}`, {
          method: 'PUT',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify(updateData),
        })

        if (res.ok) {
          toast.success('Model updated successfully')
          setShowModal(false)
          resetForm()
          await loadData()
        } else {
          const error = await res.text()
          toast.error(`Failed to update: ${error}`)
        }
      } catch (error) {
        toast.error(`Error: ${error}`)
      }
      setIsSubmitting(false)
      return
    }

    // Create new model
    try {
      let provider = providerInstance

      // Create provider if it doesn't exist
      if (!provider) {
        if (!apiKey && selectedProvider !== 'selfhosted') {
          toast.error('API key required for new provider')
          setIsSubmitting(false)
          return
        }

        const providerData: any = {
          name: pendingProviderName.trim() || `${selectedProviderInfo?.name} Provider`,
          provider_type: selectedProviderInfo?.type,
          api_key: apiKey,
        }

        if (selectedProvider === 'azure' || selectedProvider === 'selfhosted') {
          providerData.api_endpoint = apiEndpoint
          providerData.api_version = apiVersion
        }

        const res = await fetch('/v1/providers', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify(providerData),
        })

        if (res.ok) {
          provider = await res.json()
          await loadData()
        } else {
          const error = await res.text()
          toast.error(`Failed to create provider: ${error}`)
          setIsSubmitting(false)
          return
        }
      }

      // Create model
      const modelData: any = {
        name: modelName,
        provider: provider.id,
        modality: modality,
      }

      if (selectedProvider === 'azure' || selectedProvider === 'selfhosted') {
        if (apiEndpoint) modelData.api_endpoint = apiEndpoint
        if (apiVersion) modelData.api_version = apiVersion
        if (apiKey) modelData.api_key = apiKey
      }

      const res = await fetch('/v1/models', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(modelData),
      })

      if (res.ok) {
        toast.success('Model created successfully')
        setShowModal(false)
        resetForm()
        await loadData()
      } else {
        const error = await res.text()
        toast.error(`Failed to create model: ${error}`)
      }
    } catch (error) {
      toast.error(`Error: ${error}`)
    } finally {
      setIsSubmitting(false)
    }
  }

  const handleDelete = (model: any) => {
    setModelToDelete(model)
    setShowDeleteModelModal(true)
  }

  const confirmDeleteModel = async () => {
    if (!modelToDelete) return
    setIsDeletingParams(true)
    try {
      const res = await fetch(`/v1/models/${modelToDelete.id}`, {
        method: 'DELETE',
      })

      if (res.ok) {
        toast.success('Model deleted')
        setShowDeleteModelModal(false)
        await loadData()
      } else {
        toast.error('Failed to delete model')
      }
    } catch (error) {
      toast.error(`Error: ${error}`)
    } finally {
      setIsDeletingParams(false)
    }
  }

  const confirmDeleteKey = async () => {
    if (!providerInstance) return
    setIsDeletingParams(true)
    try {
      const res = await fetch(`/v1/providers/${providerInstance.id}`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ api_key: '' }),
      })

      if (res.ok) {
        toast.success('API key deleted')
        setShowDeleteKeyModal(false)
        setShowApiKeyModal(false)
        setApiKey('')
        await loadData()
      } else {
        const error = await res.text()
        toast.error(`Failed to delete API key: ${error}`)
      }
    } catch (error) {
      toast.error(`Error: ${error}`)
    } finally {
      setIsDeletingParams(false)
    }
  }

  const handleEdit = (model: any) => {
    setEditingModel(model)
    setModelName(model.name)
    setApiEndpoint(model.api_endpoint || '')
    setApiVersion(model.api_version || '2024-12-01-preview')
    setApiKey('')
    setShowModal(true)
  }

  const handleRefreshHealth = async (model: any) => {
    try {
      const button = document.activeElement as HTMLButtonElement
      if (button) button.disabled = true

      const res = await fetch(`/v1/user/models/${model.id}/health`, {
        method: 'POST',
        credentials: 'include',
      })

      if (res.ok) {
        const data = await res.json()
        toast.success(`Health checked: ${data.is_healthy ? 'Active' : 'Down'}${data.response_time_ms ? ` (${data.response_time_ms}ms)` : ''}`)
        await loadData() // Refresh to show updated status
      } else {
        toast.error('Failed to check health')
      }
    } catch (error) {
      toast.error(`Error: ${error}`)
    }
  }

  const resetForm = () => {
    setEditingModel(null)
    setModelName('')
    setModality('text')
    setApiKey('')
    setApiEndpoint('')
    setApiVersion('2024-12-01-preview')
  }

  return (
    <div className="relative h-screen overflow-y-auto bg-black">
      {/* Page content — blurs/dims behind the v3 picker modal so that
          modal feels like the unambiguous focal point. */}
      <div
        className={`min-h-full transition-all duration-300 ${
          showAddProviderModal ? 'blur-[2px] opacity-40 pointer-events-none' : ''
        }`}
      >
        {/* Topbar — single source of identity for /providers. Adapts:
            grid view = breadcrumbs + H1 "Providers" + Add CTA;
            detail view = breadcrumbs + ← Provider name + Add CTA. */}
        <div className="px-10 pt-10 pb-8 max-w-7xl mx-auto">
          <div className="text-[11px] text-slate-500 mb-3 tracking-[0.12em] uppercase font-semibold">
            {selectedProvider ? (
              <>
                <button
                  onClick={() => setSelectedProvider(null)}
                  className="hover:text-cyan-400 transition-colors"
                >
                  Workspace · Providers
                </button>
                <span className="mx-2 text-slate-700">·</span>
                <span className="text-slate-300">{selectedProviderInfo?.name}</span>
              </>
            ) : (
              <span>Workspace · Providers</span>
            )}
          </div>

          <div className="flex items-end justify-between gap-4">
            {selectedProvider ? (
              <button
                onClick={() => setSelectedProvider(null)}
                className="group inline-flex items-center gap-3 text-left"
              >
                <ArrowLeft className="w-6 h-6 text-slate-500 group-hover:text-cyan-400 group-hover:-translate-x-0.5 transition-all" />
                <h1 className="text-3xl font-bold bg-gradient-to-br from-white to-slate-400 bg-clip-text text-transparent leading-[1.1]">
                  {selectedProviderInfo?.name}
                </h1>
                <span className="text-sm text-slate-500 ml-1 font-mono tabular-nums">
                  · {filteredModels.length} model{filteredModels.length !== 1 ? 's' : ''}
                </span>
              </button>
            ) : (
              <div>
                <h1 className="text-4xl font-bold bg-gradient-to-br from-white to-slate-400 bg-clip-text text-transparent leading-[1.1]">
                  Providers
                </h1>
                <p className="text-slate-400 mt-2 text-sm">
                  <span className="font-mono tabular-nums">{configuredProviders.length}</span> configured
                  <span className="mx-2 text-slate-700">·</span>
                  <span className="font-mono tabular-nums">{PROVIDERS.length}</span> available in catalog
                </p>
              </div>
            )}

            <button
              onClick={() => setShowAddProviderModal(true)}
              className="group inline-flex shrink-0 items-center gap-2 px-4 py-2.5 rounded-xl
                         text-sm font-semibold text-black
                         bg-gradient-to-br from-cyan-300 to-cyan-600
                         shadow-[0_0_24px_rgba(34,211,238,0.32)]
                         hover:shadow-[0_0_32px_rgba(34,211,238,0.5)]
                         hover:-translate-y-px active:translate-y-0
                         transition-all"
            >
              <Plus className="w-4 h-4" strokeWidth={2.75} />
              Add Provider
            </button>
          </div>
        </div>

        {/* Body — GRID when no provider selected, DETAIL otherwise. */}
        <div className="px-10 pb-16 max-w-7xl mx-auto">
          {selectedProvider === null ? (
            /* ───────────── GRID VIEW (catalog of configured providers) ───────────── */
            loading ? (
              <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
                <Skeleton className="h-32 rounded-2xl" />
                <Skeleton className="h-32 rounded-2xl" />
                <Skeleton className="h-32 rounded-2xl" />
              </div>
            ) : configuredProviders.length === 0 ? (
              <div className="rounded-2xl border border-white/10 bg-gradient-to-b from-[#0a0a0a] to-[#050505] py-20 text-center">
                <div className="inline-flex items-center justify-center w-16 h-16 rounded-2xl bg-cyan-400/10 border border-cyan-400/20 mb-5 shadow-[0_0_24px_rgba(34,211,238,0.2)]">
                  <Plus className="w-8 h-8 text-cyan-400" strokeWidth={2} />
                </div>
                <h3 className="text-xl font-semibold text-white mb-2">No providers yet</h3>
                <p className="text-slate-400 mb-6 max-w-sm mx-auto text-sm leading-relaxed">
                  Connect your first AI provider to start routing requests through MawiGateway.
                </p>
                <button
                  onClick={() => setShowAddProviderModal(true)}
                  className="group inline-flex items-center gap-2 px-4 py-2.5 rounded-xl
                             text-sm font-semibold text-black
                             bg-gradient-to-br from-cyan-300 to-cyan-600
                             shadow-[0_0_24px_rgba(34,211,238,0.32)]
                             hover:shadow-[0_0_32px_rgba(34,211,238,0.5)]
                             hover:-translate-y-px transition-all"
                >
                  <Plus className="w-4 h-4" strokeWidth={2.75} />
                  Add your first provider
                </button>
              </div>
            ) : (
              <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
                {PROVIDERS.filter(p =>
                  configuredProviders.some(c => c.provider_type === p.type)
                ).map((p) => {
                  const cp = configuredProviders.find(c => c.provider_type === p.type)
                  const cpInstances = configuredProviders.filter(c => c.provider_type === p.type)
                  const cpModels = models.filter(m => cpInstances.some(c => c.id === m.provider))
                  const modelCount = cpModels.length
                  const healthyCount = cpModels.filter(m => m.health_status === 'healthy').length
                  const allHealthy = modelCount > 0 && healthyCount === modelCount
                  return (
                    <button
                      key={p.id}
                      onClick={() => setSelectedProvider(p.id)}
                      className="group relative overflow-hidden text-left rounded-2xl border border-white/10
                                 bg-gradient-to-br from-[#0f0f0f] to-[#080808] p-5
                                 transition-all duration-200
                                 hover:-translate-y-0.5 hover:border-cyan-400/40
                                 hover:shadow-[0_8px_24px_rgba(0,0,0,0.4),0_0_28px_rgba(34,211,238,0.18)]"
                    >
                      <span className="pointer-events-none absolute inset-0 bg-[radial-gradient(160px_100px_at_85%_-10%,rgba(34,211,238,0.12),transparent_70%)] opacity-0 group-hover:opacity-100 transition-opacity" />
                      <div className="relative flex items-center gap-3 mb-3">
                        <div className="w-12 h-12 rounded-xl bg-white p-2 border border-white/10 flex items-center justify-center shrink-0">
                          <img src={p.logo} alt={p.name} className="w-full h-full object-contain" />
                        </div>
                        <div className="flex-1 min-w-0">
                          <div className="text-base font-semibold text-white truncate">{p.name}</div>
                          <div className="text-[10px] font-semibold tracking-[0.1em] text-cyan-300/90 uppercase mt-0.5">
                            {CATEGORY_LABEL[p.category]}
                          </div>
                        </div>
                        <ArrowRight className="w-4 h-4 text-slate-600 group-hover:text-cyan-400 group-hover:translate-x-0.5 transition-all shrink-0" />
                      </div>
                      <div className="relative flex items-center gap-2 text-[11px] text-slate-400">
                        <span className="inline-flex items-center gap-1.5">
                          <span
                            className={`w-1.5 h-1.5 rounded-full ${
                              modelCount === 0
                                ? 'bg-slate-600'
                                : allHealthy
                                  ? 'bg-emerald-400 shadow-[0_0_8px_rgba(52,211,153,0.8)]'
                                  : 'bg-amber-400 shadow-[0_0_8px_rgba(251,191,36,0.6)]'
                            }`}
                          />
                          <span>
                            {modelCount === 0
                              ? 'no models'
                              : allHealthy
                                ? 'all healthy'
                                : `${healthyCount}/${modelCount} healthy`}
                          </span>
                        </span>
                        <span className="text-slate-700">·</span>
                        <span className="text-slate-500 font-mono tabular-nums">
                          {modelCount} model{modelCount === 1 ? '' : 's'}
                        </span>
                      </div>
                    </button>
                  )
                })}
              </div>
            )
          ) : (
            /* ───────────── DETAIL VIEW (selected provider's models + key) ───────────── */
            <motion.div
              key={selectedProvider}
              initial={{ opacity: 0, y: 8 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ duration: 0.18 }}
            >
              {/* Provider hero row — large logo, status, primary actions. */}
              <div className="flex items-center gap-5 mb-8 pb-8 border-b border-white/10">
                <div className="relative w-16 h-16 bg-white rounded-2xl overflow-hidden shadow-lg shadow-white/10 p-3 shrink-0">
                  <img
                    src={selectedProviderInfo?.logo || ''}
                    alt={selectedProviderInfo?.name || ''}
                    className="w-full h-full object-contain"
                  />
                </div>
                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-2 text-[11px] font-semibold tracking-[0.1em] text-cyan-300/90 uppercase mb-1">
                    {selectedProviderInfo && CATEGORY_LABEL[selectedProviderInfo.category]}
                    {isConfigured && (
                      <>
                        <span className="text-slate-700">·</span>
                        <span className="inline-flex items-center gap-1.5 text-emerald-400">
                          <span className="w-1.5 h-1.5 rounded-full bg-emerald-400 shadow-[0_0_8px_rgba(52,211,153,0.8)]" />
                          configured
                        </span>
                      </>
                    )}
                  </div>
                  <p className="text-slate-400 text-sm">
                    {filteredModels.length === 0
                      ? 'No models configured yet.'
                      : `${filteredModels.length} model${filteredModels.length === 1 ? '' : 's'} routed through this provider.`}
                  </p>
                </div>

                <div className="flex items-center gap-2 shrink-0">
                  <Button
                    variant="primary"
                    onClick={() => {
                      resetForm()
                      setShowModal(true)
                    }}
                    icon={<Plus className="w-4 h-4" strokeWidth={2.5} />}
                  >
                    Add Model
                  </Button>
                  {selectedProvider !== 'azure' && isConfigured && providerInstance?.has_api_key && (
                    <Button
                      variant="secondary"
                      onClick={() => {
                        setApiKey('')
                        setShowApiKeyModal(true)
                      }}
                      icon={<Lock className="w-4 h-4" strokeWidth={2} />}
                    >
                      Edit API Key
                    </Button>
                  )}
                </div>
              </div>

              {/* Inline API key input — when not yet configured (non-Azure). */}
              {selectedProvider !== 'azure' && !isConfigured && selectedProvider !== 'selfhosted' && (
                <div className="mb-6 flex gap-2 items-center max-w-xl">
                  <Input
                    type="password"
                    value={apiKey}
                    onChange={(e) => setApiKey(e.target.value)}
                    placeholder={`Enter ${selectedProviderInfo?.name} API key...`}
                    icon={<Key className="w-4 h-4" strokeWidth={2} />}
                    className="flex-1"
                  />
                  <Button
                    variant="primary"
                    disabled={!apiKey.trim()}
                    onClick={async () => {
                      if (!apiKey.trim()) return
                      try {
                        const createRes = await fetch('/v1/providers', {
                          method: 'POST',
                          headers: { 'Content-Type': 'application/json' },
                          body: JSON.stringify({
                            name: `${selectedProviderInfo?.name}`,
                            provider_type: selectedProviderInfo?.type,
                            api_key: apiKey,
                          }),
                        })
                        if (createRes.ok) {
                          toast.success('Provider configured with API key')
                          setApiKey('')
                          await loadData()
                        } else {
                          toast.error('Failed to create provider')
                        }
                      } catch (error) {
                        toast.error(`Error: ${error}`)
                      }
                    }}
                  >
                    Save
                  </Button>
                </div>
              )}

              {/* Models list */}
              {loading ? (
                <div className="space-y-3">
                  <Skeleton className="h-20 rounded-2xl" />
                  <Skeleton className="h-20 rounded-2xl" />
                  <Skeleton className="h-20 rounded-2xl" />
                </div>
              ) : filteredModels.length === 0 ? (
                <div className="rounded-2xl border border-white/10 bg-gradient-to-b from-[#0a0a0a] to-[#050505] py-16 text-center">
                  <div className="inline-flex items-center justify-center w-14 h-14 rounded-2xl bg-cyan-400/10 border border-cyan-400/20 mb-4 shadow-[0_0_24px_rgba(34,211,238,0.18)]">
                    <Cpu className="w-7 h-7 text-cyan-400" strokeWidth={1.75} />
                  </div>
                  <h3 className="text-lg font-semibold text-white mb-2">No models yet</h3>
                  <p className="text-slate-400 mb-6 text-sm">
                    Add your first {selectedProviderInfo?.name} model to get started.
                  </p>
                  <Button
                    variant="primary"
                    onClick={() => {
                      resetForm()
                      setShowModal(true)
                    }}
                    icon={<Plus className="w-4 h-4" strokeWidth={2.5} />}
                  >
                    Add Model
                  </Button>
                </div>
              ) : (
                <div className="space-y-3">
                  <AnimatePresence mode="popLayout">
                    {filteredModels.map((model, i) => (
                      <motion.div
                        key={model.id}
                        layout
                        initial={{ opacity: 0, y: 20 }}
                        animate={{ opacity: 1, y: 0 }}
                        exit={{ opacity: 0, scale: 0.95 }}
                        transition={{ delay: i * 0.03 }}
                      >
                        <Card hover className="group">
                          <div className="flex items-center justify-between">
                            <div className="flex items-center gap-4">
                              <div className="w-12 h-12 rounded-xl bg-gradient-to-br from-cyan-400/20 to-cyan-600/20 border border-cyan-400/50 flex items-center justify-center flex-shrink-0">
                                <Cpu className="w-6 h-6 text-cyan-300" strokeWidth={1.75} />
                              </div>
                              <div>
                                <div className="flex items-center gap-2 mb-1">
                                  <div className="font-semibold text-white text-lg">{model.name}</div>
                                  <Badge
                                    variant={
                                      model.health_status === 'healthy'
                                        ? 'success'
                                        : model.health_status === 'warning'
                                          ? 'warning'
                                          : 'danger'
                                    }
                                    size="sm"
                                  >
                                    {model.health_status === 'healthy'
                                      ? 'Active'
                                      : model.health_status === 'warning'
                                        ? 'Warning'
                                        : 'Down'}
                                  </Badge>
                                </div>
                                <div className="text-sm text-slate-400 capitalize">
                                  {model.modality} model
                                  {model.last_error && model.health_status !== 'healthy' && (
                                    <span className="text-red-400 text-xs ml-2 block">
                                      Error: {model.last_error}
                                    </span>
                                  )}
                                </div>
                              </div>
                            </div>
                            <div className="flex gap-2 opacity-0 group-hover:opacity-100 transition-opacity">
                              <Button
                                variant="ghost"
                                size="sm"
                                onClick={() => handleRefreshHealth(model)}
                                className="text-cyan-400 hover:text-cyan-300"
                              >
                                <Repeat className="w-3.5 h-3.5 mr-1" strokeWidth={2} />
                                Refresh
                              </Button>
                              <Button variant="ghost" size="sm" onClick={() => handleEdit(model)}>
                                Edit
                              </Button>
                              <Button
                                variant="ghost"
                                size="sm"
                                onClick={() => handleDelete(model)}
                                className="text-red-400 hover:text-red-300"
                              >
                                Delete
                              </Button>
                            </div>
                          </div>
                        </Card>
                      </motion.div>
                    ))}
                  </AnimatePresence>
                </div>
              )}
            </motion.div>
          )}
        </div>
      </div>


      {/* Add/Edit Modal */}
      <Modal
        isOpen={showModal}
        onClose={() => {
          setShowModal(false)
          resetForm()
        }}
        title={editingModel ? 'Edit Model' : `Add ${selectedProviderInfo?.name} Model`}
        description={editingModel ? 'Update model configuration' : 'Configure a new model deployment'}>

        <form onSubmit={handleSubmit} className="space-y-4">
          <Input
            label="Model Name"
            value={modelName}
            onChange={(e) => setModelName(e.target.value)}
            placeholder={
              selectedProvider === 'azure' ? 'gpt-4-deployment' :
                selectedProvider === 'elevenlabs' ? 'eleven_multilingual_v2' : 'gpt-4o-mini'
            }
            icon={<Tag className="w-4 h-4" strokeWidth={2} />}
            required
          />

          {/* Modality Selection */}
          <div className="space-y-2">
            <label className="block text-sm font-medium text-slate-300">
              Modality
            </label>
            <select
              value={modality}
              onChange={(e) => setModality(e.target.value as any)}
              className="w-full px-4 py-2.5 bg-[#0f0f0f] border border-white/10 rounded-xl text-white focus:border-cyan-500 focus:outline-none transition-colors"
            >
              <option value="text">Text (Chat/Completion)</option>
              <option value="multimodal">Multimodal (Text + Image Output)</option>
              <option value="image">Image (Generation Only)</option>
              <option value="video">Video (Generation)</option>
              <option value="audio">Audio (Text-to-Speech)</option>
              <option value="speech-to-text">Speech-to-Text</option>
              <option value="speech-to-speech">Speech-to-Speech</option>
            </select>
            <p className="text-xs text-slate-500">Select the capability this model provides</p>
          </div>

          {/* Azure and Self-Hosted Fields */}
          {(selectedProvider === 'azure' || selectedProvider === 'selfhosted') && (
            <>
              <Input
                label={selectedProvider === 'azure' ? `Azure Endpoint ${!editingModel && !isConfigured ? '*' : ''}` : 'Base URL *'}
                value={apiEndpoint}
                onChange={(e) => setApiEndpoint(e.target.value)}
                placeholder={selectedProvider === 'azure' ? 'https://your-resource.openai.azure.com' : 'http://localhost:11434'}
                icon={<Globe className="w-4 h-4" strokeWidth={2} />}
                helperText={selectedProvider === 'azure' ? (isConfigured ? "Override provider endpoint (optional)" : "Your Azure OpenAI resource URL") : "URL of your OpenAI-compatible endpoint"}
                required={selectedProvider === 'selfhosted' || (!editingModel && !isConfigured)}
              />

              {selectedProvider === 'azure' && (
                <Input
                  label="API Version"
                  value={apiVersion}
                  onChange={(e) => setApiVersion(e.target.value)}
                  placeholder="2024-12-01-preview"
                  icon={<Calendar className="w-4 h-4" strokeWidth={2} />}
                  helperText="Azure API version"
                />
              )}

              <Input
                label={`API Key ${editingModel ? '(keep existing)' : isConfigured ? '(override)' : selectedProvider === 'selfhosted' ? '(optional)' : '*'}`}
                type="password"
                value={apiKey}
                onChange={(e) => setApiKey(e.target.value)}
                placeholder={editingModel ? 'Leave blank to keep existing' : '...'}
                icon={<Key className="w-4 h-4" strokeWidth={2} />}
                helperText={editingModel || isConfigured ? 'Deployment-specific key (optional)' : 'Provider will be created'}
                required={selectedProvider !== 'selfhosted' && !editingModel && !isConfigured}
              />
            </>
          )}

          {/* Non-Azure API key */}
          {!editingModel && !isConfigured && selectedProvider !== 'azure' && selectedProvider !== 'selfhosted' && (
            <Input
              label="API Key *"
              type="password"
              value={apiKey}
              onChange={(e) => setApiKey(e.target.value)}
              placeholder="sk-..."
              icon={<Key className="w-4 h-4" strokeWidth={2} />}
              helperText="Provider will be created with this key"
              required
            />
          )}

          <div className="flex gap-3 pt-4 border-t border-white/10">
            <Button
              type="button"
              variant="secondary"
              onClick={() => {
                setShowModal(false)
                resetForm()
              }}
              className="flex-1">
              Cancel
            </Button>
            <Button
              type="submit"
              variant="primary"
              loading={isSubmitting}
              disabled={isSubmitting}
              className="flex-1">
              {isSubmitting
                ? editingModel ? 'Updating...' : 'Creating...'
                : editingModel ? 'Update Model' : 'Create Model'}
            </Button>
          </div>
        </form>
      </Modal>

      {/* Edit API Key Modal */}
      <Modal
        isOpen={showApiKeyModal}
        onClose={() => setShowApiKeyModal(false)}
        title={`Edit ${selectedProviderInfo?.name} API Key`}>
        <form
          onSubmit={async (e) => {
            e.preventDefault()
            if (!providerInstance || !apiKey.trim()) return

            try {
              const res = await fetch(`http://127.0.0.1:8030/v1/providers/${providerInstance.id}`, {
                method: 'PUT',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ api_key: apiKey }),
              })

              if (res.ok) {
                toast.success('API key updated successfully')
                setShowApiKeyModal(false)
                setApiKey('')
                await loadData()
              } else {
                const error = await res.text()
                toast.error(`Failed to update API key: ${error}`)
              }
            } catch (error) {
              toast.error(`Error: ${error}`)
            }
          }}
          className="space-y-4">
          <Input
            label="New API Key"
            type="password"
            value={apiKey}
            onChange={(e) => setApiKey(e.target.value)}
            placeholder="sk-..."
            icon={<Key className="w-4 h-4" strokeWidth={2} />}
            helperText="Enter the new API key for this provider"
            required
          />

          <div className="flex gap-3 pt-4 border-t border-white/10">
            <Button
              type="button"
              variant="danger"
              onClick={() => setShowDeleteKeyModal(true)}>
              Delete Key
            </Button>
            <Button
              type="button"
              variant="secondary"
              onClick={() => {
                setShowApiKeyModal(false)
                setApiKey('')
              }}
              className="flex-1">
              Cancel
            </Button>
            <Button
              type="submit"
              variant="primary"
              className="flex-1">
              Update API Key
            </Button>
          </div>
        </form>
      </Modal>

      {/* Delete Model Confirmation Modal */}
      <Modal
        isOpen={showDeleteModelModal}
        onClose={() => setShowDeleteModelModal(false)}
        title="Delete Model"
        description="Are you sure you want to delete this model?"
      >
        <div>
          <p className="text-slate-300 mb-6">
            This will permanently delete the model <strong className="text-white">{modelToDelete?.name}</strong>.
          </p>
          <div className="flex justify-end gap-3">
            <Button variant="ghost" onClick={() => setShowDeleteModelModal(false)} disabled={isDeletingParams}>Cancel</Button>
            <Button variant="danger" onClick={confirmDeleteModel} disabled={isDeletingParams}>
              {isDeletingParams ? 'Deleting...' : 'Delete Model'}
            </Button>
          </div>
        </div>
      </Modal>

      {/* Delete API Key Confirmation Modal */}
      <Modal
        isOpen={showDeleteKeyModal}
        onClose={() => setShowDeleteKeyModal(false)}
        title="Delete API Key"
        description="Are you sure you want to delete the API key?"
      >
        <div>
          <p className="text-slate-300 mb-6 flex items-center gap-2">
            <AlertTriangle className="w-4 h-4 text-amber-400 shrink-0" strokeWidth={2} />
            This provider will stop working until a new API key is provided.
          </p>
          <div className="flex justify-end gap-3">
            <Button variant="ghost" onClick={() => setShowDeleteKeyModal(false)} disabled={isDeletingParams}>Cancel</Button>
            <Button variant="danger" onClick={confirmDeleteKey} disabled={isDeletingParams}>
              {isDeletingParams ? 'Deleting...' : 'Delete Key'}
            </Button>
          </div>
        </div>
      </Modal>

      {/* ===================================================================
          v3 ADD PROVIDER PICKER — the design centerpiece.
          Mirrors /designs/mawi-web-form-modal-20260427/preview.html exactly:
          search bar (cyan magnifier) + tag chips with counts + hero "Recent"
          cards for configured providers + 3-col scrollable grid for the rest
          + keyboard hints + "Request a provider" link in the footer.
          =================================================================== */}
      <Modal
        isOpen={showAddProviderModal}
        onClose={() => setShowAddProviderModal(false)}
        title="Add Provider"
        description="Pick from supported providers, or browse the catalog"
        size="xl"
        autoFocus={false}
        footer={
          <>
            <div className="flex-1 text-xs text-slate-500">
              {pickedProviderId ? (
                <>
                  <strong className="text-slate-200">
                    {PROVIDERS.find(p => p.id === pickedProviderId)?.name}
                  </strong>{' '}
                  selected
                </>
              ) : (
                'No provider selected'
              )}
            </div>
            <Button
              variant="secondary"
              onClick={() => setShowAddProviderModal(false)}>
              Cancel
            </Button>
            <Button
              variant="primary"
              disabled={!pickedProviderId}
              onClick={confirmPickedProvider}
              icon={<ArrowRight className="w-4 h-4" strokeWidth={2.5} />}>
              Continue
            </Button>
          </>
        }
      >
        {/* ── Field 1: Provider name (preview.html line 583-589) ── */}
        <div className="mb-6">
          <label
            htmlFor="picker-provider-name"
            className="flex items-center justify-between text-[13px] font-semibold text-slate-300 mb-2.5 tracking-[0.01em]"
          >
            Provider name
            <span className="text-[11px] font-medium text-slate-500">
              a friendly label, like &ldquo;OpenAI Production&rdquo;
            </span>
          </label>
          <input
            id="picker-provider-name"
            type="text"
            value={pendingProviderName}
            onChange={(e) => setPendingProviderName(e.target.value)}
            placeholder={
              pickedProviderId
                ? `${PROVIDERS.find(p => p.id === pickedProviderId)?.name} Production`
                : 'OpenAI Production'
            }
            className="w-full px-3.5 py-3 bg-black border border-white/10 rounded-xl text-white text-sm placeholder-slate-600 outline-none focus:border-cyan-400 focus:ring-4 focus:ring-cyan-400/15 transition-all"
          />
        </div>

        {/* ── Field 2: Provider type — the picker ── */}
        <label className="flex items-center justify-between text-[13px] font-semibold text-slate-300 mb-2.5 tracking-[0.01em]">
          Provider type
          <span className="text-[11px] font-medium text-slate-500">
            {PROVIDERS.length} providers · scales to hundreds
          </span>
        </label>

        {/* Picker shell — matches preview.html structure */}
        <div className="rounded-2xl border border-white/10 overflow-hidden bg-gradient-to-b from-[#0a0a0a] to-[#050505]">

          {/* Search bar */}
          <div className="flex items-center gap-2.5 px-4 py-3.5 border-b border-white/10 bg-white/[0.015]">
            <Search className="w-4 h-4 text-cyan-400 shrink-0" strokeWidth={2} />
            <input
              ref={pickerSearchRef}
              type="text"
              value={pickerSearch}
              onChange={(e) => {
                setPickerSearch(e.target.value)
                setHighlightedIndex(-1)
              }}
              placeholder={`Search ${PROVIDERS.length} providers by name or capability...`}
              className="flex-1 bg-transparent border-none outline-none text-white placeholder-slate-600 text-sm"
            />
            <span className="font-mono text-[10px] font-semibold tracking-wider px-1.5 py-0.5 rounded bg-white/[0.05] border border-white/15 text-slate-400">
              /
            </span>
          </div>

          {/* Tag chips */}
          <div
            className="flex gap-1.5 px-4 py-2.5 border-b border-white/10 overflow-x-auto"
            style={{ scrollbarWidth: 'none' }}
          >
            {(['all', 'foundation', 'hosted', 'audio', 'image', 'selfhosted'] as const).map((cat) => {
              const active = pickerCategory === cat
              const label = cat === 'all' ? 'All' : CATEGORY_LABEL[cat]
              const count = pickerCategoryCounts[cat] || 0
              if (cat !== 'all' && count === 0) return null
              return (
                <button
                  key={cat}
                  onClick={() => {
                    setPickerCategory(cat)
                    setHighlightedIndex(-1)
                  }}
                  className={`flex-shrink-0 inline-flex items-center gap-1.5 text-[11px] font-semibold px-2.5 py-1 rounded-full border transition-all ${
                    active
                      ? 'bg-cyan-400/12 text-cyan-200 border-cyan-400/40 shadow-[0_0_12px_rgba(34,211,238,0.2)]'
                      : 'bg-white/[0.04] text-slate-400 border-white/10 hover:bg-white/[0.07] hover:text-slate-200'
                  }`}
                >
                  {label}
                  <span className={active ? 'text-cyan-400' : 'text-slate-600 font-medium'}>
                    {count}
                  </span>
                </button>
              )
            })}
          </div>

          {/* Recent section — only when there are configured providers */}
          {recentProviders.length > 0 && pickerCategory === 'all' && !pickerSearch && (
            <>
              <div className="flex items-center gap-3 px-4 pt-3.5 pb-2">
                <span className="text-[10px] font-bold tracking-[0.12em] text-slate-600 uppercase">
                  Recent
                </span>
                <span className="flex-1 h-px bg-gradient-to-r from-white/10 to-transparent" />
                <span className="text-[11px] text-slate-600">used recently</span>
              </div>
              <div className="grid grid-cols-2 gap-2.5 px-4">
                {recentProviders.map((p) => {
                  const cp = configuredProviders.find(c => c.provider_type === p.type)
                  const modelCount = models.filter(m => cp && m.provider === cp.id).length
                  const isPicked = pickedProviderId === p.id
                  return (
                    <button
                      key={p.id}
                      onClick={() => setPickedProviderId(p.id)}
                      onDoubleClick={() => {
                        setPickedProviderId(p.id)
                        confirmPickedProvider()
                      }}
                      className={`group relative overflow-hidden rounded-2xl border p-4 text-left transition-all ${
                        isPicked
                          ? 'border-cyan-400 bg-gradient-to-br from-cyan-400/[0.1] to-[#080808] shadow-[0_0_0_2px_rgba(34,211,238,0.5),0_12px_32px_rgba(0,0,0,0.5),0_0_40px_rgba(34,211,238,0.32)]'
                          : 'border-white/15 bg-gradient-to-br from-[#0f0f0f] to-[#080808] hover:border-cyan-400/50 hover:-translate-y-0.5 hover:shadow-[0_8px_24px_rgba(0,0,0,0.4),0_0_32px_rgba(34,211,238,0.22)]'
                      }`}
                    >
                      {/* Corner cyan glow — fully visible per preview spec */}
                      <span className="pointer-events-none absolute inset-0 bg-[radial-gradient(140px_90px_at_85%_-10%,rgba(34,211,238,0.2),transparent_70%)]" />

                      <div className="relative flex items-center gap-3">
                        <div className="w-11 h-11 shrink-0 rounded-xl bg-white p-2 border border-white/10 flex items-center justify-center">
                          <img src={p.logo} alt={p.name} className="w-full h-full object-contain" />
                        </div>
                        <div className="flex-1 min-w-0">
                          <div className="text-[15px] font-semibold text-white truncate">{p.name}</div>
                          <div className="text-[10px] font-semibold tracking-wider text-cyan-300/90 mt-1 truncate">
                            {CATEGORY_LABEL[p.category].toUpperCase()} · {modelCount} model{modelCount === 1 ? '' : 's'}
                          </div>
                        </div>
                      </div>
                      <div className="relative mt-2.5 flex items-center gap-2 text-[11px] text-slate-400">
                        <span className="inline-flex items-center gap-1.5">
                          <span className="w-1.5 h-1.5 rounded-full bg-emerald-400 shadow-[0_0_8px_rgba(52,211,153,0.8)]" />
                          <span>healthy</span>
                        </span>
                        <span className="text-slate-700">·</span>
                        <span className="text-slate-500">{modelCount} configured</span>
                      </div>
                    </button>
                  )
                })}
              </div>
            </>
          )}

          {/* All providers section */}
          <div className="flex items-center gap-3 px-4 pt-3.5 pb-2">
            <span className="text-[10px] font-bold tracking-[0.12em] text-slate-600 uppercase">
              {pickerSearch || pickerCategory !== 'all' ? 'Results' : 'All providers'}
            </span>
            <span className="flex-1 h-px bg-gradient-to-r from-white/10 to-transparent" />
            <span className="text-[11px] text-slate-600">
              {pickerFiltered.length} {pickerFiltered.length === 1 ? 'match' : 'matches'}
            </span>
          </div>

          {pickerFiltered.length === 0 ? (
            <div className="px-4 pb-6 pt-4 text-center">
              <div className="text-sm text-slate-400">No providers match your search.</div>
              <button
                onClick={() => {
                  setPickerSearch('')
                  setPickerCategory('all')
                }}
                className="mt-3 text-xs font-semibold text-cyan-400 hover:text-cyan-300"
              >
                Clear filters
              </button>
            </div>
          ) : (
            <div
              className="grid grid-cols-3 gap-2.5 px-4 pb-3 max-h-[420px] overflow-y-auto"
              style={{ scrollbarColor: 'rgba(255,255,255,0.1) transparent' }}
            >
              {pickerFiltered.map((p, idx) => {
                const isPicked = pickedProviderId === p.id
                const isHighlighted = highlightedIndex === idx
                const tagClass: Record<ProviderCategory, string> = {
                  foundation: 'bg-cyan-400/10 text-cyan-300 border-cyan-400/20',
                  hosted: 'bg-violet-400/10 text-violet-300 border-violet-400/20',
                  audio: 'bg-pink-400/10 text-pink-300 border-pink-400/20',
                  image: 'bg-amber-400/10 text-amber-200 border-amber-400/20',
                  selfhosted: 'bg-emerald-400/10 text-emerald-300 border-emerald-400/20',
                }
                return (
                  <button
                    key={p.id}
                    onClick={() => {
                      setPickedProviderId(p.id)
                      setHighlightedIndex(idx)
                    }}
                    onDoubleClick={() => {
                      setPickedProviderId(p.id)
                      confirmPickedProvider()
                    }}
                    onMouseEnter={() => setHighlightedIndex(idx)}
                    className={`relative flex flex-col items-center gap-2.5 rounded-xl border p-4 min-h-[120px] text-center transition-all ${
                      isPicked
                        ? 'border-cyan-400 bg-gradient-to-br from-cyan-400/[0.08] to-[#0a0a0a] shadow-[0_0_0_2px_rgba(34,211,238,0.5),0_8px_24px_rgba(0,0,0,0.5),0_0_28px_rgba(34,211,238,0.3)]'
                        : isHighlighted
                          ? 'border-cyan-400/60 bg-gradient-to-br from-cyan-400/[0.06] to-[#0a0a0a] shadow-[0_0_0_1px_rgba(34,211,238,0.4),0_0_24px_rgba(34,211,238,0.18)]'
                          : 'border-white/10 bg-[#0a0a0a] hover:bg-[#0f0f0f] hover:border-cyan-400/40 hover:-translate-y-0.5 hover:shadow-[0_8px_24px_rgba(0,0,0,0.4),0_0_24px_rgba(34,211,238,0.18)]'
                    }`}
                  >
                    <div className="w-10 h-10 rounded-xl bg-white p-1.5 flex items-center justify-center">
                      <img src={p.logo} alt={p.name} className="w-full h-full object-contain" />
                    </div>
                    <div className="text-[13px] font-semibold text-slate-100 leading-tight">
                      {p.name}
                    </div>
                    <div
                      className={`text-[9px] font-semibold uppercase tracking-wider px-1.5 py-0.5 rounded-full border ${tagClass[p.category]}`}
                    >
                      {CATEGORY_LABEL[p.category]}
                    </div>
                  </button>
                )
              })}
            </div>
          )}

          {/* Footer */}
          <div className="border-t border-white/10 px-4 py-2.5 flex items-center justify-between text-[11px] bg-black/40">
            <div className="flex gap-3.5 text-slate-500">
              <span className="inline-flex items-center gap-1.5">
                <kbd className="font-mono text-[10px] font-semibold px-1 py-px rounded bg-white/[0.05] border border-white/10 text-slate-300">↑↓←→</kbd>
                navigate
              </span>
              <span className="inline-flex items-center gap-1.5">
                <kbd className="font-mono text-[10px] font-semibold px-1 py-px rounded bg-white/[0.05] border border-white/10 text-slate-300">↵</kbd>
                select
              </span>
              <span className="inline-flex items-center gap-1.5">
                <kbd className="font-mono text-[10px] font-semibold px-1 py-px rounded bg-white/[0.05] border border-white/10 text-slate-300">/</kbd>
                search
              </span>
            </div>
            <a
              href="https://github.com/MawiLabs/MawiGateway/issues/new?labels=provider-request&title=Provider+request%3A+"
              target="_blank"
              rel="noreferrer"
              className="inline-flex items-center gap-1 font-semibold text-cyan-400 hover:text-cyan-300"
            >
              <Sparkles className="w-3 h-3" strokeWidth={2.5} />
              Request a provider
            </a>
          </div>
        </div>
      </Modal>
    </div>
  )
}
