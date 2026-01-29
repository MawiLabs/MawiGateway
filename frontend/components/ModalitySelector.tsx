'use client'

import { motion } from 'framer-motion'

interface ModalitySelectorProps {
    label: string
    selectedModalities: string[]
    onChange: (modalities: string[]) => void
    color?: 'cyan' | 'purple'
}

const modalities = [
    { id: 'text', icon: '📝', label: 'Text' },
    { id: 'image', icon: '🖼️', label: 'Image' },
    { id: 'audio', icon: '🎵', label: 'Audio' },
    { id: 'video', icon: '🎬', label: 'Video' }
]

export function ModalitySelector({
    label,
    selectedModalities,
    onChange,
    color = 'cyan'
}: ModalitySelectorProps) {
    const colorClass = color === 'cyan' ? 'text-cyan-400' : 'text-purple-400'
    const ringClass = color === 'cyan' ? 'focus:ring-cyan-400/50' : 'focus:ring-purple-400/50'

    const toggleModality = (id: string, checked: boolean) => {
        if (checked) {
            onChange([...selectedModalities, id])
        } else {
            onChange(selectedModalities.filter(m => m !== id))
        }
    }

    return (
        <div>
            <label className={`block text-xs font-semibold ${colorClass} mb-2 uppercase tracking-wider`}>
                {label}
            </label>
            <div className="grid grid-cols-2 gap-2">
                {modalities.map(mod => (
                    <motion.label
                        key={mod.id}
                        whileHover={{ scale: 1.02 }}
                        whileTap={{ scale: 0.98 }}
                        className={`flex items-center gap-3 p-3 rounded-lg border-2 cursor-pointer transition-all ${selectedModalities.includes(mod.id)
                                ? `border-${color}-400 bg-${color}-400/10`
                                : 'border-white/10 bg-white/5 hover:bg-white/10'
                            }`}>
                        <input
                            type="checkbox"
                            checked={selectedModalities.includes(mod.id)}
                            onChange={(e) => toggleModality(mod.id, e.target.checked)}
                            className={`rounded border-white/20 bg-black text-${color}-400 ${ringClass}`}
                        />
                        <span className="text-xl">{mod.icon}</span>
                        <span className="text-white font-medium">{mod.label}</span>
                    </motion.label>
                ))}
            </div>
        </div>
    )
}
