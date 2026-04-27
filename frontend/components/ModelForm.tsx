'use client'

import { useState } from 'react'
import { toast } from 'sonner'
import { MessageSquare, Music, Film, Image as ImageIcon } from 'lucide-react'
import { Button, Input, Modal } from '@/components/ui'

type Modality = 'text' | 'audio' | 'video' | 'image'

interface ModelFormProps {
    isOpen: boolean
    onClose: () => void
    onSubmit: (model: {
        name: string
        provider: string
        modality: string
        description?: string
    }) => Promise<void>
    providers: { id: string; name: string }[]
}

const MODALITIES: { value: Modality; label: string; icon: React.ReactNode }[] = [
    { value: 'text', label: 'Text', icon: <MessageSquare className="w-4 h-4" strokeWidth={2} /> },
    { value: 'image', label: 'Image', icon: <ImageIcon className="w-4 h-4" strokeWidth={2} /> },
    { value: 'audio', label: 'Audio', icon: <Music className="w-4 h-4" strokeWidth={2} /> },
    { value: 'video', label: 'Video', icon: <Film className="w-4 h-4" strokeWidth={2} /> },
]

/**
 * Canonical model-creation form. Same template as ProviderForm:
 *   - `Modal` wrapper for a11y + brand presence
 *   - `Input` for text fields
 *   - `Button loading={...}` for submit state
 *   - sonner toast on success / error
 *   - Lucide icons for the modality picker (replaces emoji 💬🎵🎬)
 */
export default function ModelForm({
    isOpen,
    onClose,
    onSubmit,
    providers,
}: ModelFormProps) {
    const [name, setName] = useState('')
    const [providerId, setProviderId] = useState(providers[0]?.id || '')
    const [modality, setModality] = useState<Modality>('text')
    const [description, setDescription] = useState('')
    const [submitting, setSubmitting] = useState(false)
    const [nameError, setNameError] = useState<string | undefined>()

    const reset = () => {
        setName('')
        setProviderId(providers[0]?.id || '')
        setModality('text')
        setDescription('')
        setNameError(undefined)
    }

    const handleClose = () => {
        if (submitting) return
        reset()
        onClose()
    }

    const handleSubmit = async (e: React.FormEvent) => {
        e.preventDefault()
        if (!name.trim()) {
            setNameError('Model name is required')
            return
        }
        setNameError(undefined)
        setSubmitting(true)
        try {
            await onSubmit({
                name: name.trim(),
                provider: providerId,
                modality,
                description: description || undefined,
            })
            toast.success('Model added', {
                description: `"${name.trim()}" is ready for service binding.`,
            })
            reset()
            onClose()
        } catch (err) {
            const msg = err instanceof Error ? err.message : 'Could not create model'
            toast.error('Model creation failed', { description: msg })
        } finally {
            setSubmitting(false)
        }
    }

    // Special-case: no providers yet. Show a helpful empty state inside
    // the modal instead of a useless form.
    if (providers.length === 0) {
        return (
            <Modal
                isOpen={isOpen}
                onClose={onClose}
                title="No providers yet"
                description="Add a provider before you can register models against it."
                size="sm"
                footer={
                    <Button variant="primary" onClick={onClose}>
                        Got it
                    </Button>
                }
            >
                <p className="text-sm text-slate-400 leading-relaxed">
                    A model belongs to a provider (OpenAI, Anthropic, your own
                    self-hosted endpoint, etc.). Once you have at least one provider
                    configured, you can register models against it here.
                </p>
            </Modal>
        )
    }

    return (
        <Modal
            isOpen={isOpen}
            onClose={handleClose}
            title="Add Model"
            description="Register a new model your services can route to"
            size="md"
            footer={
                <>
                    <Button
                        type="button"
                        variant="ghost"
                        onClick={handleClose}
                        disabled={submitting}
                    >
                        Cancel
                    </Button>
                    <Button
                        type="submit"
                        variant="primary"
                        loading={submitting}
                        onClick={handleSubmit as unknown as React.MouseEventHandler<HTMLButtonElement>}
                    >
                        {submitting ? 'Creating model...' : 'Create model'}
                    </Button>
                </>
            }
        >
            <form onSubmit={handleSubmit} className="space-y-5">
                <Input
                    label="Model name"
                    type="text"
                    value={name}
                    onChange={(e) => {
                        setName(e.target.value)
                        if (nameError) setNameError(undefined)
                    }}
                    placeholder="gpt-4o"
                    helperText="The exact model identifier the provider expects"
                    error={nameError}
                    required
                    autoFocus
                />

                <div>
                    <label className="block text-sm font-medium text-slate-300 mb-2">
                        Provider
                    </label>
                    <select
                        value={providerId}
                        onChange={(e) => setProviderId(e.target.value)}
                        className="w-full px-4 py-3 bg-black border border-white/10 rounded-xl text-white outline-none focus:border-cyan-400 focus:ring-4 focus:ring-cyan-400/20 transition-all duration-200"
                        required
                    >
                        {providers.map((p) => (
                            <option key={p.id} value={p.id}>
                                {p.name}
                            </option>
                        ))}
                    </select>
                </div>

                <div>
                    <label className="block text-sm font-medium text-slate-300 mb-2">
                        Modality
                    </label>
                    <div className="grid grid-cols-4 gap-2">
                        {MODALITIES.map((m) => {
                            const active = modality === m.value
                            return (
                                <button
                                    key={m.value}
                                    type="button"
                                    onClick={() => setModality(m.value)}
                                    aria-pressed={active}
                                    className={`flex flex-col items-center justify-center gap-1.5 px-3 py-3 rounded-xl text-xs font-semibold transition-all border
                                        ${
                                            active
                                                ? 'bg-cyan-400/10 text-cyan-300 border-cyan-400/40 shadow-[0_0_20px_rgba(34,211,238,0.18)]'
                                                : 'bg-[#0a0a0a] text-slate-300 border-white/10 hover:bg-[#0f0f0f] hover:border-white/20'
                                        }`}
                                >
                                    {m.icon}
                                    {m.label}
                                </button>
                            )
                        })}
                    </div>
                </div>

                <div>
                    <label className="block text-sm font-medium text-slate-300 mb-2">
                        Description{' '}
                        <span className="text-slate-500 font-normal">(optional)</span>
                    </label>
                    <textarea
                        value={description}
                        onChange={(e) => setDescription(e.target.value)}
                        placeholder="When should services pick this model? Notes for your team."
                        rows={2}
                        className="w-full px-4 py-3 bg-black border border-white/10 rounded-xl text-white placeholder-slate-500 transition-all duration-200 outline-none focus:border-cyan-400 focus:ring-4 focus:ring-cyan-400/20 resize-none"
                    />
                </div>
            </form>
        </Modal>
    )
}
