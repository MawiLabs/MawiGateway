import AppLayout from '@/components/AppLayout'
import './globals.css'
import type { Metadata } from 'next'
import { Toaster } from 'sonner'
import { AuthProvider } from '@/contexts/AuthContext'

export const metadata: Metadata = {
    title: 'MaWi - Agentic Gateway',
    description: 'Premium AI Agentic Gateway & Router',
}

export default function RootLayout({
    children,
}: Readonly<{
    children: React.ReactNode
}>) {
    return (
        <html lang="en">
            <body className="bg-black">
                <AuthProvider>
                    <AppLayout>
                        {children}
                    </AppLayout>
                </AuthProvider>
                <Toaster
                    position="top-right"
                    toastOptions={{
                        style: {
                            background: 'linear-gradient(135deg, rgba(15, 23, 42, 0.95) 0%, rgba(30, 41, 59, 0.95) 100%)',
                            backdropFilter: 'blur(20px)',
                            border: '1px solid rgba(255, 255, 255, 0.1)',
                            color: 'white',
                        },
                    }}
                />
            </body>
        </html>
    )
}
