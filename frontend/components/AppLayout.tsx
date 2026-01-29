'use client'

import { usePathname } from 'next/navigation'
import Sidebar from './Sidebar'

export default function AppLayout({ children }: { children: React.ReactNode }) {
    const pathname = usePathname()
    const isAuthPage = pathname?.startsWith('/auth')

    // Don't show sidebar on auth pages
    if (isAuthPage) {
        return <>{children}</>
    }

    return (
        <div className="flex h-screen overflow-hidden bg-black">
            <Sidebar />
            <main className="flex-1 ml-64 overflow-auto bg-black">
                {children}
            </main>
        </div>
    )
}
