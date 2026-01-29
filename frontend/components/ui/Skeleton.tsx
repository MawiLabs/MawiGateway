'use client'

export function Skeleton({ className = '', variant = 'default' }: {
    className?: string
    variant?: 'default' | 'text' | 'circle' | 'card'
}) {
    const variants = {
        default: 'h-4 rounded-lg',
        text: 'h-3 rounded',
        circle: 'rounded-full',
        card: 'h-32 rounded-2xl',
    }

    return (
        <div className={`bg-[#0f0f0f] shimmer ${variants[variant]} ${className}`} />
    )
}

export function SkeletonCard() {
    return (
        <div className="glass rounded-2xl p-6 animate-pulse">
            <div className="flex items-center gap-4">
                <Skeleton variant="circle" className="w-12 h-12" />
                <div className="flex-1 space-y-2">
                    <Skeleton className="w-3/4" />
                    <Skeleton variant="text" className="w-1/2" />
                </div>
            </div>
        </div>
    )
}
