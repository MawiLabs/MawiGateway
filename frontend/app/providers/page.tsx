'use client'

import { useEffect, useState } from 'react'
import { motion, AnimatePresence } from 'framer-motion'
import { Button, Card, Modal, Input, Badge, Skeleton } from '@/components/ui'
import Image from 'next/image'
import { toast } from 'sonner'
import { useSearchParams } from 'next/navigation'

const PROVIDERS = [
  { id: 'openai', name: 'OpenAI', logo: '/providers/openai.png', type: 'openai', color: 'emerald' },
  { id: 'azure', name: 'Azure', logo: '/providers/azure.png', type: 'azure', color: 'cyan' },
  { id: 'gemini', name: 'Gemini', logo: '/providers/gemini.png', type: 'google', color: 'blue' },
  { id: 'anthropic', name: 'Anthropic', logo: '/providers/anthropic.png', type: 'anthropic', color: 'orange' },
  { id: 'xai', name: 'X.AI', logo: '/providers/xai.png', type: 'xai', color: 'slate' },
  { id: 'elevenlabs', name: 'ElevenLabs', logo: '/providers/elevenlabs.png', type: 'elevenlabs', color: 'slate' },
  { id: 'mistral', name: 'Mistral', logo: '/providers/mistral.png', type: 'mistral', color: 'indigo' },
  { id: 'perplexity', name: 'Perplexity', logo: '/providers/perplexity.png', type: 'perplexity', color: 'violet' },
  { id: 'deepseek', name: 'DeepSeek', logo: '/providers/deepseek.png', type: 'deepseek', color: 'blue' },
  { id: 'selfhosted', name: 'Self-Hosted', logo: '/providers/self-hosted.png', type: 'selfhosted', color: 'gray' },
]

export default function ProvidersPage() {
  const searchParams = useSearchParams()
  const urlProviderId = searchParams.get('id')

  const [selectedProvider, setSelectedProvider] = useState('openai')
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

  // Form fields
  const [modelName, setModelName] = useState('')
  const [modality, setModality] = useState<'text' | 'image' | 'video' | 'audio' | 'speech-to-text' | 'speech-to-speech' | 'multimodal'>('text')
  const [apiKey, setApiKey] = useState('')
  const [apiEndpoint, setApiEndpoint] = useState('')
  const [apiVersion, setApiVersion] = useState('2024-12-01-preview')

  const selectedProviderInfo = PROVIDERS.find(p => p.id === selectedProvider)
  const providerInstance = configuredProviders.find(p => p.provider_type === selectedProviderInfo?.type)
  const isConfigured = !!providerInstance

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
    // Handle deep linking
    if (urlProviderId && configuredProviders.length > 0) {
      const target = configuredProviders.find(p => p.id === urlProviderId)
      if (target) {
        const uiProvider = PROVIDERS.find(p => p.type === target.provider_type)
        if (uiProvider && uiProvider.id !== selectedProvider) {
          setSelectedProvider(uiProvider.id)
          window.history.replaceState(null, '', '/providers')
        }
      }
    }
  }, [configuredProviders, urlProviderId, selectedProvider])


  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()

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
      return
    }

    // Create new model
    try {
      let provider = providerInstance

      // Create provider if it doesn't exist
      if (!provider) {
        if (!apiKey && selectedProvider !== 'selfhosted') {
          toast.error('API key required for new provider')
          return
        }

        const providerData: any = {
          name: `${selectedProviderInfo?.name} Provider`,
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
    <div className="flex h-screen">
      {/* LEFT SIDEBAR - Provider List */}
      <div className="w-80 border-r border-white/10 flex flex-col bg-gradient-to-b from-black via-[#0f0f0f] to-black shrink-0">
        <div className="p-6 border-b border-white/10">
          <h2 className="text-lg font-bold text-white">Providers</h2>
          <p className="text-xs text-slate-400 mt-1">Select a provider to configure</p>
        </div>

        <div className="flex-1 p-4 space-y-2 overflow-y-auto">
          {PROVIDERS.map((provider) => {
            const configured = configuredProviders.some(p => p.provider_type === provider.type)
            const isSelected = selectedProvider === provider.id

            // Calculate correct model count for this provider type
            const typeInstances = configuredProviders.filter(p => p.provider_type === provider.type)
            const modelCount = models.filter(m => typeInstances.some(p => p.id === m.provider)).length

            return (
              <motion.button
                key={provider.id}
                whileHover={{ x: 4 }}
                whileTap={{ scale: 0.98 }}
                onClick={() => setSelectedProvider(provider.id)}
                className={`
                  w-full relative overflow-hidden rounded-xl p-4 transition-all duration-200
                  ${isSelected
                    ? 'bg-gradient-to-r from-cyan-400/20 to-cyan-600/20 border border-cyan-400/50'
                    : 'bg-[#0f0f0f] border border-white/10 hover:border-white/20 hover:bg-[#1a1a1a]'
                  }
                `}>
                {/* Selection indicator */}
                {isSelected && (
                  <motion.div
                    layoutId="provider-active"
                    className="absolute left-0 top-0 bottom-0 w-1 bg-gradient-to-b from-cyan-400 to-cyan-600"
                    transition={{ type: 'spring', stiffness: 380, damping: 30 }}
                  />
                )}

                <div className="flex items-center gap-3">
                  <div className="relative w-8 h-8 rounded-lg overflow-hidden bg-white p-1">
                    <img
                      src={provider.logo}
                      alt={provider.name}
                      className="w-full h-full object-contain"
                    />
                  </div>
                  <div className="flex-1 text-left">
                    <div className={`font-semibold ${isSelected ? 'text-white' : 'text-slate-300'}`}>
                      {provider.name}
                    </div>
                    {configured ? (
                      <div className="flex items-center gap-1.5 text-xs mt-0.5">
                        <div className="w-1.5 h-1.5 rounded-full bg-emerald-400 animate-pulse" />
                        <span className="text-emerald-400">
                          {isSelected ? `${modelCount} models` : 'Configured'}
                        </span>
                      </div>
                    ) : (
                      <div className="text-xs text-slate-500 mt-0.5">Not configured</div>
                    )}
                  </div>
                </div>
              </motion.button>
            )
          })}
        </div>
      </div>

      {/* RIGHT PANEL - Models */}
      <div className="flex-1 overflow-y-auto p-8">
        <motion.div
          key={selectedProvider}
          initial={{ opacity: 0, x: 20 }}
          animate={{ opacity: 1, x: 0 }}
          className="max-w-5xl mx-auto">

          {/* Header */}
          <div className="mb-8">
            <div className="flex items-center gap-4 mb-2">
              <div className="relative w-16 h-16 bg-white rounded-2xl overflow-hidden shadow-lg shadow-white/10 p-3">
                <img
                  src={selectedProviderInfo?.logo || ''}
                  alt={selectedProviderInfo?.name || ''}
                  className="w-full h-full object-contain"
                />
              </div>
              <div>
                <h1 className="text-3xl font-bold text-white">
                  {selectedProviderInfo?.name}
                </h1>
                <p className="text-slate-400">
                  {filteredModels.length} model{filteredModels.length !== 1 ? 's' : ''} configured
                </p>
              </div>
            </div>
          </div>

          {/* Action Buttons */}
          <div className="mb-6 flex gap-3 items-center">
            <Button
              variant="primary"
              onClick={() => {
                resetForm()
                setShowModal(true)
              }}
              icon={<span className="text-xl">+</span>}>
              Add Model
            </Button>

            {/* API Key Section - not for Azure (has per-deployment keys) */}
            {selectedProvider !== 'azure' && isConfigured && providerInstance?.has_api_key ? (
              // Sealed state - show edit button
              <Button
                variant="secondary"
                onClick={() => {
                  setApiKey('')
                  setShowApiKeyModal(true)
                }}
                icon={<span className="text-xl">🔒</span>}>
                Edit API Key
              </Button>
            ) : selectedProvider !== 'azure' ? (
              // No API key or not configured - show inline input (not for Azure)
              <div className="flex gap-2 items-center flex-1 max-w-md">
                <Input
                  type="password"
                  value={apiKey}
                  onChange={(e) => setApiKey(e.target.value)}
                  placeholder={`Enter ${selectedProviderInfo?.name} API key...`}
                  icon={<span>🔑</span>}
                  className="flex-1"
                />
                <Button
                  variant="primary"
                  disabled={!apiKey.trim() && selectedProvider !== 'selfhosted'}
                  onClick={async () => {
                    if (!apiKey.trim() && selectedProvider !== 'selfhosted') return
                    try {
                      // If provider doesn't exist, create it first
                      let pid = providerInstance?.id
                      if (!pid) {
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
                          return
                        } else {
                          toast.error('Failed to create provider')
                          return
                        }
                      }

                      // Update existing provider
                      const res = await fetch(`/v1/providers/${pid}`, {
                        method: 'PUT',
                        headers: { 'Content-Type': 'application/json' },
                        body: JSON.stringify({ api_key: apiKey }),
                      })
                      if (res.ok) {
                        toast.success('API key saved')
                        setApiKey('')
                        await loadData()
                      } else {
                        toast.error('Failed to save API key')
                      }
                    } catch (error) {
                      toast.error(`Error: ${error}`)
                    }
                  }}>
                  Save
                </Button>
              </div>
            ) : null}
          </div>

          {/* Models List */}
          {loading ? (
            <div className="space-y-3">
              <Skeleton className="h-20 rounded-2xl" />
              <Skeleton className="h-20 rounded-2xl" />
              <Skeleton className="h-20 rounded-2xl" />
            </div>
          ) : filteredModels.length === 0 ? (
            <Card>
              <motion.div
                initial={{ opacity: 0 }}
                animate={{ opacity: 1 }}
                className="py-16 text-center">
                <div className="text-6xl mb-4">꩜</div>
                <div className="text-xl font-semibold text-white mb-2">No Models Yet</div>
                <div className="text-slate-400 mb-6">
                  Add your first {selectedProviderInfo?.name} model to get started
                  <Button
                    variant="secondary"
                    className="w-full justify-start font-normal text-slate-400 hover:text-white"
                    onClick={() => {
                      setEditingModel(null)
                      setModelName('')
                      setShowModal(true)
                    }}
                    icon={<span className="text-xl">+</span>}>
                    Add Custom Model
                  </Button>
                </div>
              </motion.div>
            </Card>
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
                    transition={{ delay: i * 0.03 }}>

                    <Card hover className="group">
                      <div className="flex items-center justify-between">
                        <div className="flex items-center gap-4">
                          <div className="w-12 h-12 rounded-xl bg-gradient-to-br from-cyan-400/20 to-cyan-600/20 border border-cyan-400/50 flex items-center justify-center text-2xl flex-shrink-0">
                            ꩜
                          </div>

                          <div>
                            <div className="flex items-center gap-2 mb-1">
                              <div className="font-semibold text-white text-lg">
                                {model.name}
                              </div>
                              <Badge
                                variant={
                                  model.health_status === 'healthy' ? 'success' :
                                    model.health_status === 'warning' ? 'warning' : 'danger'
                                }
                                size="sm">
                                {model.health_status === 'healthy' ? 'Active' :
                                  model.health_status === 'warning' ? 'Warning' : 'Down'}
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
                            className="text-cyan-400 hover:text-cyan-300">
                            🔄 Refresh
                          </Button>
                          <Button
                            variant="ghost"
                            size="sm"
                            onClick={() => handleEdit(model)}>
                            Edit
                          </Button>
                          <Button
                            variant="ghost"
                            size="sm"
                            onClick={() => handleDelete(model)}
                            className="text-red-400 hover:text-red-300">
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
            icon={<span>🏷️</span>}
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
              <option value="text">📝 Text (Chat/Completion)</option>
              <option value="multimodal">🌐 Multimodal (Text + Image Output)</option>
              <option value="image">🎨 Image (Generation Only)</option>
              <option value="video">🎬 Video (Generation)</option>
              <option value="audio">🎤 Audio (Text-to-Speech)</option>
              <option value="speech-to-text">🎙️ Speech-to-Text</option>
              <option value="speech-to-speech">🔄 Speech-to-Speech</option>
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
                icon={<span>🌐</span>}
                helperText={selectedProvider === 'azure' ? (isConfigured ? "Override provider endpoint (optional)" : "Your Azure OpenAI resource URL") : "URL of your OpenAI-compatible endpoint"}
                required={selectedProvider === 'selfhosted' || (!editingModel && !isConfigured)}
              />

              {selectedProvider === 'azure' && (
                <Input
                  label="API Version"
                  value={apiVersion}
                  onChange={(e) => setApiVersion(e.target.value)}
                  placeholder="2024-12-01-preview"
                  icon={<span>📅</span>}
                  helperText="Azure API version"
                />
              )}

              <Input
                label={`API Key ${editingModel ? '(keep existing)' : isConfigured ? '(override)' : selectedProvider === 'selfhosted' ? '(optional)' : '*'}`}
                type="password"
                value={apiKey}
                onChange={(e) => setApiKey(e.target.value)}
                placeholder={editingModel ? 'Leave blank to keep existing' : '...'}
                icon={<span>🔑</span>}
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
              icon={<span>🔑</span>}
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
              className="flex-1">
              {editingModel ? 'Update Model' : 'Create Model'}
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
            icon={<span>🔑</span>}
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
          <p className="text-slate-300 mb-6">
            ⚠️ This provider will stop working until a new API key is provided.
          </p>
          <div className="flex justify-end gap-3">
            <Button variant="ghost" onClick={() => setShowDeleteKeyModal(false)} disabled={isDeletingParams}>Cancel</Button>
            <Button variant="danger" onClick={confirmDeleteKey} disabled={isDeletingParams}>
              {isDeletingParams ? 'Deleting...' : 'Delete Key'}
            </Button>
          </div>
        </div>
      </Modal>
    </div>
  )
}
