'use client'

import { useRef, useState, KeyboardEvent } from 'react'
import { X } from 'lucide-react'

interface ChipInputProps {
    label?: string
    helperText?: string
    placeholder?: string
    /** The current chip values. Source of truth lives in the parent. */
    value: string[]
    /** Called whenever the chip array changes (add or remove). */
    onChange: (next: string[]) => void
    /**
     * Optional validator. Returns an error string to reject the chip,
     * or null/undefined to accept it. Runs on every Enter / comma.
     * Example: enforce lowercase ASCII for service aliases.
     */
    validate?: (chip: string) => string | null | undefined
    /**
     * Optional comma-separated chip count cap. Defaults to no limit.
     * Useful for "max 5 aliases per service" style constraints.
     */
    maxChips?: number
    /**
     * Custom style for the chip container — e.g. cyan tint for aliases.
     * Defaults to neutral white/10. Pass a Tailwind class string.
     */
    chipClassName?: string
}

/**
 * Multi-value chip input. Type → press Enter or comma → text becomes a
 * chip. Backspace on empty input deletes the most recent chip. X icon
 * on each chip removes it.
 *
 * Used for service aliases and any other "list of short strings"
 * field where free-text + commas would be awkward.
 */
export function ChipInput({
    label,
    helperText,
    placeholder,
    value,
    onChange,
    validate,
    maxChips,
    chipClassName,
}: ChipInputProps) {
    const [draft, setDraft] = useState('')
    const [error, setError] = useState<string | null>(null)
    const inputRef = useRef<HTMLInputElement>(null)

    const commit = (raw: string) => {
        const trimmed = raw.trim()
        if (!trimmed) return false
        if (value.includes(trimmed)) {
            setError(`"${trimmed}" is already in the list`)
            return false
        }
        if (maxChips !== undefined && value.length >= maxChips) {
            setError(`At most ${maxChips} entries allowed`)
            return false
        }
        if (validate) {
            const v = validate(trimmed)
            if (v) {
                setError(v)
                return false
            }
        }
        onChange([...value, trimmed])
        setError(null)
        return true
    }

    const handleKeyDown = (e: KeyboardEvent<HTMLInputElement>) => {
        if (e.key === 'Enter' || e.key === ',') {
            e.preventDefault()
            if (commit(draft)) setDraft('')
        } else if (e.key === 'Backspace' && draft.length === 0 && value.length > 0) {
            // Remove the trailing chip when the user keeps hitting
            // backspace from an empty input — matches Notion / Slack
            // chip-input behaviour.
            onChange(value.slice(0, -1))
            setError(null)
        }
    }

    const removeAt = (idx: number) => {
        onChange(value.filter((_, i) => i !== idx))
        setError(null)
    }

    return (
        <div className="w-full">
            {label && (
                <label className="block text-sm font-medium text-slate-300 mb-2">
                    {label}
                </label>
            )}

            {/* Click anywhere on the bordered region to focus the input —
                gives the whole "field" a single tap target like a real
                input would have. */}
            <div
                onClick={() => inputRef.current?.focus()}
                className={`min-h-[44px] flex flex-wrap items-center gap-1.5 px-3 py-2 bg-black border rounded-xl
                            transition-all cursor-text
                            ${error
                                ? 'border-red-500 focus-within:ring-4 focus-within:ring-red-500/20'
                                : 'border-white/10 focus-within:border-cyan-400 focus-within:ring-4 focus-within:ring-cyan-400/20'}`}
            >
                {value.map((chip, idx) => (
                    <span
                        key={`${chip}-${idx}`}
                        className={
                            chipClassName ??
                            'inline-flex items-center gap-1 px-2 py-0.5 rounded-md text-xs font-medium bg-cyan-400/10 text-cyan-300 border border-cyan-400/30'
                        }
                    >
                        {chip}
                        <button
                            type="button"
                            aria-label={`Remove ${chip}`}
                            onClick={(e) => {
                                e.stopPropagation()
                                removeAt(idx)
                            }}
                            className="ml-0.5 text-cyan-300/70 hover:text-cyan-100 transition-colors"
                        >
                            <X className="w-3 h-3" strokeWidth={2.5} />
                        </button>
                    </span>
                ))}

                <input
                    ref={inputRef}
                    type="text"
                    value={draft}
                    onChange={(e) => {
                        setDraft(e.target.value)
                        if (error) setError(null)
                    }}
                    onKeyDown={handleKeyDown}
                    onBlur={() => {
                        // Commit a half-typed chip on blur so users don't
                        // lose it by clicking away. Same UX as Linear.
                        if (draft.trim()) {
                            if (commit(draft)) setDraft('')
                        }
                    }}
                    placeholder={value.length === 0 ? placeholder : undefined}
                    className="flex-1 min-w-[120px] bg-transparent outline-none text-white text-sm placeholder-slate-500"
                />
            </div>

            {error && (
                <p className="text-red-400 text-xs mt-1.5">{error}</p>
            )}
            {!error && helperText && (
                <p className="text-slate-500 text-xs mt-1.5">{helperText}</p>
            )}
        </div>
    )
}
