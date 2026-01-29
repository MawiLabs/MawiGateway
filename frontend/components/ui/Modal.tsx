'use client'

import { motion, AnimatePresence } from 'framer-motion'
import { ReactNode } from 'react'

interface ModalProps {
    isOpen: boolean
    onClose: () => void
    title: string
    description?: string
    children: ReactNode
    size?: 'sm' | 'md' | 'lg'
}

export function Modal({
    isOpen,
    onClose,
    title,
    description,
    children,
    size = 'md'
}: ModalProps) {
    const sizes = {
        sm: 'max-w-md',
        md: 'max-w-lg',
        lg: 'max-w-2xl',
    }

    return (
        <AnimatePresence>
            {isOpen && (
                <motion.div
                    initial={{ opacity: 0 }}
                    animate={{ opacity: 1 }}
                    exit={{ opacity: 0 }}
                    onClick={onClose}
                    className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm px-4">

                    <motion.div
                        initial={{ scale: 0.9, y: 20 }}
                        animate={{ scale: 1, y: 0 }}
                        exit={{ scale: 0.9, y: 20 }}
                        onClick={(e) => e.stopPropagation()}
                        className={`relative w-full ${sizes[size]}
              bg-gradient-to-br from-[#0f0f0f] to-black
              backdrop-filter backdrop-blur-2xl
              border border-white/10
              rounded-2xl shadow-2xl shadow-cyan-400/20
              overflow-hidden`}>

                        {/* Gradient Accent Bar - Cyan-400 */}
                        <div className="absolute top-0 left-0 right-0 h-1 bg-gradient-to-r from-cyan-400 via-cyan-500 to-cyan-600" />

                        {/* Header */}
                        <div className="p-6 border-b border-white/10">
                            <div className="flex items-start justify-between">
                                <div>
                                    <h2 className="text-2xl font-bold text-white">
                                        {title}
                                    </h2>
                                    {description && (
                                        <p className="text-sm text-slate-400 mt-1">
                                            {description}
                                        </p>
                                    )}
                                </div>

                                {/* Close Button */}
                                <button
                                    onClick={onClose}
                                    className="text-slate-400 hover:text-white transition-colors p-1 rounded-lg hover:bg-white/10">
                                    <svg className="w-6 h-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
                                    </svg>
                                </button>
                            </div>
                        </div>

                        {/* Body */}
                        <div className="p-6 max-h-[70vh] overflow-y-auto">
                            {children}
                        </div>
                    </motion.div>
                </motion.div>
            )}
        </AnimatePresence>
    )
}
