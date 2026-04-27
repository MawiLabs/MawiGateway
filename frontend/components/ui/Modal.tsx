'use client'

import { motion, AnimatePresence } from 'framer-motion'
import { X } from 'lucide-react'
import { ReactNode, useRef } from 'react'
import { useModal } from '@/lib/useModal'

interface ModalProps {
    isOpen: boolean
    onClose: () => void
    title: string
    description?: string
    children: ReactNode
    size?: 'sm' | 'md' | 'lg' | 'xl'
    /**
     * Override the default auto-focus-first-input behaviour. Set to
     * false for confirmation modals where you want focus on the close
     * button (so accidental Enter doesn't fire the destructive action).
     */
    autoFocus?: boolean
    /**
     * Optional footer slot — renders inside the modal below the body
     * with a divider above. Use for primary/secondary action buttons
     * so the dialog has a clear commit point and the body stays
     * focused on inputs.
     */
    footer?: ReactNode
}

/**
 * The modal primitive every form/dialog should route through. Centralises:
 *   - keyboard a11y (ESC to close, focus trap, auto-focus first input)
 *     via `useModal` from `@/lib/useModal`
 *   - the cyan gradient accent bar at the top + soft cyan glow shadow
 *     that establishes brand presence
 *   - backdrop blur + click-outside to close (with stopPropagation on
 *     the modal body so clicks inside don't dismiss)
 *   - role="dialog" + aria-modal="true" so screen readers announce it
 *
 * Ad-hoc `<div className="fixed inset-0 ...">` modals in feature
 * components should be deleted in favour of this. See #41 in the UI
 * polish PR for the rationale.
 */
export function Modal({
    isOpen,
    onClose,
    title,
    description,
    children,
    size = 'md',
    autoFocus = true,
    footer,
}: ModalProps) {
    const ref = useRef<HTMLDivElement>(null)
    useModal({ isOpen, onClose, ref, autoFocus })

    const sizes = {
        sm: 'max-w-md',
        md: 'max-w-lg',
        lg: 'max-w-2xl',
        xl: 'max-w-3xl',
    }

    return (
        <AnimatePresence>
            {isOpen && (
                <motion.div
                    initial={{ opacity: 0 }}
                    animate={{ opacity: 1 }}
                    exit={{ opacity: 0 }}
                    onClick={onClose}
                    className="fixed inset-0 z-50 flex items-start justify-center bg-black/70 backdrop-blur-md px-4 pt-12 pb-12 overflow-y-auto"
                    aria-hidden="true"
                >
                    <motion.div
                        ref={ref}
                        initial={{ scale: 0.96, y: 12, opacity: 0 }}
                        animate={{ scale: 1, y: 0, opacity: 1 }}
                        exit={{ scale: 0.96, y: 12, opacity: 0 }}
                        transition={{ type: 'spring', stiffness: 320, damping: 28 }}
                        onClick={(e) => e.stopPropagation()}
                        role="dialog"
                        aria-modal="true"
                        aria-labelledby="mawi-modal-title"
                        aria-describedby={description ? 'mawi-modal-desc' : undefined}
                        className={`relative w-full ${sizes[size]}
              bg-gradient-to-br from-[#0f0f0f] to-black
              border border-white/10
              rounded-2xl shadow-2xl shadow-cyan-400/20
              overflow-hidden`}
                    >
                        {/* Gradient accent bar — brand presence */}
                        <div className="absolute top-0 left-0 right-0 h-[3px] bg-gradient-to-r from-cyan-400 via-cyan-500 to-cyan-400" />

                        {/* Header */}
                        <div className="px-7 pt-6 pb-5 border-b border-white/10">
                            <div className="flex items-start justify-between gap-4">
                                <div>
                                    <h2
                                        id="mawi-modal-title"
                                        className="text-2xl font-bold text-white"
                                    >
                                        {title}
                                    </h2>
                                    {description && (
                                        <p
                                            id="mawi-modal-desc"
                                            className="text-sm text-slate-400 mt-1"
                                        >
                                            {description}
                                        </p>
                                    )}
                                </div>

                                <button
                                    onClick={onClose}
                                    aria-label="Close dialog"
                                    className="text-slate-400 hover:text-white transition-colors p-1.5 rounded-lg hover:bg-white/10"
                                >
                                    <X className="w-5 h-5" strokeWidth={2} />
                                </button>
                            </div>
                        </div>

                        {/* Body */}
                        <div className="px-7 py-6 max-h-[70vh] overflow-y-auto">
                            {children}
                        </div>

                        {/* Footer (optional) */}
                        {footer && (
                            <div className="px-7 py-4 border-t border-white/10 bg-white/[0.01] flex items-center justify-end gap-2.5">
                                {footer}
                            </div>
                        )}
                    </motion.div>
                </motion.div>
            )}
        </AnimatePresence>
    )
}
