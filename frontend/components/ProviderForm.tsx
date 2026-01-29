'use client'

import { useState } from 'react'

interface ProviderFormProps {
  onClose: () => void
  onSubmit: (provider: { name: string; provider_type: string; api_key?: string; description?: string }) => void
}

export default function ProviderForm({ onClose, onSubmit }: ProviderFormProps) {
  const [name, setName] = useState('')
  const [providerType, setProviderType] = useState('openai')
  const [apiKey, setApiKey] = useState('')
  const [description, setDescription] = useState('')

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault()
    onSubmit({
      name,
      provider_type: providerType,
      api_key: apiKey || undefined,
      description: description || undefined,
    })
  }

  // Only providers that the unified API supports
  const supportedProviders = [
    { value: 'openai', label: 'OpenAI' },
    { value: 'azure', label: 'Azure OpenAI' },
    { value: 'anthropic', label: 'Anthropic' },
    { value: 'google', label: 'Google AI' },
  ]

  return (
    <div className="fixed inset-0 bg-black/60 flex items-center justify-center z-50" onClick={onClose}>
      <div className="card p-6 w-full max-w-md" onClick={(e) => e.stopPropagation()}>
        <h2 className="text-lg font-semibold text-white mb-4">New Provider</h2>

        <form onSubmit={handleSubmit} className="space-y-4">
          <div>
            <label className="block text-sm font-medium text-gray-400 mb-2">
              Provider Name
            </label>
            <input
              type="text"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="e.g., OpenAI Production"
              className="w-full px-3 py-2 bg-[#0a0a0a] border border-gray-700 rounded-md text-white placeholder-gray-600 focus:border-sky-500 focus:outline-none"
              required
            />
          </div>

          <div>
            <label className="block text-sm font-medium text-gray-400 mb-2">
              Provider Type
            </label>
            <div className="grid grid-cols-2 gap-2">
              {supportedProviders.map((type) => (
                <button
                  key={type.value}
                  type="button"
                  onClick={() => setProviderType(type.value)}
                  className={`px-4 py-2 rounded-md text-sm font-medium transition-all ${
                    providerType === type.value
                      ? 'bg-sky-500 text-white'
                      : 'bg-[#1a1a1a] text-gray-300 hover:bg-gray-800'
                  }`}
                >
                  {type.label}
                </button>
              ))}
            </div>
          </div>

          <div>
            <label className="block text-sm font-medium text-gray-400 mb-2">
              API Key (Optional)
            </label>
            <input
              type="password"
              value={apiKey}
              onChange={(e) => setApiKey(e.target.value)}
              placeholder="sk-..."
              className="w-full px-3 py-2 bg-[#0a0a0a] border border-gray-700 rounded-md text-white placeholder-gray-600 focus:border-sky-500 focus:outline-none"
            />
          </div>

          <div>
            <label className="block text-sm font-medium text-gray-400 mb-2">
              Description (Optional)
            </label>
            <textarea
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              placeholder="Provider purpose..."
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
