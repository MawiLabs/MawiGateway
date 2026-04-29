'use client'

import { useState } from 'react'
import { toast } from 'sonner'
import { ShieldCheck } from 'lucide-react'
import { Button, Input, Modal, ProviderPicker } from '@/components/ui'
import { PROVIDER_BY_ID } from '@/lib/providerCatalog'

interface ProviderFormProps {
    isOpen: boolean
    onClose: () => void
    /**
     * Async callback. Resolve to indicate the provider was created
     * (modal will close + success toast fires); reject with an Error
     * to surface the message in an error toast and keep the modal open.
     */
    onSubmit: (provider: {
        name: string
        provider_type: string
        api_key?: string
        description?: string
    }) => Promise<void>
    /**
     * Provider IDs to surface in the picker's "Recent" hero row.
     * Typically the most-used providers from the user's request_logs.
     */
    recentProviderIds?: string[]
}

/**
 * Canonical provider-creation form.
 *
 * This is the reference template for every form in mawi-web:
 *   - Wraps in `Modal` (gets ESC + focus trap + a11y for free).
 *   - Uses `Input` for text fields (consistent focus ring, error
 *     state, helper text).
 *   - Uses `ProviderPicker` for the type selector (hero recents +
 *     searchable card grid; scales to hundreds of providers).
 *   - Submit button shows loading state via `Button loading={...}`.
 *   - Fires `toast.success()` / `toast.error()` via sonner so the
 *     user always knows what happened.
 *
 * Forms in feature pages (e.g. `app/providers/page.tsx`) should adopt
 * this pattern. The dead-code copy in this file remains as a search
 * target for future devs ("how do I write a form?").
 */
export default function ProviderForm({
    isOpen,
    onClose,
    onSubmit,
    recentProviderIds = ['openai', 'anthropic'],
}: ProviderFormProps) {
    const [name, setName] = useState('')
    const [providerType, setProviderType] = useState('openai')
    const [apiKey, setApiKey] = useState('')
    const [description, setDescription] = useState('')
    const [submitting, setSubmitting] = useState(false)
    const [nameError, setNameError] = useState<string | undefined>()

    const reset = () => {
        setName('')
        setProviderType('openai')
        setApiKey('')
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
            setNameError('Provider name is required')
            return
        }
        setNameError(undefined)
        setSubmitting(true)
        try {
            await onSubmit({
                name: name.trim(),
                provider_type: providerType,
                api_key: apiKey || undefined,
                description: description || undefined,
            })
            const friendly = PROVIDER_BY_ID.get(providerType)?.name ?? providerType
            toast.success(`${friendly} provider added`, {
                description: `"${name.trim()}" is ready to route requests.`,
            })
            reset()
            onClose()
        } catch (err) {
            const msg = err instanceof Error ? err.message : 'Could not create provider'
            toast.error('Provider creation failed', { description: msg })
        } finally {
            setSubmitting(false)
        }
    }

    return (
        <Modal
            isOpen={isOpen}
            onClose={handleClose}
            title="Add Provider"
            description="Connect a new AI provider to your gateway"
            size="lg"
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
                        {submitting ? 'Creating provider...' : 'Create provider'}
                    </Button>
                </>
            }
        >
            <form onSubmit={handleSubmit} className="space-y-5">
                <Input
                    label="Provider name"
                    type="text"
                    value={name}
                    onChange={(e) => {
                        setName(e.target.value)
                        if (nameError) setNameError(undefined)
                    }}
                    placeholder="OpenAI Production"
                    helperText='A friendly label, e.g. "OpenAI Production" or "Self-hosted Mistral"'
                    error={nameError}
                    required
                    autoFocus
                />

                <div>
                    <label className="block text-sm font-medium text-slate-300 mb-2">
                        Provider type
                    </label>
                    <ProviderPicker
                        value={providerType}
                        onChange={setProviderType}
                        recentIds={recentProviderIds}
                    />
                </div>

                <Input
                    label="API key"
                    type="password"
                    value={apiKey}
                    onChange={(e) => setApiKey(e.target.value)}
                    placeholder="sk-..."
                    icon={<ShieldCheck className="w-4 h-4" strokeWidth={2} />}
                    helperText="Stored encrypted with AES-256-GCM. Never logged."
                />

                <div>
                    <label className="block text-sm font-medium text-slate-300 mb-2">
                        Description{' '}
                        <span className="text-slate-500 font-normal">(optional)</span>
                    </label>
                    <textarea
                        value={description}
                        onChange={(e) => setDescription(e.target.value)}
                        placeholder="What is this provider used for? Notes for your team."
                        rows={2}
                        className="w-full px-4 py-3 bg-black border border-white/10 rounded-xl text-white placeholder-slate-500 transition-all duration-200 outline-none focus:border-cyan-400 focus:ring-4 focus:ring-cyan-400/20 resize-none"
                    />
                </div>
            </form>
        </Modal>
    )
}
