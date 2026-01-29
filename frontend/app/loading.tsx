export default function Loading() {
    return (
        <div className="flex h-full w-full items-center justify-center p-8">
            <div className="flex flex-col items-center gap-4">
                {/* Animated Glow Container */}
                <div className="relative flex items-center justify-center">
                    {/* Outer glow ring */}
                    <div className="absolute h-24 w-24 animate-pulse rounded-full bg-cyan-400/20 blur-xl" />

                    {/* Spinning gradient ring */}
                    <div className="h-16 w-16 animate-spin rounded-full border-2 border-white/10 border-t-cyan-400" />

                    {/* Inner Logo/Icon */}
                    <div className="absolute inset-0 flex items-center justify-center text-xl">
                        ⚡
                    </div>
                </div>

                {/* Loading Text */}
                <p className="animate-pulse text-sm font-medium text-slate-400">
                    Loading...
                </p>
            </div>
        </div>
    )
}
