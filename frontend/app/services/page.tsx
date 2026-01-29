'use client'

import { useEffect, useState } from 'react'
import { motion } from 'framer-motion'
import { Button, Card, Modal, Input, Badge } from '@/components/ui'
import { ServiceCreationWizard } from '@/components/ServiceCreationWizard'
import { RichPromptEditor } from '@/components/RichPromptEditor'
import { toast } from 'sonner'
import Image from 'next/image'
import Link from 'next/link'

interface Service {
    id?: string
    name: string
    service_type: string
    description?: string
    strategy: string
    guardrails?: string
    pool_type?: string
    input_modalities?: string[]
    output_modalities?: string[]
    planner_model_id?: string
}

interface ServiceModel {
    model_id: string
    model_name: string
    modality: string
    position: number
    weight: number
    is_healthy?: boolean
    rtcros: {
        role?: string
        task?: string
        context?: string
        reasoning?: string
        output?: string
        stop?: string
    }
}



export default function ServicesPage() {
    const [services, setServices] = useState<Service[]>([])
    const [allModels, setAllModels] = useState<any[]>([])
    const [serviceModels, setServiceModels] = useState<ServiceModel[]>([])
    const [loading, setLoading] = useState(true)
    const [showModal, setShowModal] = useState(false)
    const [showModelsModal, setShowModelsModal] = useState(false)
    const [selectedService, setSelectedService] = useState<Service | null>(null)
    const [editingService, setEditingService] = useState<Service | null>(null)

    // Form
    const [name, setName] = useState('')
    const [serviceType, setServiceType] = useState('POOL')
    const [poolType, setPoolType] = useState('SINGLE_MODALITY')
    const [inputModalities, setInputModalities] = useState<string[]>(['text'])
    const [outputModalities, setOutputModalities] = useState<string[]>(['text'])
    const [description, setDescription] = useState('')
    const [strategy, setStrategy] = useState('weighted_random')
    const [guardrails, setGuardrails] = useState('')
    const [selectedModelIds, setSelectedModelIds] = useState<string[]>([])

    // Delete Modal
    const [showDeleteModal, setShowDeleteModal] = useState(false)
    const [serviceToDelete, setServiceToDelete] = useState<Service | null>(null)
    const [isDeleting, setIsDeleting] = useState(false)

    // Remove Model/Server Modals
    const [showRemoveModelModal, setShowRemoveModelModal] = useState(false)
    const [modelToRemove, setModelToRemove] = useState<any>(null)
    const [showRemoveServerModal, setShowRemoveServerModal] = useState(false)
    const [serverToRemove, setServerToRemove] = useState<any>(null)
    const [isRemoving, setIsRemoving] = useState(false)

    // Agentic service fields
    const [plannerModelId, setPlannerModelId] = useState('')
    const [systemPrompt, setSystemPrompt] = useState(`[ROLE]\n\n[TASK]\n\n[CONTEXT]\n\n[REASONING]\n\n[OUTPUT]\n\n[STOP]`)
    const [maxIterations, setMaxIterations] = useState(10)
    const [showToolsModal, setShowToolsModal] = useState(false)
    // New MCP State
    const [activeToolsTab, setActiveToolsTab] = useState<'models' | 'mcp'>('models')
    const [allMcpServers, setAllMcpServers] = useState<any[]>([])
    const [assignedMcpServers, setAssignedMcpServers] = useState<any[]>([])
    const [selectedMcpServerIdsForCreate, setSelectedMcpServerIdsForCreate] = useState<string[]>([])

    // Model Config Modal
    const [showConfigModal, setShowConfigModal] = useState(false)
    const [editingModel, setEditingModel] = useState<ServiceModel | null>(null)
    const [configForm, setConfigForm] = useState({
        weight: 100,
        position: 1,
        rtcros: {
            role: '',
            task: '',
            context: '',
            reasoning: '',
            output: '',
            stop: ''
        }
    })
    const [rtcrosText, setRtcrosText] = useState(`[ROLE]\n\n[TASK]\n\n[CONTEXT]\n\n[REASONING]\n\n[OUTPUT]\n\n[STOP]`)

    // Helper to parse RTCROS raw text into structured object
    const parseRtcrosText = (text: string) => {
        const sections = { role: '', task: '', context: '', reasoning: '', output: '', stop: '' }
        const regex = /\[(ROLE|TASK|CONTEXT|REASONING|OUTPUT|STOP)\]([\s\S]*?)(?=\[(?:ROLE|TASK|CONTEXT|REASONING|OUTPUT|STOP)\]|$)/gi
        let match
        while ((match = regex.exec(text)) !== null) {
            const key = match[1].toLowerCase() as keyof typeof sections
            if (key in sections) {
                sections[key] = match[2].trim()
            }
        }
        return sections
    }

    // Snippet Modal
    const [showSnippetModal, setShowSnippetModal] = useState(false)
    const [snippetService, setSnippetService] = useState<Service | null>(null)

    const handleShowSnippet = (service: Service) => {
        setSnippetService(service)
        setShowSnippetModal(true)
    }

    useEffect(() => {
        loadData()
    }, [])

    const loadData = async () => {
        setLoading(true)
        try {
            const [servicesRes, modelsRes] = await Promise.all([
                fetch('/v1/user/services', { credentials: 'include' }),
                fetch('/v1/user/models', { credentials: 'include' }),
            ])

            if (servicesRes.ok) {
                const rawServices: Service[] = await servicesRes.json()
                // Normalize types to uppercase to ensure consistent rendering
                const normalizedServices = rawServices.map(s => ({
                    ...s,
                    service_type: (s.service_type || 'POOL').toUpperCase(),
                    pool_type: (s.pool_type || '').toUpperCase()
                }))
                setServices(normalizedServices)
            } else {
                const err = await servicesRes.text()
                toast.error(`Failed to load services (${servicesRes.status}): ${err}`)
            }
            if (modelsRes.ok) setAllModels(await modelsRes.json())

            setLoading(false)
        } catch (error) {
            toast.error('Failed to load data')
            setLoading(false)
        }
    }

    const loadServiceModels = async (serviceId: string) => {
        try {
            const res = await fetch(`/v1/services/${serviceId}/models`, { credentials: 'include' })
            if (res.ok) {
                const data = await res.json()
                setServiceModels(data)
            }
        } catch (error) {
            console.error('Failed to load service models:', error)
        }
    }

    // Provider Logo Mapping
    const LOGO_MAP: Record<string, string> = {
        'openai': '/logos/openai.png',
        'anthropic': '/logos/anthropic.png',
        'google': '/logos/gemini.png',
        'azure': '/logos/azure.png',
        'meta': '/logos/meta.png',
        'mistral': '/logos/mistral.png',
        'cohere': '/logos/cohere.png',
        'stability': '/logos/stability.png',
        'elevenlabs': '/logos/elevenlabs.png',
        'local': '/logos/ollama.png',
    }

    // Prepare mentions for RichEditor
    // Only show models that are selected for this service
    const mentionModels = allModels.filter(m => selectedModelIds.includes(m.id))

    const mentions = mentionModels.map(m => {
        // Try to guess provider from name or id if provider field is missing
        const providerGuess = m.provider ||
            (m.id.includes('gpt') || m.id.includes('dall') ? 'openai' :
                m.id.includes('claude') ? 'anthropic' :
                    m.id.includes('gemini') ? 'google' :
                        m.id.includes('llama') ? 'meta' :
                            'local')

        return {
            id: m.id,
            label: m.name,
            type: 'model' as const,
            icon: (m.modality || '').includes('image') ? '🖼️' : (m.modality || '').includes('audio') ? '🎙️' : '🤖',
            logo: LOGO_MAP[providerGuess.toLowerCase()]
        }
    })

    const handleSubmit = async (e: React.FormEvent) => {
        e.preventDefault()

        // Require at least one model for new services
        if (!editingService && selectedModelIds.length === 0) {
            toast.error('Please select at least one model for this service')
            return
        }

        // VALIDATION: Single Modality Constraint
        // Ensures that if user selects "Single Modality", all assigned models actually share the same modality.
        if (serviceType === 'POOL' && poolType === 'SINGLE_MODALITY' && selectedModelIds.length > 0) {
            const distinctModalities = new Set<string>()

            selectedModelIds.forEach(id => {
                const model = allModels.find(m => m.id === id)
                if (model && model.modality) {
                    // Simple normalization
                    distinctModalities.add(model.modality.toLowerCase())
                }
            })

            if (distinctModalities.size > 1) {
                toast.error(`Validation Failed: Single Modality service cannot differ in model types. Found: ${Array.from(distinctModalities).join(', ')}. Please select Multi-Modality or remove conflicting models.`)
                return
            }
        }

        const data = {
            name,
            service_type: serviceType,
            description: description || undefined,
            strategy: serviceType === 'AGENTIC' ? 'planner' : (serviceType === 'POOL' && poolType === 'MULTI_MODALITY' ? 'none' : strategy),
            guardrails: guardrails ? guardrails.split(',').map(g => g.trim()).filter(g => g) : [],

            // AGENTIC-specific fields
            ...(serviceType === 'AGENTIC' && {
                planner_model_id: plannerModelId,
                system_prompt: systemPrompt || undefined,
                max_iterations: maxIterations,
            }),

            // POOL-specific fields
            ...(serviceType === 'POOL' && {
                pool_type: poolType,
            }),
        }

        try {
            const url = editingService ? `/v1/services/${editingService.name}` : '/v1/services'
            const method = editingService ? 'PUT' : 'POST'

            const res = await fetch(url, {
                method,
                headers: { 'Content-Type': 'application/json' },
                credentials: 'include',
                body: JSON.stringify(data),
            })

            if (res.ok) {
                const createdService = await res.json()

                // If creating new service, assign selected models
                if (!editingService && selectedModelIds.length > 0) {
                    // Calculate equal weight distribution (total must not exceed 100)
                    const weight = Math.floor(100 / selectedModelIds.length)

                    for (let i = 0; i < selectedModelIds.length; i++) {
                        const model = allModels.find((m: any) => m.id === selectedModelIds[i])
                        const res = await fetch(`/v1/services/${createdService.name}/models`, {
                            method: 'POST',
                            headers: { 'Content-Type': 'application/json' },
                            credentials: 'include',
                            body: JSON.stringify({
                                model_id: selectedModelIds[i],
                                modality: model?.modality || 'text',
                                position: i + 1,
                                weight,
                            }),
                        })
                        if (!res.ok) {
                            const errorMsg = await res.text()
                            toast.error(`Failed to assign ${model?.name || 'model'}: ${errorMsg}`)
                        }
                    }
                }

                // If creating new AGENTIC service, assign selected MCP servers
                if (!editingService && serviceType === 'AGENTIC' && selectedMcpServerIdsForCreate.length > 0) {
                    for (const serverId of selectedMcpServerIdsForCreate) {
                        const res = await fetch(`/v1/services/${createdService.name}/mcp-servers/${serverId}`, {
                            method: 'POST',
                            credentials: 'include',
                        })
                        if (!res.ok) {
                            const errorMsg = await res.text()
                            const server = allMcpServers.find(s => s.id === serverId)
                            toast.error(`Failed to assign MCP server ${server?.name || serverId}: ${errorMsg}`)
                        }
                    }
                }

                toast.success(editingService ? 'Service updated' : 'Service created with models')
                setShowModal(false)
                resetForm()
                await loadData()
            } else {
                const error = await res.text()
                toast.error(`Failed: ${error}`)
            }
        } catch (error) {
            toast.error(`Error: ${error}`)
        }
    }

    const handleDelete = (service: Service) => {
        setServiceToDelete(service)
        setShowDeleteModal(true)
    }

    const confirmDelete = async () => {
        if (!serviceToDelete) return
        setIsDeleting(true)
        try {
            const res = await fetch(`/v1/services/${serviceToDelete.name}`, {
                method: 'DELETE',
                credentials: 'include',
            })

            if (res.ok) {
                toast.success('Service deleted')
                await loadData()
                setShowDeleteModal(false)
            } else {
                const errorText = await res.text()
                toast.error(`Failed to delete: ${errorText}`)
            }
        } catch (error) {
            toast.error(`Error: ${error}`)
        } finally {
            setIsDeleting(false)
        }
    }

    const handleRemoveModel = (model: any) => {
        setModelToRemove(model)
        setShowRemoveModelModal(true)
    }

    const confirmRemoveModel = async () => {
        if (!selectedService || !modelToRemove) return
        setIsRemoving(true)
        try {
            await fetch(`/v1/services/${selectedService.name}/models/${modelToRemove.model_id}`, {
                method: 'DELETE',
                credentials: 'include',
            })
            toast.success('Model removed')
            loadServiceModels(selectedService.name)
            setShowRemoveModelModal(false)
        } catch (error) {
            toast.error('Failed to remove model')
        } finally {
            setIsRemoving(false)
        }
    }

    const handleRemoveServer = (server: any) => {
        setServerToRemove(server)
        setShowRemoveServerModal(true)
    }

    const confirmRemoveServer = async () => {
        if (!selectedService || !serverToRemove) return
        setIsRemoving(true)
        try {
            const res = await fetch(`/v1/services/${selectedService.name}/mcp-servers/${serverToRemove.id}`, {
                method: 'DELETE', credentials: 'include'
            });
            if (res.ok) {
                toast.success('Server removed');
                await loadMcpServers(selectedService.name);
                setShowRemoveServerModal(false)
            } else {
                const errText = await res.text();
                toast.error(`Failed to remove: ${errText}`);
            }
        } catch (e) { toast.error('Failed to remove') }
        finally { setIsRemoving(false) }
    }

    const handleEdit = async (service: Service) => {
        setEditingService(service)
        setName(service.name)

        // Normalize Service Type (Pool -> POOL)
        const sType = (service.service_type || 'POOL').toUpperCase()
        setServiceType(sType)

        setDescription(service.description || '')
        setStrategy(service.strategy)
        // Parse Guardrails (JSON string -> CSV for input)
        let gVal = service.guardrails || ''
        try {
            const parsed = JSON.parse(gVal)
            if (Array.isArray(parsed)) {
                gVal = parsed.join(', ')
            }
        } catch {
            // Keep original value if not valid JSON array
        }
        setGuardrails(gVal)

        // Normalize Pool Type
        let pType = service.pool_type || 'SINGLE_MODALITY'
        if (pType === 'SingleModality') pType = 'SINGLE_MODALITY'
        if (pType === 'MultiModality') pType = 'MULTI_MODALITY'
        // Use Uppercase as fallback for other cases
        pType = pType.toUpperCase()
        // Ensure valid
        if (pType !== 'SINGLE_MODALITY' && pType !== 'MULTI_MODALITY') pType = 'SINGLE_MODALITY'

        setPoolType(pType)

        setPlannerModelId(service.planner_model_id || '')

        // For Agentic fields potentially missing in list view, 
        // ideally we'd fetch a detailed endpoint, but for now we rely on what we have.
        // If system_prompt/max_iterations are crucial, we should expose them in Service interface.
        // Assuming they might be on the object as extras:
        const s: any = service
        if (s.system_prompt) setSystemPrompt(s.system_prompt)
        if (s.max_iterations) setMaxIterations(s.max_iterations)

        // Load assigned models for this service
        try {
            const res = await fetch(`/v1/services/${service.name}/models`, { credentials: 'include' })
            if (res.ok) {
                const models: ServiceModel[] = await res.json()
                setSelectedModelIds(models.map(m => m.model_id))
            }
        } catch (e) {
            console.error('Failed to load service models for edit:', e)
        }

        setShowModal(true)
    }

    const handleManageModels = (service: Service) => {
        setSelectedService(service)
        if (service.service_type === 'AGENTIC') {
            setShowToolsModal(true)
            loadMcpServers(service.name)
            loadServiceModels(service.name)
        } else {
            setShowModelsModal(true)
            loadServiceModels(service.name)
        }
    }

    const loadAllMcpServers = async () => {
        try {
            const res = await fetch('/v1/mcp/servers', { credentials: 'include' })
            if (res.ok) {
                const data = await res.json()
                setAllMcpServers(data)
            }
        } catch (error) {
            console.error('Failed to load MCP servers:', error)
        }
    }

    const loadMcpServers = async (serviceName: string) => {
        try {
            // Fetch all servers
            const allRes = await fetch('/v1/mcp/servers', {
                credentials: 'include'
            })
            if (allRes.ok) {
                const data = await allRes.json()
                console.log('Loaded ALL MCP servers:', data)
                setAllMcpServers(data)
            } else {
                console.error('Failed to fetch MCP servers:', allRes.status)
            }

            // Fetch assigned servers
            const assignedRes = await fetch(`/v1/services/${serviceName}/mcp-servers`, {
                credentials: 'include'
            })
            if (assignedRes.ok) {
                const data = await assignedRes.json()
                console.log('Loaded ASSIGNED MCP servers for', serviceName, ':', data)
                setAssignedMcpServers(data)
            } else {
                console.error('Failed to fetch assigned MCP servers:', assignedRes.status, await assignedRes.text())
            }
        } catch (error) {
            toast.error('Failed to load MCP servers')
            console.error(error)
        }
    }

    const resetForm = () => {
        setEditingService(null)
        setName('')
        setServiceType('POOL')
        setPoolType('SINGLE_MODALITY')
        setInputModalities(['text'])
        setOutputModalities(['text'])
        setDescription('')
        setStrategy('weighted_random')
        setGuardrails('')
        setSelectedModelIds([])
        setSelectedMcpServerIdsForCreate([])
        // Reset agentic fields
        setPlannerModelId('')
        setSystemPrompt('')
        setMaxIterations(10)
    }

    return (
        <div className="p-8">
            <div className="max-w-7xl mx-auto">
                <motion.div
                    initial={{ opacity: 0, y: 20 }}
                    animate={{ opacity: 1, y: 0 }}
                    className="mb-8">
                    <h1 className="text-3xl font-bold gradient-text-white mb-2">
                        Services
                    </h1>
                    <p className="text-slate-400">
                        Manage service endpoints and model routing strategies
                    </p>
                </motion.div>

                <div className="flex justify-end mb-6">
                    <Button
                        variant="primary"
                        onClick={() => {
                            resetForm()
                            loadAllMcpServers()
                            setShowModal(true)
                        }}
                        icon={<span className="text-xl">+</span>}>
                        Create Service
                    </Button>
                </div>

                {loading ? (
                    <Card>
                        <div className="py-16 text-center">
                            <div className="text-slate-400">Loading...</div>
                        </div>
                    </Card>
                ) : services.length === 0 ? (
                    <Card>
                        <motion.div
                            initial={{ opacity: 0 }}
                            animate={{ opacity: 1 }}
                            className="py-16 text-center">
                            <div className="text-6xl mb-4">⚙️</div>
                            <div className="text-slate-400 mb-6">No services configured</div>
                            <Button
                                variant="secondary"
                                onClick={() => {
                                    resetForm()
                                    loadAllMcpServers()
                                    setShowModal(true)
                                }}
                                icon={<span className="text-xl">+</span>}>
                                Create Your First Service
                            </Button>
                        </motion.div>
                    </Card>
                ) : (
                    <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
                        {services.map((service, i) => {
                            return (
                                <motion.div
                                    key={service.name}
                                    initial={{ opacity: 0, y: 20 }}
                                    animate={{ opacity: 1, y: 0 }}
                                    transition={{ delay: i * 0.05 }}>
                                    <Card hover glow="cyan">
                                        <div className="flex items-start justify-between mb-4">
                                            <div className="w-12 h-12 rounded-full bg-white border border-white/20 flex items-center justify-center p-3 overflow-hidden">
                                                {service.service_type === 'POOL' ? (
                                                    <Image src="/logos/pool.png" alt="Pool" width={24} height={24} className="object-contain shrink-0" />
                                                ) : service.service_type === 'AGENTIC' ? (
                                                    <Image src="/logos/agentic.png" alt="Agentic" width={24} height={24} className="object-contain shrink-0" />
                                                ) : (
                                                    '⚙️'
                                                )}
                                            </div>
                                            <div className="flex gap-2 items-center">
                                                <button
                                                    onClick={() => handleShowSnippet(service)}
                                                    className="w-8 h-8 rounded-full bg-white/5 hover:bg-white/10 flex items-center justify-center text-slate-400 hover:text-white transition-colors"
                                                    title="View API Code Snippet"
                                                >
                                                    <span className="font-mono text-xs">&lt;/&gt;</span>
                                                </button>
                                                <Badge variant="primary" size="sm">
                                                    {service.service_type === 'AGENTIC' ? 'Agentic' : (service.service_type || 'POOL')}
                                                </Badge>
                                                {service.service_type !== 'AGENTIC' && service.pool_type && (
                                                    <Badge variant={service.pool_type.replace(/_/g, '').toUpperCase() === 'MULTIMODALITY' ? 'purple' : 'success'} size="sm">
                                                        {service.pool_type.replace(/_/g, '').toUpperCase() === 'MULTIMODALITY' ? '🎨 Multi' : '📝 Single'}
                                                    </Badge>
                                                )}
                                            </div>
                                        </div>

                                        <h3 className="text-lg font-bold text-white mb-2">
                                            {service.name}
                                        </h3>

                                        {service.description && (
                                            <p className="text-sm text-slate-400 mb-4">
                                                {service.description}
                                            </p>
                                        )}

                                        {/* Show modalities if available */}
                                        {(service.input_modalities || service.output_modalities) && (
                                            <div className="mb-4 space-y-2">
                                                {service.input_modalities && service.input_modalities.length > 0 && (
                                                    <div className="flex items-center gap-2 text-xs">
                                                        <span className="text-slate-500">Input:</span>
                                                        <div className="flex gap-1">
                                                            {service.input_modalities.map(mod => (
                                                                <span key={`input-${service.name}-${mod}`} className="px-2 py-0.5 bg-cyan-400/10 text-cyan-400 rounded-md capitalize">
                                                                    {mod}
                                                                </span>
                                                            ))}
                                                        </div>
                                                    </div>
                                                )}
                                                {service.output_modalities && service.output_modalities.length > 0 && (
                                                    <div className="flex items-center gap-2 text-xs">
                                                        <span className="text-slate-500">Output:</span>
                                                        <div className="flex gap-1">
                                                            {service.output_modalities.map(mod => (
                                                                <span key={`output-${service.name}-${mod}`} className="px-2 py-0.5 bg-purple-400/10 text-purple-400 rounded-md capitalize">
                                                                    {mod}
                                                                </span>
                                                            ))}
                                                        </div>
                                                    </div>
                                                )}
                                            </div>
                                        )}

                                        <div className="flex items-center gap-2 text-xs text-slate-500 mb-4">
                                            <span>Strategy:</span>
                                            <span className="text-cyan-400 font-medium capitalize">
                                                {service.service_type === 'AGENTIC'
                                                    ? '🧠 Planner'
                                                    : ({
                                                        'planner': '🧠 Planner',
                                                        'weighted_random': '⚖️ Weighted',
                                                        'least_cost': '💰 Cost (Lowest Price)',
                                                        'least_latency': '⚡ Speed (Lowest Latency)',
                                                        'health': '🏥 Health (Failover)',
                                                        'none': 'None (Multi-Modality)'
                                                    }[service.strategy] || service.strategy.replace('_', ' '))
                                                }
                                            </span>
                                        </div>

                                        <div className="flex gap-2 pt-4 border-t border-white/10">
                                            <Button
                                                variant="ghost"
                                                size="sm"
                                                onClick={() => handleManageModels(service)}
                                                className="flex-1">
                                                {service.service_type === 'AGENTIC' ? 'Tools' : 'Models'}
                                            </Button>
                                            <Button
                                                variant="ghost"
                                                size="sm"
                                                onClick={() => handleEdit(service)}>
                                                Edit
                                            </Button>
                                            <Button
                                                variant="ghost"
                                                size="sm"
                                                onClick={() => handleDelete(service)}
                                                className="text-red-400 hover:text-red-300">
                                                Delete
                                            </Button>
                                        </div>
                                    </Card>
                                </motion.div>
                            )
                        })}
                    </div>
                )
                }
            </div >

            {/* Create/Edit Service Modal */}
            < Modal
                isOpen={showModal}
                onClose={() => {
                    setShowModal(false)
                    resetForm()
                }}
                title={editingService ? 'Edit Service' : 'Create Service'}
                description="Configure a new service endpoint" >

                <form onSubmit={handleSubmit} className="space-y-4">
                    <Input
                        label="Service Name"
                        value={name}
                        onChange={(e) => setName(e.target.value)}
                        placeholder="my-chat-service"
                        icon={<span>🏷️</span>}
                        required
                    />

                    <div>
                        <label className="block text-sm font-medium text-slate-400 mb-3">
                            Service Type
                        </label>
                        <div className="grid grid-cols-2 gap-3">
                            <button
                                type="button"
                                onClick={() => setServiceType('POOL')}
                                className={`relative p-4 rounded-xl border-2 transition-all ${serviceType === 'POOL'
                                    ? 'border-cyan-400 bg-cyan-400/10'
                                    : 'border-white/10 bg-white/5 hover:bg-white/10'
                                    }`}>
                                <div className="mb-2 w-16 h-16 relative mx-auto bg-white rounded-full p-6 border border-white/20 overflow-hidden shadow-inner">
                                    <Image src="/logos/pool.png" alt="Pool" fill className="object-contain" />
                                </div>
                                <div className="text-white font-medium text-sm text-center">POOL</div>
                                <div className="text-xs text-slate-400 mt-1 text-center">Model Load Balancing</div>
                                {serviceType === 'POOL' && (
                                    <div className="absolute top-2 right-2 w-5 h-5 rounded-full bg-cyan-400 flex items-center justify-center">
                                        <span className="text-xs">✓</span>
                                    </div>
                                )}
                            </button>
                            <button
                                type="button"
                                onClick={() => setServiceType('AGENTIC')}
                                className={`relative p-4 rounded-xl border-2 transition-all ${serviceType === 'AGENTIC'
                                    ? 'border-purple-400 bg-purple-400/10'
                                    : 'border-white/10 bg-white/5 hover:border-white/10'
                                    }`}>
                                <div className="mb-2 w-16 h-16 relative mx-auto bg-white rounded-full p-6 border border-white/20 overflow-hidden shadow-inner">
                                    <Image src="/logos/agentic.png" alt="Agentic" fill className="object-contain" />
                                </div>
                                <div className="text-white font-medium text-sm text-center">AGENTIC</div>
                                <div className="text-xs text-slate-400 mt-1 text-center">AI Orchestration</div>

                                {serviceType === 'AGENTIC' && (
                                    <div className="absolute top-2 left-2 w-5 h-5 rounded-full bg-purple-400 flex items-center justify-center">
                                        <span className="text-xs">✓</span>
                                    </div>
                                )}
                            </button>
                        </div>
                    </div>

                    {/* Pool Type - only show for POOL services */}
                    {serviceType === 'POOL' && (
                        <div>
                            <label className="block text-sm font-medium text-slate-400 mb-2">
                                Pool Type
                            </label>
                            <select
                                value={poolType}
                                onChange={(e) => setPoolType(e.target.value)}
                                className="w-full px-4 py-3 bg-black border border-white/10 rounded-xl text-white focus:border-cyan-400 focus:ring-4 focus:ring-cyan-400/20 outline-none">
                                <option value="SINGLE_MODALITY">📝 Single Modality</option>
                                <option value="MULTI_MODALITY">🎨 Multi Modality</option>
                            </select>
                            <p className="text-xs text-slate-500 mt-1">
                                {poolType === 'SINGLE_MODALITY'
                                    ? 'Capabilities auto-detected from assigned models (e.g., text→text)'
                                    : 'Supports mixed modalities from assigned models (e.g., text→video, audio→text)'}
                            </p>
                        </div>
                    )}

                    {/* Agentic-specific fields */}
                    {serviceType === 'AGENTIC' && (
                        <>
                            {/* Planner Model Selector */}
                            <div>
                                <label className="block text-sm font-medium text-slate-400 mb-2">
                                    Planner Model <span className="text-red-400">*</span>
                                </label>
                                <select
                                    value={plannerModelId}
                                    onChange={(e) => setPlannerModelId(e.target.value)}
                                    required
                                    className="w-full px-4 py-3 bg-black border border-white/10 rounded-xl text-white focus:border-purple-400 focus:ring-4 focus:ring-purple-400/20 outline-none">
                                    <option value="">Select planner model...</option>
                                    {selectedModelIds.length === 0 ? (
                                        <option disabled>Select models below first</option>
                                    ) : (
                                        allModels
                                            .filter(m => {
                                                const isSelected = selectedModelIds.includes(m.id);
                                                const isText = (m.modality || '').toLowerCase() === 'text';
                                                // Debug log (remove later)
                                                console.log(`Model ${m.name}: selected=${isSelected}, modality=${m.modality}, isText=${isText}`);
                                                return isSelected && isText;
                                            })
                                            .map(m => (
                                                <option key={m.id} value={m.id}>{m.name}</option>
                                            ))
                                    )}
                                </select>
                                <p className="text-xs text-slate-500 mt-1">
                                    The LLM that will orchestrate tool calls (must be a text model from selected models below)
                                </p>
                            </div>

                            {/* System Prompt */}
                            <div>
                                <div className="flex items-center justify-between mb-2">
                                    <label className="block text-sm font-medium text-slate-400">
                                        System Prompt (Optional)
                                    </label>
                                    <button
                                        type="button"
                                        onClick={() => setSystemPrompt('[ROLE]\n\n[TASK]\n\n[CONTEXT]\n\n[REASONING]\n\n[OUTPUT]\n\n[STOP]\n')}
                                        className="text-xs text-purple-400 hover:text-purple-300 transition-colors">
                                        Insert Template
                                    </button>
                                </div>

                                <div className="relative">
                                    <RichPromptEditor
                                        value={systemPrompt}
                                        onChange={setSystemPrompt}
                                        mentions={mentions}
                                        minHeight="200px"
                                        paddingTop="60px"
                                    />
                                    {/* Tag Badges Overlay */}
                                    <div className="absolute top-2 right-2 flex gap-1 pointer-events-none z-10">
                                        {['ROLE', 'TASK', 'CONTEXT', 'REASONING', 'OUTPUT', 'STOP'].map(tag => (
                                            <span key={tag} className={`text-[10px] px-1.5 py-0.5 rounded transition-colors backdrop-blur-sm ${systemPrompt.includes(`[${tag}]`) ? 'bg-green-500/20 text-green-400 border border-green-500/30' : 'bg-white/5 text-slate-500 border border-white/10'}`}>
                                                {tag}
                                            </span>
                                        ))}
                                    </div>
                                </div>
                                <p className="text-xs text-slate-500 mt-1">
                                    Custom instructions for the agent. Use [TAGS] to structure the prompt.
                                </p>
                            </div>

                            <div>
                                <label className="block text-sm font-medium text-slate-400 mb-2">
                                    Max Iterations
                                </label>
                                <Input
                                    type="number"
                                    value={maxIterations.toString()}
                                    onChange={(e) => setMaxIterations(parseInt(e.target.value) || 10)}
                                    min={1}
                                    max={20}
                                    icon={<span>🔁</span>}
                                />
                                <p className="text-xs text-slate-500 mt-1">
                                    Maximum ReAct loop iterations (default: 10)
                                </p>
                            </div>
                        </>
                    )}


                    <Input
                        label="Description (Optional)"
                        value={description}
                        onChange={(e) => setDescription(e.target.value)}
                        placeholder="Service description"
                        icon={<span>📝</span>}
                    />

                    <div>
                        <label className="block text-sm font-medium text-slate-400 mb-2">
                            {serviceType === 'AGENTIC' ? 'Orchestration Strategy' : 'Routing Strategy'}
                        </label>

                        {serviceType === 'AGENTIC' ? (
                            <div className="w-full px-4 py-3 bg-black/50 border border-white/10 rounded-xl text-slate-300 flex items-center gap-2 cursor-not-allowed">
                                <span>🧠</span>
                                <span>Planner</span>
                                <span className="text-xs text-slate-500 ml-auto">(Fixed)</span>
                            </div>
                        ) : (
                            <select
                                value={poolType === 'MULTI_MODALITY' ? 'none' : strategy}
                                onChange={(e) => setStrategy(e.target.value)}
                                disabled={poolType === 'MULTI_MODALITY'}
                                className="w-full px-4 py-3 bg-black border border-white/10 rounded-xl text-white focus:border-cyan-400 focus:ring-4 focus:ring-cyan-400/20 outline-none disabled:opacity-50 disabled:cursor-not-allowed">
                                {poolType === 'MULTI_MODALITY' ? (
                                    <option value="none">None (Multi-Modality)</option>
                                ) : (
                                    <>
                                        <option value="health">🏥 Health (Failover)</option>
                                        <option value="least_cost">💰 Cost (Lowest Price)</option>
                                        <option value="least_latency">⚡ Speed (Lowest Latency)</option>
                                        <option value="weighted_random">⚖️ Weight (Custom Distribution)</option>
                                    </>
                                )}
                            </select>
                        )}

                        {serviceType === 'AGENTIC' ? (
                            <p className="text-xs text-slate-500 mt-1">
                                Agentic services use a planner model to determine tool execution.
                            </p>
                        ) : poolType === 'MULTI_MODALITY' && (
                            <p className="text-xs text-slate-500 mt-1">
                                Routing is handled per-modality in Multi-Modality pools.
                            </p>
                        )}
                    </div>

                    {/* Guardrails (Coming Soon) */}
                    {/* Guardrails (Coming Soon) */}
                    <div className="opacity-50 pointer-events-none relative group select-none">
                        <Input
                            label="Guardrails"
                            value={guardrails}
                            onChange={(e) => setGuardrails(e.target.value)}
                            placeholder="PII filter, content moderation"
                            icon={<span>🛡️</span>}
                            disabled
                        />
                        <div className="absolute top-0 right-0">
                            <Badge variant="cyan" size="sm">Coming Soon</Badge>
                        </div>
                    </div>

                    {/* Rate Limiting (Coming Soon) */}
                    <div className="opacity-50 pointer-events-none relative group select-none">
                        <Input
                            label="Rate Limiting"
                            value=""
                            onChange={() => { }}
                            placeholder="Requests per minute"
                            icon={<span>⏱️</span>}
                            disabled
                        />
                        <div className="absolute top-0 right-0">
                            <Badge variant="cyan" size="sm">Coming Soon</Badge>
                        </div>
                    </div>

                    {/* Budget Limiting (Coming Soon) */}
                    <div className="opacity-50 pointer-events-none relative group select-none">
                        <Input
                            label="Budget Limiting"
                            value=""
                            onChange={() => { }}
                            placeholder="Monthly spending limit ($)"
                            icon={<span>💰</span>}
                            disabled
                        />
                        <div className="absolute top-0 right-0">
                            <Badge variant="cyan" size="sm">Coming Soon</Badge>
                        </div>
                    </div>

                    {/* Model Selection - required for new services */}
                    {!editingService && (
                        <div>
                            <label className="block text-sm font-medium text-slate-400 mb-2">
                                Models <span className="text-red-400">*</span>
                            </label>
                            {allModels.length === 0 ? (
                                <div className="p-4 bg-amber-400/10 border border-amber-400/30 rounded-xl text-amber-400 text-sm">
                                    ⚠️ No models available. Please add models in the Providers section first.
                                </div>
                            ) : (
                                <div className="space-y-2 max-h-40 overflow-y-auto border border-white/10 rounded-xl p-3">
                                    {allModels.map((model) => (
                                        <label
                                            key={model.id}
                                            className={`flex items-center gap-3 p-2 rounded-lg cursor-pointer transition-colors ${selectedModelIds.includes(model.id)
                                                ? 'bg-cyan-400/20 border border-cyan-400/50'
                                                : 'hover:bg-white/5'
                                                }`}>
                                            <input
                                                type="checkbox"
                                                checked={selectedModelIds.includes(model.id)}
                                                onChange={(e) => {
                                                    if (e.target.checked) {
                                                        setSelectedModelIds([...selectedModelIds, model.id])
                                                    } else {
                                                        setSelectedModelIds(selectedModelIds.filter(id => id !== model.id))
                                                    }
                                                }}
                                                className="rounded border-white/20 bg-black text-cyan-400 focus:ring-cyan-400/50"
                                            />
                                            <span className="text-white">{model.name}</span>
                                            <span className="text-xs text-slate-500">{model.modality}</span>
                                        </label>
                                    ))}
                                </div>
                            )}
                            {selectedModelIds.length === 0 && allModels.length > 0 && (
                                <p className="text-xs text-red-400 mt-1">Select at least one model</p>
                            )}
                        </div>
                    )}

                    {/* MCP Server Selection - only for AGENTIC services */}
                    {!editingService && serviceType === 'AGENTIC' && (
                        <div>
                            <label className="block text-sm font-medium text-slate-400 mb-2">
                                MCP Servers <span className="text-xs text-slate-500">(Optional)</span>
                            </label>
                            {allMcpServers.length === 0 ? (
                                <div className="p-4 bg-white/5 border border-white/10 rounded-xl text-slate-400 text-sm text-center">
                                    🔌 No MCP servers configured. Add them in Providers → MCP.
                                </div>
                            ) : (
                                <div className="space-y-2 max-h-40 overflow-y-auto border border-white/10 rounded-xl p-3">
                                    {allMcpServers.map((server) => (
                                        <label
                                            key={server.id}
                                            className={`flex items-center gap-3 p-2 rounded-lg cursor-pointer transition-colors ${selectedMcpServerIdsForCreate.includes(server.id)
                                                ? 'bg-purple-400/20 border border-purple-400/50'
                                                : 'hover:bg-white/5'
                                                }`}>
                                            <input
                                                type="checkbox"
                                                checked={selectedMcpServerIdsForCreate.includes(server.id)}
                                                onChange={(e) => {
                                                    if (e.target.checked) {
                                                        setSelectedMcpServerIdsForCreate([...selectedMcpServerIdsForCreate, server.id])
                                                    } else {
                                                        setSelectedMcpServerIdsForCreate(selectedMcpServerIdsForCreate.filter(id => id !== server.id))
                                                    }
                                                }}
                                                className="rounded border-white/20 bg-black text-purple-400 focus:ring-purple-400/50"
                                            />
                                            <div className="flex-1 flex items-center gap-2">
                                                <span className="text-white">{server.name}</span>
                                                <span className={`w-2 h-2 rounded-full ${server.status === 'connected' ? 'bg-green-400' : 'bg-slate-400'}`} />
                                            </div>
                                            <span className="text-xs text-slate-500">{server.server_type}</span>
                                        </label>
                                    ))}
                                </div>
                            )}
                            {selectedMcpServerIdsForCreate.length > 0 && (
                                <p className="text-xs text-purple-400 mt-1">✓ {selectedMcpServerIdsForCreate.length} MCP server{selectedMcpServerIdsForCreate.length > 1 ? 's' : ''} selected</p>
                            )}
                        </div>
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
                            {editingService ? 'Update' : 'Create'}
                        </Button>
                    </div>
                </form>
            </Modal >

            {/* Manage Models Modal */}
            <Modal
                isOpen={showSnippetModal}
                onClose={() => setShowSnippetModal(false)}
                title="API Code Snippet"
                description={`How to call ${snippetService?.name} via API`}
            >
                <div className="space-y-4">
                    <p className="text-sm text-slate-400">
                        Use this cURL command to interact with your service. Replace <code>YOUR_API_KEY</code> with your actual API key.
                    </p>
                    <div className="relative group">
                        <pre className="bg-black border border-white/10 rounded-xl p-4 overflow-x-auto text-xs font-mono text-slate-300">
                            {`curl -X POST http://localhost:8030/v1/chat/completions \\
  -H "Content-Type: application/json" \\
  -H "Authorization: Bearer YOUR_API_KEY" \\
  -d '{
    "service": "${snippetService?.name || 'service-name'}",
    "messages": [
      {
        "role": "user",
        "content": "Hello!"
      }
    ]
  }'`}
                        </pre>
                        <button
                            onClick={() => {
                                const code = `curl -X POST http://localhost:8030/v1/chat/completions \\
  -H "Content-Type: application/json" \\
  -H "Authorization: Bearer YOUR_API_KEY" \\
  -d '{
    "service": "${snippetService?.name || 'service-name'}",
    "messages": [
      {
        "role": "user",
        "content": "Hello!"
      }
    ]
  }'`
                                navigator.clipboard.writeText(code)
                                toast.success('Copied to clipboard')
                            }}
                            className="absolute top-2 right-2 p-2 bg-white/10 hover:bg-white/20 rounded-lg text-white opacity-0 group-hover:opacity-100 transition-opacity"
                        >
                            📋 Copy
                        </button>
                    </div>
                </div>
            </Modal>

            {/* Manage Models Modal */}
            < Modal
                isOpen={showModelsModal}
                onClose={() => setShowModelsModal(false)}
                title={`Manage Models - ${selectedService?.name}`}
                description="Assign models and configure routing weights"
                size="lg" >

                <div className="space-y-4">
                    {/* Current assigned models - BULK EDITOR for POOL */}
                    {serviceModels.length === 0 ? (
                        <div className="py-8 text-center text-slate-400">
                            No models assigned yet
                        </div>
                    ) : (
                        <div className="space-y-4">
                            <div className="flex items-center justify-between">
                                <div className="text-sm font-medium text-slate-400">Assigned Models</div>
                                {selectedService?.service_type === 'POOL' && (
                                    <div className="flex items-center gap-2 text-xs">
                                        <span className="text-slate-500">Total Weight:</span>
                                        <span className={`font-bold ${serviceModels.reduce((acc, m) => acc + (m.weight || 0), 0) === 100 ? 'text-emerald-400' : 'text-amber-400'}`}>
                                            {serviceModels.reduce((acc, m) => acc + (m.weight || 0), 0)}%
                                        </span>
                                    </div>
                                )}
                            </div>

                            {serviceModels.map((sm, idx) => (
                                <div
                                    key={sm.model_id}
                                    className="glass rounded-xl p-4">
                                    <div className="flex items-center justify-between gap-4">
                                        <div className="flex-1">
                                            <div className="font-semibold text-white flex items-center gap-2">
                                                {sm.model_name}
                                                {sm.is_healthy === false && (
                                                    <span title="Model Unhealthy" className="text-lg">⛔️</span>
                                                )}
                                            </div>
                                            <div className="text-xs text-slate-400">{sm.modality}</div>
                                        </div>

                                        {/* Inputs for POOL */}
                                        {selectedService?.service_type === 'POOL' ? (
                                            <div className="flex items-center gap-3">
                                                <div className="flex flex-col gap-1 w-20">
                                                    <label className="text-[10px] text-slate-500 uppercase">Pos</label>
                                                    <input
                                                        type="number"
                                                        value={sm.position}
                                                        onChange={(e) => {
                                                            const newModels = [...serviceModels];
                                                            newModels[idx].position = parseInt(e.target.value) || 0;
                                                            setServiceModels(newModels);
                                                        }}
                                                        className="w-full bg-black/50 border border-white/10 rounded px-2 py-1 text-sm text-white text-center focus:border-cyan-400 outline-none"
                                                    />
                                                </div>
                                                <div className="flex flex-col gap-1 w-20">
                                                    <label className="text-[10px] text-slate-500 uppercase">Weight</label>
                                                    <input
                                                        type="number"
                                                        value={sm.weight}
                                                        onChange={(e) => {
                                                            const newModels = [...serviceModels];
                                                            newModels[idx].weight = parseInt(e.target.value) || 0;
                                                            setServiceModels(newModels);
                                                        }}
                                                        className="w-full bg-black/50 border border-white/10 rounded px-2 py-1 text-sm text-white text-center focus:border-cyan-400 outline-none"
                                                    />
                                                </div>
                                            </div>
                                        ) : (
                                            <div className="text-xs text-slate-500">
                                                Pos: {sm.position}
                                            </div>
                                        )}

                                        <div className="flex items-center gap-2">
                                            <button
                                                onClick={() => {
                                                    setEditingModel(sm)
                                                    setConfigForm({
                                                        weight: sm.weight,
                                                        position: sm.position,
                                                        rtcros: {
                                                            role: sm.rtcros?.role || '',
                                                            task: sm.rtcros?.task || '',
                                                            context: sm.rtcros?.context || '',
                                                            reasoning: sm.rtcros?.reasoning || '',
                                                            output: sm.rtcros?.output || '',
                                                            stop: sm.rtcros?.stop || ''
                                                        }
                                                    })
                                                    // RTCROS text logic...
                                                    const initialText = [
                                                        (sm.rtcros?.role || '') && `[ROLE]\n${sm.rtcros?.role}`,
                                                        (sm.rtcros?.task || '') && `[TASK]\n${sm.rtcros?.task}`,
                                                        (sm.rtcros?.context || '') && `[CONTEXT]\n${sm.rtcros?.context}`,
                                                        (sm.rtcros?.reasoning || '') && `[REASONING]\n${sm.rtcros?.reasoning}`,
                                                        (sm.rtcros?.output || '') && `[OUTPUT]\n${sm.rtcros?.output}`,
                                                        (sm.rtcros?.stop || '') && `[STOP]\n${sm.rtcros?.stop}`
                                                    ].filter(Boolean).join('\n\n')
                                                    setRtcrosText(initialText || '[ROLE]\n\n[TASK]\n\n[CONTEXT]\n\n[REASONING]\n\n[OUTPUT]\n\n[STOP]\n')
                                                    setShowConfigModal(true)
                                                }}
                                                className="p-2 hover:bg-white/10 rounded-lg text-cyan-400 transition-colors"
                                                title="Configure Advanced (RTCROS)"
                                            >
                                                ⚙️
                                            </button>
                                            <button
                                                onClick={() => handleRemoveModel(sm)}
                                                className="p-2 hover:bg-white/10 rounded-lg text-red-400 transition-colors"
                                                title="Remove"
                                            >
                                                🗑️
                                            </button>
                                        </div>
                                    </div>
                                </div>
                            ))}

                            {/* Bulk Save Button */}
                            {selectedService?.service_type === 'POOL' && serviceModels.length > 0 && (
                                <div className="pt-4 border-t border-white/10 flex justify-end gap-3">
                                    <div className="text-xs text-slate-500 flex items-center mr-auto">
                                        💡 Weights must sum to 100%
                                    </div>
                                    <Button
                                        onClick={async () => {
                                            const total = serviceModels.reduce((acc, m) => acc + (m.weight || 0), 0);
                                            if (total !== 100) {
                                                toast.error(`Total weight must be 100% (Currently ${total}%)`);
                                                return;
                                            }

                                            try {
                                                const res = await fetch(`/v1/services/${selectedService.name}/models-bulk`, {
                                                    method: 'PUT',
                                                    headers: { 'Content-Type': 'application/json' },
                                                    body: JSON.stringify({
                                                        models: serviceModels.map(m => ({
                                                            model_id: m.model_id,
                                                            position: m.position,
                                                            weight: m.weight,
                                                            rtcros_role: m.rtcros?.role,
                                                            rtcros_task: m.rtcros?.task,
                                                            rtcros_context: m.rtcros?.context,
                                                            rtcros_reasoning: m.rtcros?.reasoning,
                                                            rtcros_output: m.rtcros?.output,
                                                            rtcros_stop: m.rtcros?.stop
                                                        }))
                                                    })
                                                });

                                                if (res.ok) {
                                                    toast.success('Distribution saved');
                                                    loadServiceModels(selectedService.name);
                                                } else {
                                                    const err = await res.text();
                                                    toast.error(`Save failed: ${err}`);
                                                }
                                            } catch (e) {
                                                toast.error('Failed to save distribution');
                                            }
                                        }}
                                        className="bg-cyan-500 hover:bg-cyan-400 text-black font-bold"
                                    >
                                        Save Distribution
                                    </Button>
                                </div>
                            )}
                        </div>
                    )}

                    {/* Available models to assign */}
                    {allModels.filter(m => !serviceModels.some(sm => sm.model_id === m.id)).length > 0 && (
                        <div>
                            <div className="text-sm font-medium text-slate-400 mb-2">Available Models</div>
                            <div className="space-y-2 max-h-40 overflow-y-auto">
                                {allModels
                                    .filter(m => !serviceModels.some(sm => sm.model_id === m.id))
                                    .map((model) => (
                                        <div
                                            key={model.id}
                                            className="flex items-center justify-between p-3 rounded-xl bg-white/5 hover:bg-white/10 transition-colors">
                                            <div>
                                                <div className="text-white">{model.name}</div>
                                                <div className="text-xs text-slate-500">{model.modality}</div>
                                            </div>
                                            <Button
                                                variant="secondary"
                                                onClick={async () => {
                                                    if (!selectedService) return
                                                    try {
                                                        const res = await fetch(`/v1/services/${selectedService.name}/models`, {
                                                            method: 'POST',
                                                            headers: { 'Content-Type': 'application/json' },
                                                            credentials: 'include',
                                                            body: JSON.stringify({
                                                                model_id: model.id,
                                                                modality: model.modality || 'text',
                                                                position: serviceModels.length + 1,
                                                                weight: 50, // Default to 50 so users can add multiple models
                                                            }),
                                                        })
                                                        if (res.ok) {
                                                            toast.success(`${model.name} assigned`)
                                                            loadServiceModels(selectedService.name)
                                                        } else {
                                                            const errorMsg = await res.text()
                                                            toast.error(`Failed: ${errorMsg}`)
                                                        }
                                                    } catch (error) {
                                                        toast.error(`Error: ${error}`)
                                                    }
                                                }}>
                                                + Add
                                            </Button>
                                        </div>
                                    ))}
                            </div>
                        </div>
                    )}

                    {allModels.length === 0 && (
                        <div className="p-4 bg-amber-400/10 border border-amber-400/30 rounded-xl text-amber-400 text-sm">
                            ⚠️ No models available. Add models in the Providers section first.
                        </div>
                    )}
                </div>
            </Modal >

            {/* Tools Management Modal */}
            <Modal
                isOpen={showToolsModal}
                onClose={() => setShowToolsModal(false)}
                title={`Tools - ${selectedService?.name}`}
                description="Configure tools available to the agent"
                size="lg">

                <div className="flex gap-2 mb-6 border-b border-white/10 pb-2">
                    <button
                        onClick={() => setActiveToolsTab('models')}
                        className={`px-4 py-2 rounded-lg text-sm transition-colors ${activeToolsTab === 'models' ? 'bg-white/10 text-white' : 'text-slate-400 hover:text-white'}`}>
                        Models
                    </button>
                    <button
                        onClick={() => setActiveToolsTab('mcp')}
                        className={`px-4 py-2 rounded-lg text-sm transition-colors ${activeToolsTab === 'mcp' ? 'bg-white/10 text-white' : 'text-slate-400 hover:text-white'}`}>
                        MCP Servers
                    </button>
                </div>

                <div className="space-y-4">

                    {/* MODELS TAB */}
                    {activeToolsTab === 'models' && (
                        <div className="space-y-6">
                            {/* Assigned Models Section */}
                            {serviceModels.length === 0 ? (
                                <div className="py-8 text-center text-slate-400">
                                    No models assigned yet
                                </div>
                            ) : (
                                <div className="space-y-2">
                                    <div className="text-sm font-medium text-slate-400 mb-2">Assigned Models</div>
                                    {serviceModels.map((sm) => (
                                        <div
                                            key={sm.model_id}
                                            className="glass rounded-xl p-4">
                                            <div className="flex items-center justify-between">
                                                <div>
                                                    <div className="font-semibold text-white flex items-center gap-2">
                                                        {sm.model_name}
                                                        {sm.is_healthy === false && (
                                                            <span title="Model Unhealthy" className="text-lg">⛔️</span>
                                                        )}
                                                    </div>
                                                    <div className="text-xs text-slate-400">Position {sm.position} • {sm.modality}</div>
                                                </div>
                                                <div className="flex items-center gap-3">
                                                    {selectedService?.service_type !== 'AGENTIC' && (
                                                        <Badge variant="primary">
                                                            Weight: {sm.weight}
                                                        </Badge>
                                                    )}
                                                    <button
                                                        onClick={() => handleRemoveModel(sm)}
                                                        className="text-red-400 hover:text-red-300 text-sm">
                                                        Remove
                                                    </button>
                                                    <button
                                                        onClick={() => {
                                                            setEditingModel(sm);
                                                            setConfigForm({
                                                                weight: sm.weight,
                                                                position: sm.position,
                                                                rtcros: {
                                                                    role: sm.rtcros?.role || '',
                                                                    task: sm.rtcros?.task || '',
                                                                    context: sm.rtcros?.context || '',
                                                                    reasoning: sm.rtcros?.reasoning || '',
                                                                    output: sm.rtcros?.output || '',
                                                                    stop: sm.rtcros?.stop || ''
                                                                }
                                                            });
                                                            const initialText = [
                                                                (sm.rtcros?.role || '') && `[ROLE]\n${sm.rtcros?.role}`,
                                                                (sm.rtcros?.task || '') && `[TASK]\n${sm.rtcros?.task}`,
                                                                (sm.rtcros?.context || '') && `[CONTEXT]\n${sm.rtcros?.context}`,
                                                                (sm.rtcros?.reasoning || '') && `[REASONING]\n${sm.rtcros?.reasoning}`,
                                                                (sm.rtcros?.output || '') && `[OUTPUT]\n${sm.rtcros?.output}`,
                                                                (sm.rtcros?.stop || '') && `[STOP]\n${sm.rtcros?.stop}`
                                                            ].filter(Boolean).join('\n\n');
                                                            setRtcrosText(initialText || '[ROLE]\n\n[TASK]\n\n[CONTEXT]\n\n[REASONING]\n\n[OUTPUT]\n\n[STOP]\n');
                                                            setShowConfigModal(true);
                                                        }}
                                                        className="text-cyan-400 hover:text-cyan-300 text-sm flex items-center gap-1">
                                                        <span>⚙️</span> Configure
                                                    </button>
                                                </div>
                                            </div>
                                        </div>
                                    ))}
                                </div>
                            )}

                            {/* Available Models Section */}
                            {allModels.filter(m => !serviceModels.some(sm => sm.model_id === m.id)).length > 0 && (
                                <div>
                                    <div className="text-sm font-medium text-slate-400 mb-2">Available Models</div>
                                    <div className="space-y-2 max-h-40 overflow-y-auto">
                                        {allModels
                                            .filter(m => !serviceModels.some(sm => sm.model_id === m.id))
                                            .map((model) => (
                                                <div
                                                    key={model.id}
                                                    className="flex items-center justify-between p-3 rounded-xl bg-white/5 hover:bg-white/10 transition-colors">
                                                    <div>
                                                        <div className="text-white">{model.name}</div>
                                                        <div className="text-xs text-slate-500">{model.modality}</div>
                                                    </div>
                                                    <Button
                                                        variant="secondary"
                                                        size="sm"
                                                        onClick={async () => {
                                                            if (!selectedService) return;
                                                            try {
                                                                await fetch(`/v1/services/${selectedService.name}/models`, {
                                                                    method: 'POST',
                                                                    headers: { 'Content-Type': 'application/json' },
                                                                    credentials: 'include',
                                                                    body: JSON.stringify({
                                                                        model_id: model.id,
                                                                        modality: model.modality || 'text',
                                                                        position: serviceModels.length + 1,
                                                                        weight: 100,
                                                                    }),
                                                                })
                                                                toast.success('Model added');
                                                                loadServiceModels(selectedService.name);
                                                            } catch (e) { toast.error('Failed to add') }
                                                        }}>
                                                        + Add
                                                    </Button>
                                                </div>
                                            ))}
                                    </div>
                                </div>
                            )}

                            {allModels.length === 0 && (
                                <div className="p-4 bg-amber-400/10 border border-amber-400/30 rounded-xl text-amber-400 text-sm">
                                    ⚠️ No models available. Add models in the Providers section first.
                                </div>
                            )}
                        </div>
                    )}

                    {/* MCP SERVERS TAB */}
                    {activeToolsTab === 'mcp' && (
                        <div className="space-y-6">
                            {/* Assigned MCP Servers Section */}
                            {assignedMcpServers.length === 0 ? (
                                <div className="py-8 text-center text-slate-400">
                                    No MCP servers assigned yet
                                </div>
                            ) : (
                                <div className="space-y-2">
                                    <div className="text-sm font-medium text-slate-400 mb-2">Assigned MCP Servers</div>
                                    {assignedMcpServers.map((server) => (
                                        <div
                                            key={server.id}
                                            className="glass rounded-xl p-4">
                                            <div className="flex items-center justify-between">
                                                <div>
                                                    <div className="font-semibold text-white">{server.name}</div>
                                                    <div className="text-xs text-slate-400 capitalize">{server.server_type} • {server.status}</div>
                                                </div>
                                                <div className="flex items-center gap-3">
                                                    <Badge variant={server.status === 'connected' ? 'success' : 'warning'}>
                                                        {server.status}
                                                    </Badge>
                                                    <button
                                                        onClick={() => handleRemoveServer(server)}
                                                        className="text-red-400 hover:text-red-300 text-sm">
                                                        Remove
                                                    </button>
                                                </div>
                                            </div>
                                        </div>
                                    ))}
                                </div>
                            )}

                            {/* Available MCP Servers Section */}
                            {allMcpServers.filter(s => !assignedMcpServers.some(as => as.id === s.id)).length > 0 && (
                                <div>
                                    <div className="text-sm font-medium text-slate-400 mb-2">Available MCP Servers</div>
                                    <div className="space-y-2 max-h-40 overflow-y-auto">
                                        {allMcpServers
                                            .filter(s => !assignedMcpServers.some(as => as.id === s.id))
                                            .map((server) => (
                                                <div
                                                    key={server.id}
                                                    className="flex items-center justify-between p-3 rounded-xl bg-white/5 hover:bg-white/10 transition-colors">
                                                    <div>
                                                        <div className="text-white">{server.name}</div>
                                                        <div className="text-xs text-slate-500 capitalize">{server.server_type} • {server.status}</div>
                                                    </div>
                                                    <Button
                                                        variant="secondary"
                                                        size="sm"
                                                        onClick={async () => {
                                                            if (!selectedService) return;
                                                            const url = `/v1/services/${selectedService.name}/mcp-servers/${server.id}`;
                                                            console.log('Adding MCP server - URL:', url, 'Service:', selectedService.name, 'Server ID:', server.id);
                                                            try {
                                                                const res = await fetch(url, {
                                                                    method: 'POST', credentials: 'include'
                                                                });
                                                                console.log('Response status:', res.status, 'ok:', res.ok);
                                                                if (res.ok) {
                                                                    toast.success('Server added');
                                                                    await loadMcpServers(selectedService.name);
                                                                } else {
                                                                    const errText = await res.text();
                                                                    console.error('Add server failed:', errText);
                                                                    toast.error(`Failed to add: ${errText}`);
                                                                }
                                                            } catch (e) {
                                                                console.error('Add server error:', e);
                                                                toast.error('Failed to add server');
                                                            }
                                                        }}>
                                                        + Add
                                                    </Button>
                                                </div>
                                            ))}
                                    </div>
                                </div>
                            )}

                            {allMcpServers.length === 0 && (
                                <div className="py-8 text-center">
                                    <div className="text-4xl mb-3">🔌</div>
                                    <div className="text-slate-400 mb-4">No MCP Servers available</div>
                                    <Link
                                        href="/providers/mcp"
                                        className="inline-flex items-center gap-2 px-4 py-2 bg-cyan-400/10 hover:bg-cyan-400/20 border border-cyan-400/30 rounded-lg text-cyan-400 text-sm transition-colors"
                                    >
                                        <span>+</span> Connect MCP Server
                                    </Link>
                                </div>
                            )}
                        </div>
                    )}
                </div>
            </Modal>

            {/* Model Config Modal */}
            < Modal
                isOpen={showConfigModal}
                onClose={() => setShowConfigModal(false)}
                title={editingModel ? `Configure ${editingModel.model_name}` : 'Configure Model'}
                description="Adjust routing weight and RTCROS prompts"
                size="lg" >

                <div className="space-y-6">
                    {/* Routing Settings */}
                    <div className="grid grid-cols-2 gap-4">
                        <Input
                            label="Routing Weight"
                            type="number"
                            value={configForm.weight}
                            onChange={(e) => setConfigForm({ ...configForm, weight: parseInt(e.target.value) || 0 })}
                            min={0}
                            max={100}
                        />
                        <Input
                            label="Position (Priority)"
                            type="number"
                            value={configForm.position}
                            onChange={(e) => setConfigForm({ ...configForm, position: parseInt(e.target.value) || 1 })}
                            min={1}
                        />
                    </div>

                    <div className="h-px bg-white/10" />

                    {/* RTCROS Settings */}
                    <div className="space-y-4">
                        <div className="flex items-center justify-between">
                            <h3 className="text-sm font-semibold text-white flex items-center gap-2">
                                <span className="text-lg">⚡</span>
                                RTCROS Override
                            </h3>
                            <Badge variant="purple" size="sm">Optional</Badge>
                        </div>
                        <p className="text-xs text-slate-400">
                            Define model-specific behaviors. These override the generic system prompt structure.
                        </p>

                        <div className="relative">
                            <RichPromptEditor
                                value={rtcrosText}
                                onChange={(val) => {
                                    setRtcrosText(val)
                                    const parsed = parseRtcrosText(val)
                                    setConfigForm(prev => ({
                                        ...prev,
                                        rtcros: parsed
                                    }))
                                }}
                                mentions={mentions}
                                minHeight="250px"
                                paddingTop="60px"
                            />
                            {/* Tag Badges Overlay */}
                            <div className="absolute top-2 right-2 flex gap-1 pointer-events-none z-10">
                                {['ROLE', 'TASK', 'CONTEXT', 'REASONING', 'OUTPUT', 'STOP'].map(tag => (
                                    <span key={tag} className={`text-[10px] px-1.5 py-0.5 rounded transition-colors backdrop-blur-sm ${configForm.rtcros[tag.toLowerCase() as keyof typeof configForm.rtcros] ? 'bg-green-500/20 text-green-400 border border-green-500/30' : 'bg-white/5 text-slate-500 border border-white/10'}`}>
                                        {tag}
                                    </span>
                                ))}
                            </div>
                        </div>

                    </div>

                    <div className="flex gap-3 pt-4 border-t border-white/10">
                        <Button
                            type="button"
                            variant="secondary"
                            onClick={() => setShowConfigModal(false)}
                            className="flex-1">
                            Cancel
                        </Button>
                        <Button
                            type="button"
                            variant="primary"
                            onClick={async () => {
                                if (!selectedService || !editingModel) return
                                try {
                                    const res = await fetch(`/v1/services/${selectedService.name}/models/${editingModel.model_id}`, {
                                        method: 'PUT',
                                        headers: { 'Content-Type': 'application/json' },
                                        credentials: 'include',
                                        body: JSON.stringify({
                                            weight: configForm.weight,
                                            position: configForm.position,
                                            rtcros_role: configForm.rtcros.role,
                                            rtcros_task: configForm.rtcros.task,
                                            rtcros_context: configForm.rtcros.context,
                                            rtcros_reasoning: configForm.rtcros.reasoning,
                                            rtcros_output: configForm.rtcros.output,
                                            rtcros_stop: configForm.rtcros.stop
                                        })
                                    })

                                    if (res.ok) {
                                        toast.success('Configuration saved')
                                        setShowConfigModal(false)
                                        loadServiceModels(selectedService.name)
                                    } else {
                                        const errorMsg = await res.text()
                                        toast.error(`Failed to save: ${errorMsg}`)
                                    }
                                } catch (error) {
                                    toast.error(`Error: ${error}`)
                                }
                            }}
                            className="flex-1">
                            Save Changes
                        </Button>
                    </div>
                </div>
            </Modal>
            {/* Delete Confirmation Modal */}
            <Modal
                isOpen={showDeleteModal}
                onClose={() => setShowDeleteModal(false)}
                title="Delete Service"
                description="Are you sure you want to delete this service?"
            >
                <div>
                    <p className="text-slate-300 mb-6">
                        This will permanently delete the service <strong className="text-white">{serviceToDelete?.name}</strong>.
                        This action cannot be undone.
                    </p>
                    <div className="flex justify-end gap-3">
                        <Button variant="ghost" onClick={() => setShowDeleteModal(false)} disabled={isDeleting}>Cancel</Button>
                        <Button variant="danger" onClick={confirmDelete} disabled={isDeleting}>
                            {isDeleting ? 'Deleting...' : 'Delete Service'}
                        </Button>
                    </div>
                </div>
            </Modal>

            {/* Remove Model Modal */}
            <Modal
                isOpen={showRemoveModelModal}
                onClose={() => setShowRemoveModelModal(false)}
                title="Remove Model"
                description="Remove this model from the service?"
            >
                <div>
                    <p className="text-slate-300 mb-6">
                        This will remove <strong className="text-white">{modelToRemove?.model_name || modelToRemove?.name}</strong> from <strong className="text-white">{selectedService?.name}</strong>.
                    </p>
                    <div className="flex justify-end gap-3">
                        <Button variant="ghost" onClick={() => setShowRemoveModelModal(false)} disabled={isRemoving}>Cancel</Button>
                        <Button variant="danger" onClick={confirmRemoveModel} disabled={isRemoving}>
                            {isRemoving ? 'Removing...' : 'Remove Model'}
                        </Button>
                    </div>
                </div>
            </Modal>

            {/* Remove Server Modal */}
            <Modal
                isOpen={showRemoveServerModal}
                onClose={() => setShowRemoveServerModal(false)}
                title="Remove MCP Server"
                description="Remove this server from the service?"
            >
                <div>
                    <p className="text-slate-300 mb-6">
                        This will remove <strong className="text-white">{serverToRemove?.name}</strong> from <strong className="text-white">{selectedService?.name}</strong>.
                    </p>
                    <div className="flex justify-end gap-3">
                        <Button variant="ghost" onClick={() => setShowRemoveServerModal(false)} disabled={isRemoving}>Cancel</Button>
                        <Button variant="danger" onClick={confirmRemoveServer} disabled={isRemoving}>
                            {isRemoving ? 'Removing...' : 'Remove Server'}
                        </Button>
                    </div>
                </div>
            </Modal>
        </div >
    )
}
