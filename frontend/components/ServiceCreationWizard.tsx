'use client'

import { useState } from 'react'
import { motion, AnimatePresence } from 'framer-motion'
import { Button } from '@/components/ui'
import Image from 'next/image'
import { ServiceTypeCard } from '@/components/ServiceTypeCard'
import { ModalitySelector } from '@/components/ModalitySelector'

interface ServiceWizardProps {
    onComplete: (config: ServiceConfig) => void
    onCancel: () => void
    allModels: any[]
    mcpServers?: any[]
}

interface ServiceConfig {
    name: string
    serviceType: string
    poolType?: string
    inputModalities?: string[]
    outputModalities?: string[]
    description?: string
    strategy: string
    guardrails?: string
    selectedModelIds: string[]
    selectedMcpServerIds?: string[]
}

const STEPS = [
    { id: 1, title: 'Service Type', icon: '/logos/pool.png' },
    { id: 2, title: 'Pool Config', icon: '⚙️' },
    { id: 3, title: 'Modalities', icon: '🎨' },
    { id: 4, title: 'Details', icon: '📝' },
    { id: 5, title: 'Models', icon: '/logos/agentic.png' },
    { id: 6, title: 'Review', icon: '✅' }
]

export function ServiceCreationWizard({ onComplete, onCancel, allModels, mcpServers = [] }: ServiceWizardProps) {
    const [currentStep, setCurrentStep] = useState(1)
    const [config, setConfig] = useState<ServiceConfig>({
        name: '',
        serviceType: 'POOL',
        poolType: 'SINGLE_MODALITY',
        inputModalities: ['text'],
        outputModalities: ['text'],
        description: '',
        strategy: 'weighted_random',
        guardrails: '',
        selectedModelIds: [],
        selectedMcpServerIds: []
    })

    const updateConfig = (updates: Partial<ServiceConfig>) => {
        setConfig(prev => ({ ...prev, ...updates }))
    }

    const canProceed = () => {
        switch (currentStep) {
            case 1: return !!config.serviceType
            case 2: return config.serviceType !== 'POOL' || !!config.poolType
            case 3: return (config.inputModalities?.length || 0) > 0 && (config.outputModalities?.length || 0) > 0
            case 4: return !!config.name && config.name.length > 0
            case 5: return config.selectedModelIds.length > 0
            default: return true
        }
    }

    const nextStep = () => {
        if (currentStep === 2 && config.serviceType === 'AGENTIC') {
            setCurrentStep(4) // Skip modalities for AGENTIC
        } else if (currentStep < 6) {
            setCurrentStep(currentStep + 1)
        }
    }

    const prevStep = () => {
        if (currentStep === 4 && config.serviceType === 'AGENTIC') {
            setCurrentStep(2) // Skip back over modalities
        } else if (currentStep > 1) {
            setCurrentStep(currentStep - 1)
        }
    }

    const handleComplete = () => {
        onComplete(config)
    }

    return (
        <div className="space-y-6">
            {/* Progress Steps */}
            <div className="flex items-center justify-between">
                {STEPS.map((step, idx) => {
                    // Skip modalities step for AGENTIC services
                    if (step.id === 3 && config.serviceType === 'AGENTIC') return null

                    const isActive = step.id === currentStep
                    const isCompleted = step.id < currentStep
                    const isAccessible = step.id <= currentStep

                    return (
                        <div key={step.id} className="flex items-center flex-1">
                            <div className="flex flex-col items-center flex-1">
                                <motion.div
                                    whileHover={isAccessible ? { scale: 1.1 } : {}}
                                    onClick={() => isAccessible && setCurrentStep(step.id)}
                                    className={`w-10 h-10 rounded-full flex items-center justify-center text-lg font-bold transition-all cursor-pointer overflow-hidden ${isActive
                                        ? 'bg-cyan-400 text-black shadow-lg shadow-cyan-400/50'
                                        : isCompleted
                                            ? 'bg-green-400 text-black'
                                            : 'bg-white/10 text-slate-400 p-2'
                                        }`}>
                                    {isCompleted ? '✓' : (
                                        typeof step.icon === 'string' && step.icon.startsWith('/') ? (
                                            <div className="w-full h-full relative p-3">
                                                <Image
                                                    src={step.icon}
                                                    alt={step.title}
                                                    fill
                                                    className="object-contain"
                                                />
                                            </div>
                                        ) : step.icon
                                    )}
                                </motion.div>
                                <div className={`text-[10px] mt-1 font-medium ${isActive ? 'text-cyan-400' : 'text-slate-500'}`}>
                                    {step.title}
                                </div>
                            </div>
                            {idx < STEPS.filter(s => s.id !== 3 || config.serviceType !== 'AGENTIC').length - 1 && (
                                <div className={`h-0.5 flex-1 mx-2 transition-colors ${isCompleted ? 'bg-green-400' : 'bg-white/10'
                                    }`} />
                            )}
                        </div>
                    )
                })}
            </div>

            {/* Step Content */}
            <AnimatePresence mode="wait">
                <motion.div
                    key={currentStep}
                    initial={{ opacity: 0, x: 20 }}
                    animate={{ opacity: 1, x: 0 }}
                    exit={{ opacity: 0, x: -20 }}
                    transition={{ duration: 0.2 }}
                    className="min-h-[400px]">

                    {/* Step 1: Service Type */}
                    {currentStep === 1 && (
                        <div className="space-y-4">
                            <div className="text-center mb-6">
                                <h3 className="text-xl font-bold text-white mb-2">Choose Your Service Type</h3>
                                <p className="text-sm text-slate-400">Select how you want to route requests to models</p>
                            </div>
                            <div className="grid grid-cols-2 gap-4">
                                <ServiceTypeCard
                                    icon="/logos/pool.png"
                                    title="POOL"
                                    description="Load balance requests across multiple models using routing strategies"
                                    selected={config.serviceType === 'POOL'}
                                    onClick={() => updateConfig({ serviceType: 'POOL' })}
                                    color="cyan"
                                />
                                <ServiceTypeCard
                                    icon="/logos/agentic.png"
                                    title="AGENTIC"
                                    description="AI-driven orchestration with intelligent model selection and planning"
                                    selected={config.serviceType === 'AGENTIC'}
                                    onClick={() => updateConfig({ serviceType: 'AGENTIC' })}
                                    color="purple"
                                    badge="SOON"
                                />
                            </div>
                        </div>
                    )}

                    {/* Step 2: Pool Configuration */}
                    {currentStep === 2 && config.serviceType === 'POOL' && (
                        <div className="space-y-4">
                            <div className="text-center mb-6">
                                <h3 className="text-xl font-bold text-white mb-2">Pool Configuration</h3>
                                <p className="text-sm text-slate-400">How should models in your pool handle different types of data?</p>
                            </div>
                            <div className="grid grid-cols-2 gap-4">
                                <ServiceTypeCard
                                    icon="📝"
                                    title="Single Modality"
                                    description="All models handle the same input and output types (e.g., text → text). Simpler routing."
                                    selected={config.poolType === 'SINGLE_MODALITY'}
                                    onClick={() => updateConfig({ poolType: 'SINGLE_MODALITY' })}
                                    color="green"
                                />
                                <ServiceTypeCard
                                    icon="🎨"
                                    title="Multi Modality"
                                    description="Models support different input types (text, images, audio). Advanced routing based on request type."
                                    selected={config.poolType === 'MULTI_MODALITY'}
                                    onClick={() => updateConfig({ poolType: 'MULTI_MODALITY' })}
                                    color="purple"
                                />
                            </div>
                        </div>
                    )}

                    {/* Step 3: Modalities */}
                    {currentStep === 3 && (
                        <div className="space-y-6">
                            <div className="text-center mb-6">
                                <h3 className="text-xl font-bold text-white mb-2">Configure Modalities</h3>
                                <p className="text-sm text-slate-400">Select which types of input and output your models will support</p>
                            </div>
                            <ModalitySelector
                                label="Input Modalities"
                                selectedModalities={config.inputModalities || []}
                                onChange={(mods) => updateConfig({ inputModalities: mods })}
                                color="cyan"
                            />
                            <ModalitySelector
                                label="Output Modalities"
                                selectedModalities={config.outputModalities || []}
                                onChange={(mods) => updateConfig({ outputModalities: mods })}
                                color="purple"
                            />
                        </div>
                    )}

                    {/* Step 4: Service Details */}
                    {currentStep === 4 && (
                        <div className="space-y-4">
                            <div className="text-center mb-6">
                                <h3 className="text-xl font-bold text-white mb-2">Service Details</h3>
                                <p className="text-sm text-slate-400">Configure basic information and routing strategy</p>
                            </div>

                            <div>
                                <label className="block text-sm font-medium text-slate-400 mb-2">
                                    Service Name *
                                </label>
                                <input
                                    type="text"
                                    value={config.name}
                                    onChange={(e) => updateConfig({ name: e.target.value })}
                                    placeholder="my-chat-service"
                                    className="w-full px-4 py-3 bg-black border border-white/10 rounded-xl text-white focus:border-cyan-400 focus:ring-4 focus:ring-cyan-400/20 outline-none"
                                />
                            </div>

                            <div>
                                <label className="block text-sm font-medium text-slate-400 mb-2">
                                    Description (Optional)
                                </label>
                                <textarea
                                    value={config.description}
                                    onChange={(e) => updateConfig({ description: e.target.value })}
                                    placeholder="Describe what this service does..."
                                    rows={3}
                                    className="w-full px-4 py-3 bg-black border border-white/10 rounded-xl text-white focus:border-cyan-400 focus:ring-4 focus:ring-cyan-400/20 outline-none resize-none"
                                />
                            </div>

                            <div>
                                <label className="block text-sm font-medium text-slate-400 mb-3">
                                    Routing Strategy
                                </label>
                                <div className="grid grid-cols-2 gap-3">
                                    {(config.poolType === 'MULTI_MODALITY' ? [
                                        { value: 'none', label: 'None', desc: 'Managed per-modality', icon: '🔘' }
                                    ] : [
                                        { value: 'health', label: 'Health', desc: 'Prioritize healthiest', icon: '🏥' },
                                        { value: 'least_cost', label: 'Cost', desc: 'Lowest price first', icon: '💰' },
                                        { value: 'least_latency', label: 'Speed', desc: 'Lowest latency first', icon: '⚡' },
                                        { value: 'weighted_random', label: 'Weight', desc: 'Custom distribution', icon: '⚖️' }
                                    ]).map(strategy => (
                                        <motion.button
                                            key={strategy.value}
                                            type="button"
                                            whileHover={{ scale: 1.02 }}
                                            whileTap={{ scale: 0.98 }}
                                            onClick={() => updateConfig({ strategy: strategy.value })}
                                            className={`p-3 rounded-lg border-2 text-left transition-all ${config.strategy === strategy.value || (config.poolType === 'MULTI_MODALITY' && strategy.value === 'none')
                                                ? 'border-cyan-400 bg-cyan-400/10'
                                                : 'border-white/10 bg-white/5 hover:border-white/20'
                                                }`}>
                                            <div className="flex items-center gap-2 mb-1">
                                                <span className="text-xl">{strategy.icon}</span>
                                                <span className="font-semibold text-white text-sm">{strategy.label}</span>
                                            </div>
                                            <div className="text-xs text-slate-400">{strategy.desc}</div>
                                        </motion.button>
                                    ))}
                                </div>
                            </div>
                        </div>
                    )}

                    {/* Step 5: Model Selection */}
                    {currentStep === 5 && (
                        <div className="space-y-4">
                            <div className="text-center mb-6">
                                <h3 className="text-xl font-bold text-white mb-2">
                                    {config.serviceType === 'AGENTIC' ? 'Select Models & MCP Servers' : 'Select Models'}
                                </h3>
                                <p className="text-sm text-slate-400">
                                    {config.serviceType === 'AGENTIC'
                                        ? 'Choose models and MCP servers for your agentic service'
                                        : 'Choose which models will be part of this service pool'}
                                </p>
                            </div>

                            {/* Models Section */}
                            <div>
                                <h4 className="text-sm font-semibold text-slate-300 mb-3 flex items-center gap-2">
                                    <span className="text-lg">🤖</span> Models
                                </h4>
                                {allModels.length === 0 ? (
                                    <div className="p-6 text-center border border-amber-400/30 bg-amber-400/10 rounded-xl">
                                        <div className="text-3xl mb-2">⚠️</div>
                                        <div className="text-amber-400 font-semibold mb-1">No Models Available</div>
                                        <div className="text-sm text-amber-400/70">Please add models in the Providers section first</div>
                                    </div>
                                ) : (
                                    <div className="space-y-2 max-h-[200px] overflow-y-auto pr-2">
                                        {allModels.map(model => (
                                            <motion.label
                                                key={model.id}
                                                whileHover={{ scale: 1.01 }}
                                                className={`flex items-center gap-4 p-3 rounded-xl border-2 cursor-pointer transition-all ${config.selectedModelIds.includes(model.id)
                                                    ? 'border-cyan-400 bg-cyan-400/10'
                                                    : 'border-white/10 bg-white/5 hover:bg-white/10'
                                                    }`}>
                                                <input
                                                    type="checkbox"
                                                    checked={config.selectedModelIds.includes(model.id)}
                                                    onChange={(e) => {
                                                        if (e.target.checked) {
                                                            updateConfig({ selectedModelIds: [...config.selectedModelIds, model.id] })
                                                        } else {
                                                            updateConfig({ selectedModelIds: config.selectedModelIds.filter(id => id !== model.id) })
                                                        }
                                                    }}
                                                    className="rounded border-white/20 bg-black text-cyan-400 focus:ring-cyan-400/50"
                                                />
                                                <div className="flex-1">
                                                    <div className="font-semibold text-white">{model.name}</div>
                                                    <div className="text-xs text-slate-400 capitalize">{model.modality} • {model.provider_id}</div>
                                                </div>
                                            </motion.label>
                                        ))}
                                    </div>
                                )}
                            </div>

                            {/* MCP Servers Section - Only for AGENTIC */}
                            {config.serviceType === 'AGENTIC' && (
                                <div className="mt-6 pt-6 border-t border-white/10">
                                    <h4 className="text-sm font-semibold text-slate-300 mb-3 flex items-center gap-2">
                                        <span className="text-lg">🔧</span> MCP Servers
                                        <span className="text-xs px-2 py-0.5 bg-purple-400/20 text-purple-400 rounded-full">Optional</span>
                                    </h4>
                                    {mcpServers.length === 0 ? (
                                        <div className="p-4 text-center border border-white/10 bg-white/5 rounded-xl">
                                            <div className="text-2xl mb-2">🔌</div>
                                            <div className="text-slate-400 text-sm">No MCP servers configured</div>
                                            <div className="text-xs text-slate-500 mt-1">Add MCP servers in the Providers → MCP section</div>
                                        </div>
                                    ) : (
                                        <div className="space-y-2 max-h-[150px] overflow-y-auto pr-2">
                                            {mcpServers.map(server => (
                                                <motion.label
                                                    key={server.id}
                                                    whileHover={{ scale: 1.01 }}
                                                    className={`flex items-center gap-4 p-3 rounded-xl border-2 cursor-pointer transition-all ${(config.selectedMcpServerIds || []).includes(server.id)
                                                        ? 'border-purple-400 bg-purple-400/10'
                                                        : 'border-white/10 bg-white/5 hover:bg-white/10'
                                                        }`}>
                                                    <input
                                                        type="checkbox"
                                                        checked={(config.selectedMcpServerIds || []).includes(server.id)}
                                                        onChange={(e) => {
                                                            const currentIds = config.selectedMcpServerIds || []
                                                            if (e.target.checked) {
                                                                updateConfig({ selectedMcpServerIds: [...currentIds, server.id] })
                                                            } else {
                                                                updateConfig({ selectedMcpServerIds: currentIds.filter(id => id !== server.id) })
                                                            }
                                                        }}
                                                        className="rounded border-white/20 bg-black text-purple-400 focus:ring-purple-400/50"
                                                    />
                                                    <div className="flex-1">
                                                        <div className="font-semibold text-white flex items-center gap-2">
                                                            {server.name}
                                                            <span className={`w-2 h-2 rounded-full ${server.status === 'connected' ? 'bg-green-400' : 'bg-slate-400'}`} />
                                                        </div>
                                                        <div className="text-xs text-slate-400">{server.server_type} • {server.status}</div>
                                                    </div>
                                                </motion.label>
                                            ))}
                                        </div>
                                    )}
                                </div>
                            )}

                            {/* Selection Summary */}
                            <div className="flex gap-4 justify-center text-sm pt-2">
                                {config.selectedModelIds.length > 0 && (
                                    <div className="text-cyan-400">
                                        ✓ {config.selectedModelIds.length} model{config.selectedModelIds.length > 1 ? 's' : ''} selected
                                    </div>
                                )}
                                {config.serviceType === 'AGENTIC' && (config.selectedMcpServerIds || []).length > 0 && (
                                    <div className="text-purple-400">
                                        ✓ {(config.selectedMcpServerIds || []).length} MCP server{(config.selectedMcpServerIds || []).length > 1 ? 's' : ''} selected
                                    </div>
                                )}
                            </div>
                        </div>
                    )}

                    {/* Step 6: Review */}
                    {currentStep === 6 && (
                        <div className="space-y-4">
                            <div className="text-center mb-6">
                                <h3 className="text-xl font-bold text-white mb-2">Review & Create</h3>
                                <p className="text-sm text-slate-400">Confirm your service configuration</p>
                            </div>

                            <div className="space-y-3">
                                <div className="glass rounded-xl p-4">
                                    <div className="text-xs text-slate-500 uppercase tracking-wider mb-2">Service Name</div>
                                    <div className="text-white font-semibold">{config.name}</div>
                                </div>

                                <div className="glass rounded-xl p-4">
                                    <div className="text-xs text-slate-500 uppercase tracking-wider mb-2">Configuration</div>
                                    <div className="flex gap-2">
                                        <span className="px-3 py-1 bg-cyan-400/20 text-cyan-400 rounded-lg text-sm font-medium">
                                            {config.serviceType}
                                        </span>
                                        {config.poolType && (
                                            <span className="px-3 py-1 bg-purple-400/20 text-purple-400 rounded-lg text-sm font-medium">
                                                {config.poolType}
                                            </span>
                                        )}
                                        <span className="px-3 py-1 bg-green-400/20 text-green-400 rounded-lg text-sm font-medium capitalize">
                                            {config.strategy.replace('_', ' ')}
                                        </span>
                                    </div>
                                </div>

                                {(config.inputModalities || config.outputModalities) && (
                                    <div className="glass rounded-xl p-4">
                                        <div className="text-xs text-slate-500 uppercase tracking-wider mb-2">Modalities</div>
                                        <div className="space-y-2">
                                            {config.inputModalities && config.inputModalities.length > 0 && (
                                                <div className="flex items-center gap-2 text-sm">
                                                    <span className="text-slate-400">Input:</span>
                                                    <div className="flex gap-1">
                                                        {config.inputModalities.map(mod => (
                                                            <span key={mod} className="px-2 py-0.5 bg-cyan-400/10 text-cyan-400 rounded-md capitalize">
                                                                {mod}
                                                            </span>
                                                        ))}
                                                    </div>
                                                </div>
                                            )}
                                            {config.outputModalities && config.outputModalities.length > 0 && (
                                                <div className="flex items-center gap-2 text-sm">
                                                    <span className="text-slate-400">Output:</span>
                                                    <div className="flex gap-1">
                                                        {config.outputModalities.map(mod => (
                                                            <span key={mod} className="px-2 py-0.5 bg-purple-400/10 text-purple-400 rounded-md capitalize">
                                                                {mod}
                                                            </span>
                                                        ))}
                                                    </div>
                                                </div>
                                            )}
                                        </div>
                                    </div>
                                )}

                                <div className="glass rounded-xl p-4">
                                    <div className="text-xs text-slate-500 uppercase tracking-wider mb-2">Models</div>
                                    <div className="text-white">
                                        {config.selectedModelIds.length} model{config.selectedModelIds.length > 1 ? 's' : ''} selected
                                    </div>
                                </div>

                                {config.serviceType === 'AGENTIC' && (config.selectedMcpServerIds || []).length > 0 && (
                                    <div className="glass rounded-xl p-4">
                                        <div className="text-xs text-slate-500 uppercase tracking-wider mb-2">MCP Servers</div>
                                        <div className="text-white">
                                            {(config.selectedMcpServerIds || []).length} MCP server{(config.selectedMcpServerIds || []).length > 1 ? 's' : ''} selected
                                        </div>
                                    </div>
                                )}
                            </div>
                        </div>
                    )}
                </motion.div>
            </AnimatePresence>

            {/* Navigation */}
            <div className="flex gap-3 pt-4 border-t border-white/10">
                <Button
                    variant="secondary"
                    onClick={onCancel}
                    className="flex-1">
                    Cancel
                </Button>
                {currentStep > 1 && (
                    <Button
                        variant="secondary"
                        onClick={prevStep}>
                        ← Back
                    </Button>
                )}
                {currentStep < 6 ? (
                    <Button
                        variant="primary"
                        onClick={nextStep}
                        disabled={!canProceed()}
                        className="flex-1">
                        Next →
                    </Button>
                ) : (
                    <Button
                        variant="primary"
                        onClick={handleComplete}
                        className="flex-1">
                        Create Service
                    </Button>
                )}
            </div>
        </div>
    )
}
