'use client'

export function Spinner({ size = 'md' }: { size?: 'sm' | 'md' | 'lg' }) {
    const sizes = {
        sm: 'w-8 h-8',
        md: 'w-12 h-12',
        lg: 'w-16 h-16',
    }

    return (
        <div className={`relative ${sizes[size]}`}>
            {/* Outer ring */}
            <div className="absolute inset-0 rounded-full border-4 border-[#1a1a1a]" />

            {/* Spinning gradient ring - Cyan-400 */}
            <div className="absolute inset-0 rounded-full border-4 border-transparent border-t-cyan-400 border-r-cyan-500 animate-spin" />

            {/* Inner circle */}
            <div className="absolute inset-2 rounded-full glass" />
        </div>
    )
}
