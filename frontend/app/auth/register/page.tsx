'use client'

import { useState } from 'react'
import { useRouter } from 'next/navigation'
import { motion } from 'framer-motion'
import { Button, Input, Card } from '@/components/ui'
import { useAuth } from '@/contexts/AuthContext'
import { toast } from 'sonner'
import Link from 'next/link'

export default function RegisterPage() {
    const router = useRouter()
    const { register } = useAuth()
    const [name, setName] = useState('')
    const [orgName, setOrgName] = useState('')
    const [email, setEmail] = useState('')
    const [password, setPassword] = useState('')
    const [confirmPassword, setConfirmPassword] = useState('')
    const [loading, setLoading] = useState(false)

    const handleSubmit = async (e: React.FormEvent) => {
        e.preventDefault()

        // Validation
        if (password !== confirmPassword) {
            toast.error('Passwords do not match')
            return
        }

        if (password.length < 8) {
            toast.error('Password must be at least 8 characters')
            return
        }

        if (!orgName.trim()) {
            toast.error('Organization Name is required')
            return
        }

        setLoading(true)

        try {
            // 1. Register User
            await register(email, password, name || undefined)

            // 2. Create Organization
            const res = await fetch('/v1/organizations', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    name: orgName,
                    tier: 'community',
                    industry: 'other'
                }),
                credentials: 'include'
            })

            if (!res.ok) {
                // If org creation fails, we still registered. 
                // Redirect to onboarding page to retry org creation.
                toast.error('Account created, but failed to create organization. Please try again.')
                router.push('/onboarding/organization')
                return
            }

            toast.success('Account & Organization created! Welcome to MaWi 🎉')

            // 3. User is now fully set up (org_id linked in backend). 
            // We might need to refresh user context? Register already sets user.
            // But Register sets user BEFORE org creation.
            // So 'user.organization_id' in context will be undefined.
            // We should reload the page or force refresh.
            // Since we push to '/', checkSession might handle it if we are lucky, 
            // but context state is stale.
            // A hard navigation or explicit context refresh is safest.
            window.location.href = '/'

        } catch (error: any) {
            toast.error(error.message || 'Registration failed')
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
                    <h1 className="text-3xl font-bold gradient-text-white mb-2">Join MaWi</h1>
                    <p className="text-slate-400">Start with free Tier A ($5/month)</p>
                </div>

                {/* Register Card */}
                <Card className="p-8">
                    <form onSubmit={handleSubmit} className="space-y-6">
                        <Input
                            label="Name (optional)"
                            type="text"
                            value={name}
                            onChange={(e) => setName(e.target.value)}
                            placeholder="John Doe"
                            icon={<span>👤</span>}
                        />

                        <Input
                            label="Organization Name"
                            type="text"
                            value={orgName}
                            onChange={(e) => setOrgName(e.target.value)}
                            placeholder="My Company"
                            icon={<span>🏢</span>}
                            required
                        />

                        <Input
                            label="Email"
                            type="email"
                            value={email}
                            onChange={(e) => setEmail(e.target.value)}
                            placeholder="you@example.com"
                            icon={<span>📧</span>}
                            required
                        />

                        <Input
                            label="Password"
                            type="password"
                            value={password}
                            onChange={(e) => setPassword(e.target.value)}
                            placeholder="••••••••"
                            icon={<span>🔒</span>}
                            helperText="At least 8 characters"
                            required
                        />

                        <Input
                            label="Confirm Password"
                            type="password"
                            value={confirmPassword}
                            onChange={(e) => setConfirmPassword(e.target.value)}
                            placeholder="••••••••"
                            icon={<span>🔒</span>}
                            required
                        />

                        <Button
                            type="submit"
                            variant="primary"
                            className="w-full"
                            loading={loading}>
                            Create Account
                        </Button>
                    </form>

                    {/* Login Link */}
                    <div className="mt-6 text-center">
                        <span className="text-slate-400 text-sm">Already have an account? </span>
                        <Link href="/auth/login" className="text-cyan-400 text-sm hover:underline">
                            Sign In
                        </Link>
                    </div>
                </Card>

                {/* Footer */}
                <p className="text-center text-sm text-slate-500 mt-6">
                    By creating an account, you agree to our Terms of Service and Privacy Policy
                </p>
            </motion.div>
        </div>
    )
}
