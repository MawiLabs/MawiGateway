'use client'

import Sidebar from '@/components/Sidebar'

export default function DocsPage() {
    const endpoints = [
        { method: 'POST', path: '/v1/chat/completions', desc: 'Unified chat endpoint' },
        { method: 'GET', path: '/v1/providers', desc: 'List all providers' },
        { method: 'POST', path: '/v1/providers', desc: 'Create provider' },
        { method: 'GET', path: '/v1/models', desc: 'List all models' },
        { method: 'POST', path: '/v1/models', desc: 'Create model' },
        { method: 'GET', path: '/v1/services', desc: 'List all services' },
        { method: 'POST', path: '/v1/services', desc: 'Create service' },
    ]

    return (
        <>
            <Sidebar />

            <main className="flex-1 overflow-y-auto bg-[#0a0a0a]">
                <div className="border-b border-gray-800 px-8 py-5">
                    <div>
                        <h1 className="text-xl font-semibold text-white">API Documentation</h1>
                        <p className="text-sm text-gray-500 mt-1">Complete API reference</p>
                    </div>
                </div>

                <div className="p-8">
                    {/* Swagger Link */}
                    <a
                        href="http://localhost:8030/swagger-ui"
                        target="_blank"
                        className="card p-6 block hover:border-gray-700 transition-colors mb-8"
                    >
                        <div className="flex items-center justify-between">
                            <div>
                                <h3 className="text-white font-medium mb-1">Interactive API Documentation</h3>
                                <p className="text-sm text-gray-500">Open Swagger UI for testing endpoints</p>
                            </div>
                            <span className="text-sky-400">→</span>
                        </div>
                    </a>

                    {/* Endpoints List */}
                    <div className="card p-6">
                        <h2 className="text-white font-medium mb-4">API Endpoints</h2>
                        <div className="space-y-3">
                            {endpoints.map((endpoint, i) => (
                                <div key={i} className="flex items-center gap-4 py-3 border-b border-gray-800 last:border-0">
                                    <span className={`px-2 py-1 rounded text-xs font-medium ${endpoint.method === 'GET' ? 'bg-sky-500/10 text-sky-400' : 'bg-green-500/10 text-green-400'
                                        }`}>
                                        {endpoint.method}
                                    </span>
                                    <code className="text-sm text-gray-300 font-mono">{endpoint.path}</code>
                                    <span className="text-sm text-gray-500 ml-auto">{endpoint.desc}</span>
                                </div>
                            ))}
                        </div>
                    </div>

                    {/* Example */}
                    <div className="card p-6 mt-6">
                        <h2 className="text-white font-medium mb-4">Example Request</h2>
                        <pre className="bg-[#0a0a0a] rounded p-4 text-sm text-gray-300 overflow-x-auto">
                            {`curl -X POST http://localhost:8030/v1/chat/completions \\
  -H "Content-Type: application/json" \\
  -d '{
    "service": "customer-chat",
    "messages": [
      {"role": "user", "content": "Hello!"}
    ]
  }'`}
                        </pre>
                    </div>
                </div>
            </main>
        </>
    )
}
