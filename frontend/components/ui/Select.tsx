'use client'

import { SelectHTMLAttributes, ReactNode } from 'react'
import { ChevronDown } from 'lucide-react'

interface SelectOption {
    value: string
    label: string
}

interface SelectProps extends Omit<SelectHTMLAttributes<HTMLSelectElement>, 'children'> {
    label?: string
    error?: string
    icon?: ReactNode
    helperText?: string
    options: SelectOption[]
    /**
     * Optional placeholder shown as the first, disabled option. When set,
     * the select is uncontrolled-by-default to "Select…" until the user
     * picks something.
     */
    placeholder?: string
}

/**
 * Form-grade Select that matches `<Input>` chrome exactly so forms read
 * as one product: black bg, rounded-xl, cyan-400 focus ring, optional
 * left icon that lights up on focus, optional helper / error footer.
 *
 * Why a custom component instead of bare `<select>`: the native control
 * has OS-default chrome (system fonts, system arrow, light backgrounds
 * on focus). On a dark glassmorphic theme that breaks the design system.
 */
export function Select({
    label,
    error,
    icon,
    helperText,
    options,
    placeholder,
    className = '',
    ...props
}: SelectProps) {
    return (
        <div className="group w-full">
            {label && (
                <label className="block text-sm font-medium text-slate-300 mb-2">
                    {label}
                </label>
            )}

            <div className="relative">
                {icon && (
                    <span className="absolute left-4 top-1/2 -translate-y-1/2 text-slate-400 group-focus-within:text-cyan-400 transition-colors pointer-events-none z-10">
                        {icon}
                    </span>
                )}

                <select
                    className={`
                        w-full px-4 py-3 pr-10 bg-black border rounded-xl
                        text-white outline-none
                        appearance-none cursor-pointer
                        transition-all duration-200
                        ${icon ? 'pl-12' : ''}
                        ${error
                            ? 'border-red-500 focus:border-red-400 focus:ring-4 focus:ring-red-500/20'
                            : 'border-white/10 focus:border-cyan-400 focus:ring-4 focus:ring-cyan-400/20'
                        }
                        ${className}
                    `}
                    {...props}
                >
                    {placeholder && (
                        <option value="" disabled>
                            {placeholder}
                        </option>
                    )}
                    {options.map((opt) => (
                        <option key={opt.value} value={opt.value} className="bg-[#0f0f0f] text-white">
                            {opt.label}
                        </option>
                    ))}
                </select>

                {/* Custom chevron — native arrow is OS-styled (light gray on macOS),
                    breaks the dark theme. pointer-events:none lets clicks fall
                    through to the underlying select. */}
                <ChevronDown
                    className="absolute right-4 top-1/2 -translate-y-1/2 w-4 h-4 text-slate-500 group-focus-within:text-cyan-400 transition-colors pointer-events-none"
                    strokeWidth={2}
                />
            </div>

            {error && (
                <p className="text-red-400 text-sm mt-2">
                    {error}
                </p>
            )}

            {helperText && !error && (
                <p className="text-slate-500 text-xs mt-2">
                    {helperText}
                </p>
            )}
        </div>
    )
}
