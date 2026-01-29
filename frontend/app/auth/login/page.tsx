'use client'

import { useState } from 'react'
import { useRouter } from 'next/navigation'
import { motion } from 'framer-motion'
import { Button, Input, Card } from '@/components/ui'
import { useAuth } from '@/contexts/AuthContext'
import { toast } from 'sonner'
import Link from 'next/link'

export default function LoginPage() {
    const router = useRouter()
    const { login } = useAuth()
    const [email, setEmail] = useState('')
    const [password, setPassword] = useState('')
    const [loading, setLoading] = useState(false)

    const handleSubmit = async (e: React.FormEvent) => {
        e.preventDefault()
        setLoading(true)

        try {
            await login(email, password)
            toast.success('Welcome back!')
            router.push('/')
        } catch (error: any) {
            toast.error(error.message || 'Login failed')
        } finally {
            setLoading(false)
        }
    }

    return (
        <div className="min-h-screen bg-black flex items-center justify-center p-8">
            <motion.div
                initial={{ opacity: 0, y: 20 }}
                animate={{ opacity: 1, y: 0 }}
                className="w-full max-w-md">

                {/* Logo */}
                <div className="text-center mb-8">
                    <div className="text-5xl mb-4">🌊</div>
                    <h1 className="text-3xl font-bold gradient-text-white mb-2">MaWi Gateway</h1>
                    <p className="text-slate-400">Sign in to your account</p>
                </div>

                {/* Login Card */}
                <Card className="p-8">
                    <form onSubmit={handleSubmit} className="space-y-6">
                        <Input
                            label="Email"
                            type="email"
                            value={email}
                            onChange={(e) => setEmail(e.target.value)}
                            placeholder="you@example.com"
                            icon={<span>📧</span>}
                            required
                            autoFocus
                        />

                        <Input
                            label="Password"
                            type="password"
                            value={password}
                            onChange={(e) => setPassword(e.target.value)}
                            placeholder="••••••••"
                            icon={<span>🔒</span>}
                            required
                        />

                        <Button
                            type="submit"
                            variant="primary"
                            className="w-full"
                            loading={loading}>
                            Sign In
                        </Button>
                    </form>

                    {/* Divider */}
                    <div className="relative my-6">
                        <div className="absolute inset-0 flex items-center">
                            <div className="w-full border-t border-white/10" />
                        </div>
                        <div className="relative flex justify-center text-sm">
                            <span className="px-2 bg-[#0f0f0f] text-slate-400">New to MaWi?</span>
                        </div>
                    </div>

                    {/* Register Link */}
                    <Link href="/auth/register">
                        <Button variant="secondary" className="w-full">
                            Create Account
                        </Button>
                    </Link>
                </Card>

                {/* Footer */}
                <p className="text-center text-sm text-slate-500 mt-6">
                    By signing in, you agree to our Terms of Service and Privacy Policy
                </p>
            </motion.div>
        </div>
    )
}
