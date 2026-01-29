'use client'

import { useEffect, useState, useRef, useCallback } from 'react'
import {
  ReactFlow,
  Background,
  Controls,
  ControlButton, // Added
  Panel,
  Node,
  Edge,
  useNodesState,
  useEdgesState,
  ConnectionLineType,
  NodeChange,
  useReactFlow, // Added
} from '@xyflow/react'
import '@xyflow/react/dist/style.css'
import UserNode from '@/components/UserNode'
import GatewayNode from '@/components/GatewayNode'
import ServiceNode from '@/components/ServiceNode'
import ModelNode from '@/components/ModelNode'
import { Card } from '@/components/ui'
import { motion } from 'framer-motion'
import { useAuth } from '@/contexts/AuthContext'
import { useRouter } from 'next/navigation'

const nodeTypes = {
  user: UserNode,
  gateway: GatewayNode,
  service: ServiceNode,
  model: ModelNode,
}

export default function Home() {
  const { user } = useAuth()
  const router = useRouter()
  const [providers, setProviders] = useState(0)
  const [services, setServices] = useState(0)
  const [models, setModels] = useState(0)
  const [mcpServers, setMcpServers] = useState(0)
  const [nodes, setNodes, onNodesChange] = useNodesState<Node>([])
  const [edges, setEdges, onEdgesChange] = useEdgesState<Edge>([])
  const [isInitialized, setIsInitialized] = useState(false)

  // Store user-modified positions
  const nodePositionsRef = useRef<Record<string, { x: number; y: number }>>({})

  // Custom handler that saves positions when nodes are dragged
  const handleNodesChange = useCallback((changes: NodeChange[]) => {
    onNodesChange(changes)

    // Save positions when nodes are moved
    changes.forEach((change) => {
      if (change.type === 'position' && change.position && change.id) {
        nodePositionsRef.current[change.id] = change.position
      }
    })
  }, [onNodesChange])

  const loadData = useCallback(async () => {
    try {
      const response = await fetch('/v1/topology')

      if (!response.ok) {
        throw new Error('Failed to fetch topology')
      }

      const data = await response.json()

      setProviders(data.providers.length)
      setServices(data.services.length)
      setModels(data.models.length)

      // Calculate unique MCP servers used in services
      const uniqueMcpServers = new Set(
        (data.services || []).flatMap((s: any) =>
          (s.mcp_servers || []).map((ms: any) => ms.id)
        )
      )
      setMcpServers(uniqueMcpServers.size)

      buildVisualization(data.providers, data.services, data.models)
    } catch (error) {
      console.error('Failed to load data:', error)
      // Only rebuild if we don't have existing data to avoid flashing empty state on transient errors
      // But for initial load we might strictly want to reset.
      // Keeping existing behavior of partial reset on error but improving resilience.
    }
  }, []) // Stable dependency

  useEffect(() => {
    loadData()
    // Reduced polling frequency from 8s to 30s to improve performance
    const interval = setInterval(loadData, 30000)
    return () => clearInterval(interval)
  }, [loadData])

  // Authentication Guard
  useEffect(() => {
    if (!user) {
      router.push('/auth/login')
    }
  }, [user, router])

  if (!user) return null

  // Helper function to determine routing order based on strategy
  const getRoutingOrder = (strategy: string, models: any[]) => {
    const strategyLower = strategy?.toLowerCase() || 'health'

    // For cost-based routing, sort by cost (self-hosted = $0 first)
    if (strategyLower === 'least_cost') {
      return [...models].sort((a, b) => {
        const aCost = a.provider_type?.toLowerCase() === 'selfhosted' || a.provider_type?.toLowerCase() === 'ollama' ? 0 : (a.cost || 999)
        const bCost = b.provider_type?.toLowerCase() === 'selfhosted' || b.provider_type?.toLowerCase() === 'ollama' ? 0 : (b.cost || 999)
        return aCost - bCost
      })
    }

    // For weighted/random strategies, all are equal (show weights instead)
    if (strategyLower.includes('weighted') || strategyLower === 'random' || strategyLower === 'pool') {
      return models // No single "first"
    }

    // For health/priority/leader-worker, use position order
    return [...models].sort((a, b) => (a.position || 999) - (b.position || 999))
  }

  const buildVisualization = (providersData: any[], servicesData: any[], allModels: any[]) => {
    // Calculate layout - horizontal flow with even spacing
    const HORIZONTAL_GAP = 380
    const VERTICAL_CENTER = 320
    const MODEL_VERTICAL_GAP = 120 // Increased slightly to prevent overlap
    const MIN_SERVICE_HEIGHT = 300 // Minimum vertical space per service

    // Calculate required height for each service based on its model count
    const serviceHeights = servicesData.map(s => {
      const modelCount = (s.models || []).length
      // Calculate height needed for models relative to their service center
      // If 4 models, they span (3 * 120) = 360px center-to-center, plus padding (say 100px) = ~460px
      // A simple heuristic: modelCount * MODEL_VERTICAL_GAP or MIN_SERVICE_HEIGHT
      return Math.max(MIN_SERVICE_HEIGHT, (modelCount * MODEL_VERTICAL_GAP) + 50)
    })

    const totalStructureHeight = serviceHeights.reduce((a, b) => a + b, 0)
    let currentY = VERTICAL_CENTER - (totalStructureHeight / 2) // Start from top-most point

    // Helper to get position - use saved position if exists, otherwise default
    const getPosition = (id: string, defaultPos: { x: number; y: number }) => {
      return nodePositionsRef.current[id] || defaultPos
    }

    // Only show providers that have at least one model configured
    const activeProviders = providersData.filter((p: any) =>
      allModels.some((m: any) => m.provider === p.id)
    )

    const builtNodes: Node[] = [
      {
        id: 'user',
        type: 'user',
        position: getPosition('user', { x: 80, y: VERTICAL_CENTER }),
        data: {
          user: user
        }
      },
      {
        id: 'gateway',
        type: 'gateway',
        position: getPosition('gateway', { x: 80 + HORIZONTAL_GAP, y: VERTICAL_CENTER - 100 }),
        data: { providers: activeProviders }
      },
    ]

    const builtEdges: Edge[] = [
      {
        id: 'e-user-gateway',
        source: 'user',
        target: 'gateway',
        type: 'default',
        animated: true,
        style: {
          stroke: '#22d3ee',
          strokeWidth: 2.5,
          strokeDasharray: '8 4',
        },
      },
    ]

    servicesData.forEach((serviceWithModels, serviceIndex) => {
      // Handle nested structure from /v1/topology
      const service = serviceWithModels.service || serviceWithModels
      const serviceId = `service-${service.name}`
      const serviceModels = serviceWithModels.models || []

      // Handle Agentic vs Pool layout calculation early so we can pass prop to node
      const serviceTypeUpper = service.service_type?.toUpperCase() || ''
      const isAgentic = serviceTypeUpper === 'AGENTIC' || !!service.planner_model_id

      const thisHeight = serviceHeights[serviceIndex]
      // Center of this service block
      const serviceY = currentY + (thisHeight / 2)

      // Advance Y for next service
      currentY += thisHeight

      // Calculate Modality
      const distinctModalities = Array.from(new Set(
        serviceModels.length > 0
          ? serviceModels.map((m: any) => m.modality)
          : (service.input_modalities || ['text'])
      ))
      const derivedModality = distinctModalities.length > 1 ? 'multi-modal' : distinctModalities[0]

      builtNodes.push({
        id: serviceId,
        type: 'service',
        position: getPosition(serviceId, { x: 80 + HORIZONTAL_GAP * 2 + 80, y: serviceY - 60 }),
        data: {
          label: service.name,
          type: service.service_type,
          strategy: service.strategy,
          guardrails: service.guardrails && service.guardrails !== '[]',
          models: serviceModels,
          isAgentic: isAgentic,
          modality: derivedModality
        }
      })

      // Curved bezier edge from gateway to service
      builtEdges.push({
        id: `e-gateway-${serviceId}`,
        source: 'gateway',
        target: serviceId,
        type: 'default',
        animated: true,
        style: {
          stroke: '#a855f7',
          strokeWidth: 2,
          strokeDasharray: '6 4',
        },
      })

      if (isAgentic) {
        // Find planner model (rest of logic remains same)
        // Find planner model
        const plannerId = service.planner_model_id
        // Try to find the assigned planner, or finding via model_id, or fallback to first if none match but isAgentic?
        // Actually if plannerId is missing but isAgentic is true (e.g. manual type), we might default to first model or warn?
        // Let's stick to user request: Planner First.
        const plannerModel = plannerId
          ? serviceModels.find((m: any) => m.model_id === plannerId)
          : serviceModels[0] // Fallback to first as planner if explicit ID missing but type is Agentic

        if (plannerModel) {
          // 1. Add Planner Node
          const plannerNodeId = `model-${service.name}-${plannerModel.model_id}`
          const modelInfo = allModels.find((m: any) => m.id === plannerModel.model_id)
          const provider = providersData.find((p: any) => p.id === modelInfo?.provider)

          const LOGO_MAP: Record<string, string> = {
            'openai': '/providers/openai.png',
            'azure': '/providers/azure.png',
            'google': '/providers/gemini.png',
            'anthropic': '/providers/anthropic.png',
            'xai': '/providers/xai.png',
            'mistral': '/providers/mistral.png',
            'elevenlabs': '/providers/elevenlabs.png',
            'perplexity': '/providers/perplexity.png',
            'selfhosted': '/providers/self-hosted.png',
            'deepseek': '/providers/deepseek.png',
          }
          // logic remains...
          const logo = provider?.icon_url || (provider ? LOGO_MAP[provider.provider_type] : undefined)

          builtNodes.push({
            id: plannerNodeId,
            type: 'model',
            position: getPosition(plannerNodeId, { x: 80 + HORIZONTAL_GAP * 3 + 120, y: serviceY - 35 }),
            data: {
              name: plannerModel.model_name,
              provider: provider?.provider_type || 'unknown',
              modality: plannerModel.modality || 'text',
              isLeader: false,
              isPlanner: true, // Show PLANNER badge
              weight: 100,
              is_healthy: plannerModel.is_healthy,
              health_status: plannerModel.health_status,
              logo: logo
            }
          })

          // Edge: Service -> Planner
          builtEdges.push({
            id: `e-${serviceId}-${plannerNodeId}`,
            source: serviceId,
            target: plannerNodeId,
            type: 'default',
            animated: true,
            style: {
              stroke: '#6366f1',
              strokeWidth: 2.5,
              strokeDasharray: '8 4',
            },
          })

          // 2. Add Tool Models AND MCP Servers (behind Planner)
          const toolModels = serviceModels.filter((m: any) => m.model_id !== plannerModel.model_id)
          const mcpServers = serviceWithModels.mcp_servers || []

          const totalTools = toolModels.length + mcpServers.length

          if (totalTools > 0) {
            const toolVerticalGap = 100
            const totalToolHeight = (totalTools - 1) * toolVerticalGap
            const toolStartY = serviceY - totalToolHeight / 2

            let currentIndex = 0

            // Helper to add tool node
            const addToolNode = (id: string, name: string, type: 'model' | 'mcp', info: any, logo?: string) => {
              builtNodes.push({
                id: id,
                type: 'model', // Reuse model node for visual consistency
                position: getPosition(id, { x: 80 + HORIZONTAL_GAP * 3 + 120 + 250, y: toolStartY + currentIndex * toolVerticalGap - 35 }),
                data: {
                  name: name,
                  provider: type === 'mcp' ? 'MCP' : (info.provider || 'unknown'),
                  modality: type === 'mcp' ? 'tool' : (info.modality || 'text'),
                  isLeader: false,
                  isPlanner: false,
                  weight: 0,
                  is_healthy: info.is_healthy ?? (type === 'mcp' && info.status === 'connected'),
                  health_status: info.health_status || info.status,
                  logo: logo
                }
              })

              // Edge: Planner -> Tool/MCP
              builtEdges.push({
                id: `e-${plannerNodeId}-${id}`,
                source: plannerNodeId,
                target: id,
                type: 'default',
                animated: true,
                style: {
                  stroke: type === 'mcp' ? '#f59e0b' : '#4b5563', // Amber for MCP, Gray for Models
                  strokeWidth: 1.5,
                  strokeDasharray: '4 4',
                },
              })

              currentIndex++
            }

            // Add Model Tools
            toolModels.forEach((model: any) => {
              const toolNodeId = `model-${service.name}-${model.model_id}`
              const toolInfo = allModels.find((m: any) => m.id === model.model_id)
              const toolProvider = providersData.find((p: any) => p.id === toolInfo?.provider)
              const toolLogo = toolProvider?.icon_url || (toolProvider ? LOGO_MAP[toolProvider.provider_type] : undefined)

              addToolNode(toolNodeId, model.model_name, 'model', {
                ...model,
                provider: toolProvider?.provider_type,
                modality: model.modality
              }, toolLogo)
            })

            // Add MCP Servers
            mcpServers.forEach((server: any) => {
              const serverNodeId = `mcp-${service.name}-${server.id}`
              // Use a default MCP logo or specific based on name
              const mcpLogo = '/logos/mcp.png' // Ensure this exists or fallback?
              // For now, let's use a generic icon in the node if logo fails, or just 'MCP' text provider

              addToolNode(serverNodeId, server.name, 'mcp', server, undefined)
            })
          }
        }
      } else {
        // Standard POOL layout (existing logic)
        const modelCount = serviceModels.length
        const totalModelHeight = modelCount > 1 ? (modelCount - 1) * MODEL_VERTICAL_GAP : 0
        const modelStartY = serviceY - totalModelHeight / 2

        serviceModels.forEach((model: any, modelIndex: number) => {
          const modelId = `model-${service.name}-${model.model_id}`

          // Calculate routing order based on strategy
          const routingOrder = getRoutingOrder(service.strategy, serviceModels)
          const isFirstByStrategy = routingOrder[0]?.model_id === model.model_id

          // For weighted strategies, no single "leader"
          const isWeightedStrategy = service.strategy?.toLowerCase().includes('weighted') ||
            service.strategy?.toLowerCase() === 'random' ||
            service.strategy?.toLowerCase() === 'pool'

          const modelY = modelStartY + modelIndex * MODEL_VERTICAL_GAP

          // Find provider info for this model
          const modelInfo = allModels.find((m: any) => m.id === model.model_id)
          const provider = providersData.find((p: any) => p.id === modelInfo?.provider)

          // Logo mapping
          const LOGO_MAP: Record<string, string> = {
            'openai': '/providers/openai.png',
            'azure': '/providers/azure.png',
            'google': '/providers/gemini.png',
            'anthropic': '/providers/anthropic.png',
            'xai': '/providers/xai.png',
            'mistral': '/providers/mistral.png',
            'elevenlabs': '/providers/elevenlabs.png',
            'perplexity': '/providers/perplexity.png',
            'selfhosted': '/providers/self-hosted.png',
            'deepseek': '/providers/deepseek.png',
          }
          const logo = provider?.icon_url || (provider ? LOGO_MAP[provider.provider_type] : undefined)

          builtNodes.push({
            id: modelId,
            type: 'model',
            position: getPosition(modelId, { x: 80 + HORIZONTAL_GAP * 3 + 120, y: modelY - 35 }),
            data: {
              name: model.model_name,
              provider: provider?.provider_type || 'unknown',
              modality: model.modality || 'text',
              isLeader: isFirstByStrategy && !isWeightedStrategy,
              weight: model.weight || 100,
              is_healthy: model.is_healthy,
              health_status: model.health_status,
              logo: logo
            }
          })

          // Green animated edge for first-by-strategy, gray for others
          builtEdges.push({
            id: `e-${serviceId}-${modelId}`,
            source: serviceId,
            target: modelId,
            type: 'default',
            animated: isFirstByStrategy && !isWeightedStrategy,
            style: {
              stroke: isFirstByStrategy && !isWeightedStrategy ? '#10b981' : '#4b5563',
              strokeWidth: isFirstByStrategy && !isWeightedStrategy ? 2.5 : 1.5,
              strokeDasharray: isFirstByStrategy && !isWeightedStrategy ? '8 4' : '4 4',
            },
          })
        })
      }
    })

    setNodes(builtNodes)
    setEdges(builtEdges)
  }

  return (
    <div className="min-h-screen bg-[#050508]">
      {/* Subtle grid pattern overlay */}
      <div className="fixed inset-0 opacity-[0.015] pointer-events-none"
        style={{ backgroundImage: 'radial-gradient(circle at 1px 1px, #fff 1px, transparent 0)', backgroundSize: '40px 40px' }} />

      {/* Ambient Background - more subtle for enterprise */}
      <div className="fixed inset-0 overflow-hidden pointer-events-none">
        <div className="absolute top-0 left-1/3 w-[800px] h-[800px] bg-cyan-500/[0.03] rounded-full blur-[200px]" />
        <div className="absolute bottom-0 right-1/3 w-[600px] h-[600px] bg-purple-500/[0.03] rounded-full blur-[200px]" />
        <div className="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-[1000px] h-[400px] bg-blue-500/[0.02] rounded-full blur-[200px]" />
      </div>

      {/* Header */}
      <motion.div
        initial={{ opacity: 0, y: -10 }}
        animate={{ opacity: 1, y: 0 }}
        className="relative z-10 border-b border-white/5 px-8 py-5 bg-gradient-to-r from-[#050508]/95 via-[#0a0a0f]/95 to-[#050508]/95 backdrop-blur-xl">
        <div className="flex items-center justify-between max-w-[1800px] mx-auto">
          <div className="flex items-center gap-6">
            <div className="w-10 h-10 rounded-xl bg-gradient-to-br from-cyan-400/20 to-purple-400/20 border border-white/10 flex items-center justify-center">
              <span className="text-lg">⚡</span>
            </div>
            <div>
              <h1 className="text-2xl font-bold text-white tracking-tight">
                System Overview
              </h1>
              <p className="text-sm text-slate-500">Real-time visualization of your AI gateway topology</p>
            </div>
          </div>

          <div className="flex items-center gap-3">
            <Card className="px-4 py-2.5 bg-[#0a0a0f]/80 border-cyan-400/20 backdrop-blur-xl">
              <div className="flex items-center gap-3">
                <div className="w-2 h-2 rounded-full bg-cyan-400 animate-pulse" />
                <span className="text-xs text-slate-400">Providers</span>
                <span className="text-lg font-bold text-cyan-400">{providers}</span>
              </div>
            </Card>
            <Card className="px-4 py-2.5 bg-[#0a0a0f]/80 border-amber-400/20 backdrop-blur-xl">
              <div className="flex items-center gap-3">
                <div className="w-2 h-2 rounded-full bg-amber-400 animate-pulse" />
                <span className="text-xs text-slate-400">MCP Servers</span>
                <span className="text-lg font-bold text-amber-400">{mcpServers}</span>
              </div>
            </Card>
            <Card className="px-4 py-2.5 bg-[#0a0a0f]/80 border-purple-400/20 backdrop-blur-xl">
              <div className="flex items-center gap-3">
                <div className="w-2 h-2 rounded-full bg-purple-400 animate-pulse" />
                <span className="text-xs text-slate-400">Services</span>
                <span className="text-lg font-bold text-purple-400">{services}</span>
              </div>
            </Card>
            <Card className="px-4 py-2.5 bg-[#0a0a0f]/80 border-emerald-400/20 backdrop-blur-xl">
              <div className="flex items-center gap-3">
                <div className="w-2 h-2 rounded-full bg-emerald-400 animate-pulse" />
                <span className="text-xs text-slate-400">Models</span>
                <span className="text-lg font-bold text-emerald-400">{models}</span>
              </div>
            </Card>
          </div>
        </div>
      </motion.div>

      {/* ReactFlow Visualization */}
      <div className="h-[calc(100vh-85px)] relative z-10">
        <ReactFlow
          nodes={nodes}
          edges={edges}
          onNodesChange={handleNodesChange}
          onEdgesChange={onEdgesChange}
          nodeTypes={nodeTypes}
          connectionLineType={ConnectionLineType.SmoothStep}
          nodesDraggable={true}
          nodesConnectable={false}
          elementsSelectable={true}
          fitView
          fitViewOptions={{ padding: 0.12, minZoom: 0.5, maxZoom: 1.2 }}
          minZoom={0.3}
          maxZoom={2}
          defaultEdgeOptions={{
            type: 'default',
            animated: true,
          }}
          proOptions={{ hideAttribution: true }}
          onNodeClick={(e, node) => {
            if (node.type === 'service') {
              // Navigate to edit service
              // Assuming ID is service-ServiceName
              const serviceName = node.id.replace('service-', '')
              // We need to encode if needed, but simple for now
              // Wait, router.push needs to be imported or available.
              // It is available in Home component scope.
              // But onNodeClick logic:
              window.location.href = '/services?edit=' + serviceName // Or use router if accessible
            }
          }}
        >
          <Background color="#18181b" gap={50} size={1} />
          <Controls
            className="!bg-[#0a0a0f]/90 !backdrop-blur-xl !border !border-white/10 !rounded-xl !shadow-xl"
            showInteractive={false}
          >
            <ControlButton
              onClick={() => {
                // Clear saved positions
                nodePositionsRef.current = {}
                // Reload data to trigger layout recalculation
                loadData()
              }}
              title="Reset Layout"
            >
              <span className="text-lg">⟲</span>
            </ControlButton>
          </Controls>

          {providers === 0 && services === 0 && (
            <Panel position="top-center">
              <motion.div
                initial={{ opacity: 0, scale: 0.95 }}
                animate={{ opacity: 1, scale: 1 }}
                className="mt-12 p-8 rounded-2xl bg-gradient-to-br from-[#0a0a0f]/95 to-[#05050a]/95 backdrop-blur-2xl border border-white/10 text-center shadow-2xl max-w-md">
                <div className="w-16 h-16 mx-auto mb-4 rounded-2xl bg-gradient-to-br from-cyan-400/20 to-purple-400/20 flex items-center justify-center">
                  <span className="text-3xl">🏗️</span>
                </div>
                <div className="text-lg font-semibold text-white mb-2">Begin Your Configuration</div>
                <div className="text-sm text-slate-500 mb-4">
                  Add providers and services to visualize your intelligent routing topology
                </div>
                <button
                  onClick={() => router.push('/providers')}
                  className="px-4 py-2 bg-gradient-to-r from-cyan-400 to-cyan-600 text-white rounded-lg text-sm font-semibold shadow-lg shadow-cyan-500/20 hover:scale-105 transition-transform">
                  Connect Provider
                </button>
              </motion.div>
            </Panel>
          )}

          {/* Flow Labels */}
          <Panel position="top-left">
            <motion.div
              initial={{ opacity: 0, x: -10 }}
              animate={{ opacity: 1, x: 0 }}
              transition={{ delay: 0.2 }}
              className="flex items-center gap-6 px-5 py-3 rounded-xl bg-[#0a0a0f]/80 backdrop-blur-xl border border-white/5">
              <div className="flex items-center gap-2">
                <svg width="32" height="4">
                  <line x1="0" y1="2" x2="32" y2="2" stroke="#22d3ee" strokeWidth="2.5" strokeDasharray="8 4">
                    <animate attributeName="stroke-dashoffset" from="0" to="-12" dur="0.5s" repeatCount="indefinite" />
                  </line>
                </svg>
                <span className="text-xs text-slate-500">Request</span>
              </div>
              <div className="flex items-center gap-2">
                <svg width="32" height="4">
                  <line x1="0" y1="2" x2="32" y2="2" stroke="#6366f1" strokeWidth="2" strokeDasharray="6 4">
                    <animate attributeName="stroke-dashoffset" from="0" to="-10" dur="0.5s" repeatCount="indefinite" />
                  </line>
                </svg>
                <span className="text-xs text-slate-500">Route</span>
              </div>
              <div className="flex items-center gap-2">
                <svg width="32" height="4">
                  <line x1="0" y1="2" x2="32" y2="2" stroke="#10b981" strokeWidth="2.5" strokeDasharray="8 4">
                    <animate attributeName="stroke-dashoffset" from="0" to="-12" dur="0.5s" repeatCount="indefinite" />
                  </line>
                </svg>
                <span className="text-xs text-slate-500">Will Respond First</span>
              </div>
              <div className="flex items-center gap-2">
                <svg width="32" height="4">
                  <line x1="0" y1="2" x2="32" y2="2" stroke="#4b5563" strokeWidth="1.5" strokeDasharray="4 4" />
                </svg>
                <span className="text-xs text-slate-500">Fallback</span>
              </div>
            </motion.div>
          </Panel>

          {/* Status indicator */}
          <Panel position="bottom-right">
            <motion.div
              initial={{ opacity: 0 }}
              animate={{ opacity: 1 }}
              transition={{ delay: 0.4 }}
              className="px-4 py-2 rounded-lg bg-[#0a0a0f]/80 backdrop-blur-xl border border-white/5 flex items-center gap-2">
              <div className="w-2 h-2 rounded-full bg-emerald-400 animate-pulse" />
              <span className="text-xs text-slate-500">System Operational</span>
            </motion.div>
          </Panel>
        </ReactFlow>
      </div>

      {/* Custom CSS for animated dashes */}
      <style jsx global>{`
        .react-flow__edge-path {
          stroke-linecap: round;
        }
        .react-flow__edge.animated path {
          animation: dashmove 0.5s linear infinite;
        }
        
        /* Custom Controls Styling */
        .react-flow__controls-button {
          background: transparent !important;
          border-bottom: 1px solid rgba(255, 255, 255, 0.1) !important;
          border-radius: 0 !important;
        }
        .react-flow__controls-button:last-child {
          border-bottom: none !important;
        }
        .react-flow__controls-button:hover {
          background: rgba(255, 255, 255, 0.05) !important;
        }
        .react-flow__controls-button svg {
          fill: white !important;
          max-width: 14px;
          max-height: 14px;
        }

        @keyframes dashmove {
          to {
            stroke-dashoffset: -12;
          }
        }
      `}</style>
    </div>
  )
}
