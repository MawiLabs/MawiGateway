'use client'

import { useState } from 'react'
import { Button, Card, Input, Modal, Badge, Spinner, Skeleton, SkeletonCard } from '@/components/ui'
import { toast } from 'sonner'

export default function UIShowcasePage() {
    const [showModal, setShowModal] = useState(false)
    const [loading, setLoading] = useState(false)

    return (
        <div className="min-h-screen p-8 bg-black">
            <div className="max-w-7xl mx-auto space-y-8">
                {/* Header */}
                <div className="text-center mb-12">
                    <h1 className="text-5xl font-bold gradient-text-white mb-4">
                        Premium UI Components
                    </h1>
                    <p className="text-slate-400 text-lg">
                        Glassmorphism • Framer Motion • Gradient Accents
                    </p>
                </div>

                {/* Buttons */}
                <Card>
                    <h2 className="text-2xl font-bold text-white mb-4">Buttons</h2>
                    <div className="flex flex-wrap gap-4">
                        <Button variant="primary" onClick={() => toast.success('Success!', { description: 'Primary button clicked' })}>
                            Primary Button
                        </Button>
                        <Button variant="secondary">
                            Secondary
                        </Button>
                        <Button variant="danger">
                            Danger
                        </Button>
                        <Button variant="ghost">
                            Ghost
                        </Button>
                        <Button variant="primary" size="sm">
                            Small
                        </Button>
                        <Button variant="primary" size="lg">
                            Large
                        </Button>
                        <Button variant="primary" loading={loading} onClick={() => {
                            setLoading(true)
                            setTimeout(() => setLoading(false), 2000)
                        }}>
                            With Loading
                        </Button>
                        <Button variant="primary" icon={<span>🚀</span>}>
                            With Icon
                        </Button>
                    </div>
                </Card>

                {/* Cards */}
                <Card>
                    <h2 className="text-2xl font-bold text-white mb-4">Cards with Glow</h2>
                    <div className="grid grid-cols-3 gap-4">
                        <Card glow="cyan" className="text-center">
                            <div className="text-4xl mb-2">💎</div>
                            <div className="text-white font-semibold">Cyan Glow</div>
                        </Card>
                        <Card glow="purple" className="text-center">
                            <div className="text-4xl mb-2">✨</div>
                            <div className="text-white font-semibold">Purple Glow</div>
                        </Card>
                        <Card glow="green" className="text-center">
                            <div className="text-4xl mb-2">🌟</div>
                            <div className="text-white font-semibold">Green Glow</div>
                        </Card>
                    </div>
                </Card>

                {/* Inputs */}
                <Card>
                    <h2 className="text-2xl font-bold text-white mb-4">Inputs</h2>
                    <div className="space-y-4 max-w-md">
                        <Input
                            label="Email Address"
                            type="email"
                            placeholder="you@example.com"
                            icon={<span>📧</span>}
                        />
                        <Input
                            label="Password"
                            type="password"
                            placeholder="••••••••"
                            icon={<span>🔒</span>}
                            helperText="Must be at least 8 characters"
                        />
                        <Input
                            label="Error State"
                            error="This field is required"
                            icon={<span>⚠️</span>}
                        />
                    </div>
                </Card>

                {/* Badges */}
                <Card>
                    <h2 className="text-2xl font-bold text-white mb-4">Badges</h2>
                    <div className="flex flex-wrap gap-3">
                        <Badge variant="primary" glow>Primary</Badge>
                        <Badge variant="success">Success</Badge>
                        <Badge variant="warning">Warning</Badge>
                        <Badge variant="danger">Danger</Badge>
                        <Badge variant="purple">Purple</Badge>
                        <Badge size="sm">Small</Badge>
                    </div>
                </Card>

                {/* Loading States */}
                <Card>
                    <h2 className="text-2xl font-bold text-white mb-4">Loading States</h2>
                    <div className="flex items-center gap-8">
                        <Spinner size="sm" />
                        <Spinner size="md" />
                        <Spinner size="lg" />
                    </div>
                </Card>

                {/* Skeletons */}
                <Card>
                    <h2 className="text-2xl font-bold text-white mb-4">Skeleton Loaders</h2>
                    <div className="space-y-4">
                        <SkeletonCard />
                        <div className="space-y-2">
                            <Skeleton className="w-full" />
                            <Skeleton className="w-3/4" />
                            <Skeleton className="w-1/2" />
                        </div>
                    </div>
                </Card>

                {/* Modal */}
                <Card>
                    <h2 className="text-2xl font-bold text-white mb-4">Modal</h2>
                    <Button onClick={() => setShowModal(true)}>
                        Open Modal
                    </Button>
                </Card>

                {/* Toast Examples */}
                <Card>
                    <h2 className="text-2xl font-bold text-white mb-4">Toast Notifications</h2>
                    <div className="flex flex-wrap gap-3">
                        <Button variant="secondary" onClick={() =>
                            toast.success('Success!', { description: 'Operation completed successfully' })
                        }>
                            Success Toast
                        </Button>
                        <Button variant="secondary" onClick={() =>
                            toast.error('Error!', { description: 'Something went wrong' })
                        }>
                            Error Toast
                        </Button>
                        <Button variant="secondary" onClick={() =>
                            toast.info('Info', { description: 'Here is some information' })
                        }>
                            Info Toast
                        </Button>
                        <Button variant="secondary" onClick={() =>
                            toast.loading('Loading...', { description: 'Please wait' })
                        }>
                            Loading Toast
                        </Button>
                    </div>
                </Card>
            </div>

            {/* Demo Modal */}
            <Modal
                isOpen={showModal}
                onClose={() => setShowModal(false)}
                title="Premium Modal"
                description="This is a beautiful glassmorphic modal with animations">
                <div className="space-y-4">
                    <Input
                        label="Name"
                        placeholder="Enter your name"
                        icon={<span>👤</span>}
                    />
                    <Input
                        label="Email"
                        type="email"
                        placeholder="you@example.com"
                        icon={<span>📧</span>}
                    />
                    <div className="flex gap-3 pt-4">
                        <Button variant="ghost" onClick={() => setShowModal(false)} className="flex-1">
                            Cancel
                        </Button>
                        <Button variant="primary" onClick={() => {
                            toast.success('Submitted!', { description: 'Your form has been submitted' })
                            setShowModal(false)
                        }} className="flex-1">
                            Submit
                        </Button>
                    </div>
                </div>
            </Modal>
        </div>
    )
}
