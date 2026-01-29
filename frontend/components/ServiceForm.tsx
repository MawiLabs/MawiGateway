'use client'

import { useState } from 'react'

interface Service {
    name: string
    service_type: string
    description?: string
    strategy: string
    guardrails: string[]
}

interface ServiceFormProps {
    onClose: () => void
    onSubmit: (service: Service) => void
    initialData?: Service
}

export default function ServiceForm({ onClose, onSubmit, initialData }: ServiceFormProps) {
    const [name, setName] = useState(initialData?.name || '')
    const [serviceType, setServiceType] = useState(initialData?.service_type || 'chat')
    const [description, setDescription] = useState(initialData?.description || '')
    const [strategy, setStrategy] = useState(initialData?.strategy || 'weighted')

    const handleSubmit = (e: React.FormEvent) => {
        e.preventDefault()
        onSubmit({
            name,
            service_type: serviceType,
            description: description || undefined,
            strategy,
            guardrails: [],
        })
    }

    return (
        <div className="fixed inset-0 bg-black/60 flex items-center justify-center z-50" onClick={onClose}>
            <div className="card p-6 w-full max-w-md" onClick={(e) => e.stopPropagation()}>
                <h2 className="text-lg font-semibold text-white mb-4">
                    {initialData ? 'Edit Service' : 'New Service'}
                </h2>

                <form onSubmit={handleSubmit} className="space-y-4">
                    <div>
                        <label className="block text-sm font-medium text-gray-400 mb-2">
                            Service Name
                        </label>
                        <input
                            type="text"
                            value={name}
                            onChange={(e) => setName(e.target.value)}
                            placeholder="e.g., customer-chat"
                            className="w-full px-3 py-2 bg-[#0a0a0a] border border-gray-700 rounded-md text-white placeholder-gray-600 focus:border-sky-500 focus:outline-none"
                            required
                        />
                    </div>

                    <div>
                        <label className="block text-sm font-medium text-gray-400 mb-2">
                            Service Type
                        </label>
                        <div className="space-y-2">
                            {[
                                { value: 'chat', label: 'Chat', icon: '💬' },
                                { value: 'audio', label: 'Audio', icon: '🎵' },
                                { value: 'video', label: 'Video', icon: '🎬' },
                            ].map((type) => (
                                <button
                                    key={type.value}
                                    type="button"
                                    onClick={() => setServiceType(type.value)}
                                    className={`w-full px-4 py-2 rounded-md text-sm font-medium transition-all flex items-center gap-2 ${serviceType === type.value
                                        ? 'bg-sky-500 text-white'
                                        : 'bg-[#1a1a1a] text-gray-300 hover:bg-gray-800'
                                        }`}
                                >
                                    <span>{type.icon}</span>
                                    <span>{type.label}</span>
                                </button>
                            ))}
                        </div>
                    </div>

                    <div>
                        <label className="block text-sm font-medium text-gray-400 mb-2">
                            Strategy
                        </label>
                        <select
                            value={strategy}
                            onChange={(e) => setStrategy(e.target.value)}
                            className="w-full px-3 py-2 bg-[#0a0a0a] border border-gray-700 rounded-md text-white focus:border-sky-500 focus:outline-none"
                        >
                            <option value="weighted">Weighted Distribution</option>
                            <option value="leader-failover">Leader with Failover</option>
                            <option value="round-robin">Round Robin</option>
                        </select>
                    </div>

                    <div>
                        <label className="block text-sm font-medium text-gray-400 mb-2">
                            Description (Optional)
                        </label>
                        <textarea
                            value={description}
                            onChange={(e) => setDescription(e.target.value)}
                            placeholder="Service purpose..."
                            rows={2}
                            className="w-full px-3 py-2 bg-[#0a0a0a] border border-gray-700 rounded-md text-white placeholder-gray-600 focus:border-sky-500 focus:outline-none resize-none"
                        />
                    </div>

                    <div className="flex gap-3 pt-4">
                        <button
                            type="button"
                            onClick={onClose}
                            className="flex-1 px-4 py-2 bg-gray-700 hover:bg-gray-600 text-white rounded-md font-medium transition-colors"
                        >
                            Cancel
                        </button>
                        <button
                            type="submit"
                            className="flex-1 px-4 py-2 bg-sky-500 hover:bg-sky-600 text-white rounded-md font-medium transition-colors"
                        >
                            {initialData ? 'Update' : 'Create'}
                        </button>
                    </div>
                </form>
            </div>
        </div>
    )
}
