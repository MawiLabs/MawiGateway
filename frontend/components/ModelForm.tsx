'use client'

import { useState } from 'react'

interface ModelFormProps {
    onClose: () => void
    onSubmit: (model: { name: string; provider: string; modality: string; description?: string }) => void
    providers: { id: string; name: string }[]
}

export default function ModelForm({ onClose, onSubmit, providers }: ModelFormProps) {
    const [name, setName] = useState('')
    const [providerId, setProviderId] = useState(providers[0]?.id || '')
    const [modality, setModality] = useState('text')
    const [description, setDescription] = useState('')

    const handleSubmit = (e: React.FormEvent) => {
        e.preventDefault()
        onSubmit({
            name,
            provider: providerId,
            modality,
            description: description || undefined,
        })
    }

    if (providers.length === 0) {
        return (
            <div className="fixed inset-0 bg-black/60 flex items-center justify-center z-50" onClick={onClose}>
                <div className="card p-6 w-full max-w-md text-center" onClick={(e) => e.stopPropagation()}>
                    <h2 className="text-lg font-semibold text-white mb-4">No Providers</h2>
                    <p className="text-gray-400 mb-6">You need to create a provider first before adding models.</p>
                    <button
                        onClick={onClose}
                        className="px-4 py-2 bg-sky-500 hover:bg-sky-600 text-white rounded-md font-medium transition-colors"
                    >
                        Close
                    </button>
                </div>
            </div>
        )
    }

    return (
        <div className="fixed inset-0 bg-black/60 flex items-center justify-center z-50" onClick={onClose}>
            <div className="card p-6 w-full max-w-md" onClick={(e) => e.stopPropagation()}>
                <h2 className="text-lg font-semibold text-white mb-4">New Model</h2>

                <form onSubmit={handleSubmit} className="space-y-4">
                    <div>
                        <label className="block text-sm font-medium text-gray-400 mb-2">
                            Model Name
                        </label>
                        <input
                            type="text"
                            value={name}
                            onChange={(e) => setName(e.target.value)}
                            placeholder="e.g., GPT-4 Turbo"
                            className="w-full px-3 py-2 bg-[#0a0a0a] border border-gray-700 rounded-md text-white placeholder-gray-600 focus:border-sky-500 focus:outline-none"
                            required
                        />
                    </div>

                    <div>
                        <label className="block text-sm font-medium text-gray-400 mb-2">
                            Provider
                        </label>
                        <select
                            value={providerId}
                            onChange={(e) => setProviderId(e.target.value)}
                            className="w-full px-3 py-2 bg-[#0a0a0a] border border-gray-700 rounded-md text-white focus:border-sky-500 focus:outline-none"
                            required
                        >
                            {providers.map((provider) => (
                                <option key={provider.id} value={provider.id}>
                                    {provider.name}
                                </option>
                            ))}
                        </select>
                    </div>

                    <div>
                        <label className="block text-sm font-medium text-gray-400 mb-2">
                            Modality
                        </label>
                        <div className="grid grid-cols-3 gap-2">
                            {[
                                { value: 'text', label: '💬 Text' },
                                { value: 'audio', label: '🎵 Audio' },
                                { value: 'video', label: '🎬 Video' },
                            ].map((mod) => (
                                <button
                                    key={mod.value}
                                    type="button"
                                    onClick={() => setModality(mod.value)}
                                    className={`px-4 py-2 rounded-md text-sm font-medium transition-all ${modality === mod.value
                                            ? 'bg-sky-500 text-white'
                                            : 'bg-[#1a1a1a] text-gray-300 hover:bg-gray-800'
                                        }`}
                                >
                                    {mod.label}
                                </button>
                            ))}
                        </div>
                    </div>

                    <div>
                        <label className="block text-sm font-medium text-gray-400 mb-2">
                            Description (Optional)
                        </label>
                        <textarea
                            value={description}
                            onChange={(e) => setDescription(e.target.value)}
                            placeholder="Model details..."
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
                            Create
                        </button>
                    </div>
                </form>
            </div>
        </div>
    )
}
