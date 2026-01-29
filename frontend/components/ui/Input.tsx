'use client'

import { InputHTMLAttributes, ReactNode } from 'react'

interface InputProps extends InputHTMLAttributes<HTMLInputElement> {
    label?: string
    error?: string
    icon?: ReactNode
    helperText?: string
}

export function Input({
    label,
    error,
    icon,
    helperText,
    className = '',
    ...props
}: InputProps) {
    return (
        <div className="group w-full">
            {label && (
                <label className="block text-sm font-medium text-slate-300 mb-2">
                    {label}
                </label>
            )}

            <div className="relative">
                {icon && (
                    <span className="absolute left-4 top-1/2 -translate-y-1/2 text-slate-400 group-focus-within:text-cyan-400 transition-colors">
                        {icon}
                    </span>
                )}

                <input
                    className={`
            w-full px-4 py-3 bg-black border rounded-xl
            text-white placeholder-slate-500
            transition-all duration-200 outline-none
            ${icon ? 'pl-12' : ''}
            ${error ?
                            'border-red-500 focus:border-red-400 focus:ring-4 focus:ring-red-500/20' :
                            'border-white/10 focus:border-cyan-400 focus:ring-4 focus:ring-cyan-400/20'
                        }
            ${className}
          `}
                    {...props}
                />
            </div>

            {error && (
                <p className="text-red-400 text-sm mt-2 flex items-center gap-1">
                    <span>⚠️</span>
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
