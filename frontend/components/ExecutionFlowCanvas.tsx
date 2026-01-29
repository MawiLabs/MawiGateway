'use client'

import { useEffect, useState } from 'react'
import {
    ReactFlow,
    Background,
    Controls,
    Panel,
    Node,
    Edge,
    useNodesState,
    useEdgesState,
} from '@xyflow/react'
import '@xyflow/react/dist/style.css'

import UserNode from './UserNode'
import GatewayNode from './GatewayNode'
import ServiceNode from './ServiceNode'
import ModelNode from './ModelNode'

const nodeTypes = {
    user: UserNode,
    gateway: GatewayNode,
    service: ServiceNode,
    model: ModelNode,
}

interface ModelGroup {
    id: string
    name: string
    provider: string
}

interface Model {
    id: string
    name: string
    model_group_id: string
    modality: string
}

interface Service {
    name: string
    service_type: string
    description?: string
    guardrails?: string
    planner_model_id?: string
}

export default function ExecutionFlowCanvas() {
    const [modelGroups, setModelGroups] = useState<ModelGroup[]>([])
    const [models, setModels] = useState<Model[]>([])
    const [services, setServices] = useState<Service[]>([])
    const [nodes, setNodes, onNodesChange] = useNodesState<Node>([])
    const [edges, setEdges, onEdgesChange] = useEdgesState<Edge>([])

    // Fetch data from backend
    useEffect(() => {
        Promise.all([
            fetch('http://127.0.0.1:8030/v1/model-groups').then(res => res.json()),
            fetch('http://127.0.0.1:8030/v1/models').then(res => res.json()),
            fetch('http://127.0.0.1:8030/v1/services').then(res => res.json()),
        ])
            .then(([groups, mods, servs]) => {
                setModelGroups(groups)
                setModels(mods)
                setServices(servs)
            })
            .catch(console.error)
    }, [])

    // Build flow diagram
    useEffect(() => {
        const builtNodes: Node[] = [
            { id: 'user', type: 'user', position: { x: 50, y: 300 }, data: {} },
            {
                id: 'gateway',
                type: 'gateway',
                position: { x: 350, y: 250 },
                data: { models: modelGroups.map(g => g.name) }
            },
        ]

        const builtEdges: Edge[] = [
            {
                id: 'e-user-gateway',
                source: 'user',
                target: 'gateway',
                type: 'smoothstep',
                style: { stroke: '#a78bfa', strokeWidth: 2 }
            },
        ]

        // Add services
        services.forEach((service, i) => {
            const serviceId = `service-${service.name}`
            const serviceTypeUpper = service.service_type?.toUpperCase() || ''
            // Check if explicitly Agentic OR if it has a planner assigned (fallback for legacy types)
            const isAgentic = serviceTypeUpper === 'AGENTIC' || !!service.planner_model_id

            builtNodes.push({
                id: serviceId,
                type: 'service',
                position: { x: 750, y: 100 + i * (isAgentic ? 300 : 180) }, // More space for agentic
                data: {
                    label: service.name,
                    type: service.service_type,
                    guardrails: service.guardrails ? JSON.parse(service.guardrails).length > 0 : false
                }
            })

            const color = service.service_type === 'chat' ? '#10b981' :
                service.service_type === 'audio' ? '#8b5cf6' :
                    isAgentic ? '#ec4899' : '#ef4444'

            builtEdges.push({
                id: `e-gateway-${service.name}`,
                source: 'gateway',
                target: serviceId,
                type: 'smoothstep',
                style: { stroke: color, strokeWidth: 2 }
            })

            if (isAgentic) {
                // AGENTIC TOPOLOGY: Service -> Planner -> Tools
                const planner = models.find(m => m.id === service.planner_model_id)
                // Tools: pick models NOT planner
                const toolModels = models
                    .filter(m => m.id !== service.planner_model_id)
                    .slice(0, 3)

                if (planner) {
                    const plannerGroupId = modelGroups.find(g => g.id === planner.model_group_id)?.id
                    const plannerNodeId = `model-${service.name}-planner`

                    // Planner Node
                    builtNodes.push({
                        id: plannerNodeId,
                        type: 'model',
                        position: { x: 1100, y: 80 + i * 300 }, // Aligned with service top
                        data: {
                            label: `${planner.name} (planner)`,
                            name: planner.name,
                            provider: modelGroups.find(g => g.id === planner.model_group_id)?.provider || 'Unknown',
                            modality: planner.modality,
                            weight: 100,
                            isLeader: false,
                            isPlanner: true
                        }
                    })

                    // Edge: Service -> Planner
                    builtEdges.push({
                        id: `e-${serviceId}-planner`,
                        source: serviceId,
                        target: plannerNodeId,
                        type: 'smoothstep',
                        style: { stroke: '#10b981', strokeWidth: 2, strokeDasharray: '5,5' },
                        animated: true,
                        label: 'Planning'
                    })

                    // Tool Nodes (Fan out from Planner)
                    toolModels.forEach((tool, k) => {
                        const toolNodeId = `model-${service.name}-tool-${k}`
                        const toolGroup = modelGroups.find(g => g.id === tool.model_group_id)

                        builtNodes.push({
                            id: toolNodeId,
                            type: 'model',
                            position: { x: 1450, y: 50 + i * 300 + k * 80 }, // Shifted right and spread vertical
                            data: {
                                label: `${tool.name} (${toolGroup?.name || 'Tool'})`,
                                name: tool.name,
                                provider: toolGroup?.provider || 'Unknown',
                                modality: tool.modality,
                                weight: 0,
                                isLeader: false,
                                role: 'tool',
                                status: 'idle'
                            }
                        })

                        // Edge: Planner -> Tool
                        builtEdges.push({
                            id: `e-${plannerNodeId}-${toolNodeId}`,
                            source: plannerNodeId,
                            target: toolNodeId,
                            type: 'smoothstep',
                            style: { stroke: '#64748b', strokeWidth: 1.5, strokeDasharray: '4,4' },
                            label: 'Call Tool'
                        })
                    })
                }
            } else {
                // STANDARD POOL TOPOLOGY
                const serviceModels = models.filter(m =>
                    (service.service_type === 'chat' && m.modality === 'text') ||
                    (service.service_type === 'audio' && m.modality === 'audio') ||
                    (service.service_type === 'video' && m.modality === 'video')
                ).slice(0, 3)

                serviceModels.forEach((model, j) => {
                    const modelId = `model-${service.name}-${model.id}`
                    const group = modelGroups.find(g => g.id === model.model_group_id)

                    builtNodes.push({
                        id: modelId,
                        type: 'model',
                        position: { x: 1100, y: 80 + i * 180 + j * 60 },
                        data: {
                            label: `${model.name} (${group?.name || 'Unknown'})`,
                            role: j === 0 ? 'leader' : 'worker',
                            status: 'active'
                        }
                    })

                    if (j === 0) {
                        builtEdges.push({
                            id: `e-${serviceId}-${modelId}`,
                            source: serviceId,
                            target: modelId,
                            type: 'smoothstep',
                            style: { stroke: '#14b8a6', strokeWidth: 2 },
                            label: 'Leader'
                        })
                    } else {
                        const prevModelId = `model-${service.name}-${serviceModels[j - 1].id}`
                        builtEdges.push({
                            id: `e-${prevModelId}-${modelId}`,
                            source: prevModelId,
                            target: modelId,
                            type: 'smoothstep',
                            style: { stroke: '#f59e0b', strokeWidth: 2, strokeDasharray: '5, 5' },
                            label: `Failover ${j}`
                        })
                    }
                })
            }
        })

        setNodes(builtNodes)
        setEdges(builtEdges)
    }, [modelGroups, models, services, setNodes, setEdges])

    return (
        <div className="w-full h-full relative">
            <ReactFlow
                nodes={nodes}
                edges={edges}
                onNodesChange={onNodesChange}
                onEdgesChange={onEdgesChange}
                nodeTypes={nodeTypes}
                nodesDraggable={false}
                nodesConnectable={false}
                elementsSelectable={false}
                fitView
                className="bg-gray-900"
                proOptions={{ hideAttribution: true }}
            >
                <Background color="#374151" gap={16} className="bg-gray-900" />
                <Controls className="bg-gray-800 border border-gray-700" />

                <Panel position="top-right" className="bg-gray-800 border-2 border-teal-500 rounded-lg px-4 py-2 shadow-xl">
                    <div className="flex items-center gap-2">
                        <div className="w-3 h-3 bg-teal-500 rounded-full animate-pulse" />
                        <span className="text-teal-400 font-bold text-sm">LIVE</span>
                    </div>
                    <div className="text-gray-400 text-xs mt-1">
                        {modelGroups.length} groups • {models.length} models
                    </div>
                </Panel>

                <Panel position="top-left" className="bg-gray-800/90 border border-gray-700 rounded-lg px-4 py-3 shadow-xl">
                    <div className="text-white font-bold text-sm mb-2">Model Groups</div>
                    <div className="space-y-1">
                        {modelGroups.map(group => (
                            <div key={group.id} className="flex items-center gap-2 text-xs">
                                <div className="w-2 h-2 bg-blue-400 rounded-full" />
                                <span className="text-gray-300">{group.name}</span>
                                <span className="text-gray-500">({group.provider})</span>
                            </div>
                        ))}
                    </div>
                </Panel>
            </ReactFlow>
        </div>
    )
}
