'use client'

import { createContext, useContext, useState, useEffect, ReactNode } from 'react'
import { useRouter, usePathname } from 'next/navigation'

interface User {
    id: string
    email: string
    name?: string
    tier: 'community' | 'team' | 'enterprise'
    monthly_quota_usd: number
    current_usage_usd: number
    quota_remaining_usd?: number
    is_free_tier: boolean
    organization_id?: string
}

interface AuthContextType {
    user: User | null
    loading: boolean
    login: (email: string, password: string) => Promise<void>
    register: (email: string, password: string, name?: string) => Promise<void>
    logout: () => Promise<void>
    refreshUser: () => Promise<void>
}

const AuthContext = createContext<AuthContextType | undefined>(undefined)

// Public routes that don't satisfy auth checks
const PUBLIC_ROUTES = ['/auth/login', '/auth/register']

export function AuthProvider({ children }: { children: ReactNode }) {
    const [user, setUser] = useState<User | null>(null)
    const [loading, setLoading] = useState(true)
    const router = useRouter()
    const pathname = usePathname()

    // ✅ Check session ONCE on mount - cached auth check
    useEffect(() => {
        checkSession()
    }, []) // No dependencies - runs only once

    const checkSession = async () => {
        console.log('[AuthContext] Checking session...')
        try {
            const res = await fetch('/v1/auth/me', { credentials: 'include' })
            console.log('[AuthContext] Session check response:', res.status)
            if (res.ok) {
                const data = await res.json()
                setUser(data.user)
                console.log('[AuthContext] User authenticated:', data.user)

                // Check for organization - redirect to onboarding if missing
                if (data.user && !data.user.organization_id && !pathname?.startsWith('/onboarding')) {
                    router.push('/onboarding/organization')
                }
            } else if (res.status === 401) {
                setUser(null)
                console.log('[AuthContext] User not authenticated (401)')
                // Redirect to login if on protected route
                const isPublicRoute = PUBLIC_ROUTES.some(path => pathname?.startsWith(path))
                if (!isPublicRoute) {
                    router.push('/auth/login')
                }
            } else {
                console.error('Session check returned status:', res.status)
                setUser(null)
            }
        } catch (error) {
            console.error('[AuthContext] Session check failed:', error)
            setUser(null)
        } finally {
            console.log('[AuthContext] Setting loading to false')
            setLoading(false)
        }
    }

    const login = async (email: string, password: string) => {
        const res = await fetch('/v1/auth/login', {
            method: 'POST',
            credentials: 'include',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ email, password }),
        })
        if (!res.ok) {
            const err = await res.text()
            throw new Error(err || 'Login failed')
        }
        const data = await res.json()
        setUser(data.user)

        // Check for organization
        if (!data.user.organization_id) {
            router.push('/onboarding/organization')
            return
        }

        router.push('/')
    }

    const register = async (email: string, password: string, name?: string) => {
        const res = await fetch('/v1/auth/register', {
            method: 'POST',
            credentials: 'include',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ email, password, name }),
        })
        if (!res.ok) {
            const err = await res.text()
            throw new Error(err || 'Registration failed')
        }
        const data = await res.json()
        setUser(data.user)

        // Check for organization
        if (!data.user.organization_id) {
            router.push('/onboarding/organization')
            return
        }

        router.push('/')
    }

    const logout = async () => {
        try {
            await fetch('/v1/auth/logout', { method: 'POST', credentials: 'include' })
        } catch (error) {
            console.error('Logout failed', error)
        }
        setUser(null)
        router.push('/auth/login')
    }

    const refreshUser = async () => {
        await checkSession()
    }

    // ✅ Only show loading spinner on initial app load, not during navigation
    if (loading) {
        return (
            <div className="min-h-screen bg-black flex items-center justify-center">
                <div className="animate-spin rounded-full h-8 w-8 border-t-2 border-b-2 border-cyan-500"></div>
            </div>
        )
    }

    // ✅ No double redirect logic - just provide auth state to children
    // Pages can decide what to do with auth state
    return (
        <AuthContext.Provider value={{ user, loading, login, register, logout, refreshUser }}>
            {children}
        </AuthContext.Provider>
    )
}

// Exported helpers to fetch user resources
export const fetchProviders = async () => {
    const res = await fetch('/v1/user/providers', { credentials: 'include' })
    if (!res.ok) {
        const err = await res.text()
        throw new Error(err || 'Failed to load providers')
    }
    const data = await res.json()
    return { providers: Array.isArray(data) ? data : data.providers }
}

export const fetchModels = async () => {
    const res = await fetch('/v1/user/models', { credentials: 'include' })
    if (!res.ok) {
        const err = await res.text()
        throw new Error(err || 'Failed to load models')
    }
    const data = await res.json()
    return { models: Array.isArray(data) ? data : data.models }
}

export function useAuth() {
    const context = useContext(AuthContext)
    if (!context) {
        throw new Error('useAuth must be used within AuthProvider')
    }
    return context
}
