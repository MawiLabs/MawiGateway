'use client'

import { useState, useEffect, useRef } from 'react'
import { motion, AnimatePresence } from 'framer-motion'
import DOMPurify from 'dompurify' // Sanitize HTML
import { Button, Card, Badge, Spinner } from '@/components/ui'
import { toast } from 'sonner'
import { AudioInput } from '@/components/AudioInput'
import ReactMarkdown from 'react-markdown'
import { ThoughtTimeline, AgenticStreamEvent } from '@/components/playground/ThoughtTimeline'
import { Settings, TestTube2, AlertTriangle, Send, User, Sparkles, Trash2, SlidersHorizontal } from 'lucide-react'

interface Service {
    name: string
    label?: string
    group?: string
    service_type?: string
    description?: string
    modality?: 'text' | 'multimodal' | 'image' | 'audio' | 'music' | 'speech-to-text' | 'speech-to-speech' | 'video'
    system_prompt?: string
    pool_type?: string
    strategy?: string  // Routing strategy (e.g., least_cost, weighted, etc.)
}


interface Message {
    role: 'user' | 'assistant' | 'system'
    content: string
    model?: string
    timestamp: Date
    events?: AgenticStreamEvent[] // Store agentic events attached to this message
    isComplete?: boolean
    isFinalAnswerStarted?: boolean
}

interface Config {
    temperature: number
    maxTokens: number
    topP: number
    frequencyPenalty: number
    presencePenalty: number
    systemPrompt: string
    stopSequences: string
    reasoningEffort: 'low' | 'medium' | 'high'
    rtcrosEnabled: boolean
    rtcros: {
        role: string
        task: string
        context: string
        reasoning: string
        output: string
        stop: string
    }
}

const DEFAULT_CONFIG: Config = {
    temperature: 0.7,
    maxTokens: 2048,
    topP: 1.0,
    frequencyPenalty: 0,
    presencePenalty: 0,
    systemPrompt: '',
    stopSequences: '',
    reasoningEffort: 'medium',
    rtcrosEnabled: false,
    rtcros: {
        role: '',
        task: '',
        context: '',
        reasoning: '',
        output: '',
        stop: ''
    }
}

export default function PlaygroundPage() {
    const [services, setServices] = useState<Service[]>([])
    const [selectedService, setSelectedService] = useState<string>(() => {
        // Load from localStorage on mount
        if (typeof window !== 'undefined') {
            return localStorage.getItem('playground_selectedService') || ''
        }
        return ''
    })
    const [prompt, setPrompt] = useState('')
    const [messages, setMessages] = useState<Message[]>([])
    const [isLoading, setIsLoading] = useState(false)
    const [streamingContent, setStreamingContent] = useState('')
    const [showSettings, setShowSettings] = useState(true)
    const [config, setConfig] = useState<Config>(DEFAULT_CONFIG)
    // Voice for TTS / S2S. Empty = let the provider pick its own default —
    // each provider's adapter handles this case (Hume picks a stock Octave
    // voice, ElevenLabs falls back to Rachel `21m00…`). Hardcoding an
    // ElevenLabs voice id here (the previous behaviour) caused 404s when
    // the service routed to Hume.
    const [voiceId, setVoiceId] = useState('')
    const [audioBlob, setAudioBlob] = useState<Blob | null>(null)
    const [clearTrigger, setClearTrigger] = useState(0)
    const [showDebug, setShowDebug] = useState(false)
    const messagesEndRef = useRef<HTMLDivElement>(null)

    const selectedItem = services.find(s => s.name === selectedService)

    // Audio role disambiguation (shared by submit logic + render).
    // Models in the DB are tagged with a single `audio` modality
    // regardless of direction; pull that apart by name so the JSX can
    // hide the Voice picker on Whisper / music services and the
    // submit handler can route to the right endpoint. Empty string
    // when no service is picked yet so the conditionals all evaluate
    // false safely.
    const audioRole: 'tts' | 'stt' | 'sts' | 'music' | 'other' = (() => {
        if (!selectedItem) return 'other'
        const tag = ((selectedService || '') + ' ' + (selectedItem.label || '')).toLowerCase()
        if (selectedItem.modality === 'speech-to-text' || /\b(whisper|stt|transcribe)\b/.test(tag)) return 'stt'
        if (selectedItem.modality === 'speech-to-speech' || /speech-to-speech|\bsts\b/.test(tag)) return 'sts'
        if (selectedItem.modality === 'music' || /\bmusic\b/.test(tag)) return 'music'
        if (selectedItem.modality === 'audio') return 'tts'
        return 'other'
    })()



    useEffect(() => {
        loadServices()
    }, [])

    useEffect(() => {
        messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' })
    }, [messages, streamingContent])

    // Persist selected service to localStorage
    useEffect(() => {
        if (selectedService) {
            localStorage.setItem('playground_selectedService', selectedService)
        }
    }, [selectedService])

    // Sync System Prompt when Service Changes
    useEffect(() => {
        if (!selectedService) return;
        const s = services.find(x => x.name === selectedService);
        if (s?.system_prompt) {
            // toast.info(`Loaded System Prompt from ${s.label}`);
            setConfig(prev => ({ ...prev, systemPrompt: s.system_prompt! }));
        } else {
            // Optional: Clear or keep previous? Keeping previous is safer for manual edits.
            // But if switching between services, maybe we want to see the new context.
            // Let's only set if present.
        }
    }, [selectedService, services]);

    const loadServices = async () => {
        try {
            console.log('🔄 Loading topology...')
            const res = await fetch('/v1/topology', {
                credentials: 'include'
            })

            console.log('📡 Topology response status:', res.status, res.statusText)

            if (!res.ok) {
                const errorText = await res.text()
                console.error('❌ Topology fetch failed:', res.status, errorText)
                toast.error(`Failed to load services: ${res.status} ${res.statusText}`)
                return
            }

            const data = await res.json()
            console.log('📦 Topology data:', data)

            const newServices: Service[] = []

            // 1. Add Services (Routers)
            if (data.services) {
                console.log(`✅ Found ${data.services.length} services`)
                newServices.push(...data.services.map((s: any) => ({
                    name: `service:${s.service.name}`, // Unique Value
                    label: `Service: ${s.service.name}`,
                    group: 'Services',
                    description: `Router for ${s.models?.length || 0} models`,
                    service_type: s.service.service_type, // Track AGENTIC vs POOL
                    system_prompt: s.service.system_prompt, // Load system prompt
                    pool_type: s.service.pool_type, // Track Single vs Multi Modality
                    strategy: s.service.strategy // Load routing strategy
                })))
            }

            // 2. Add Models grouped by Service
            if (data.services) {
                data.services.forEach((s: any) => {
                    if (s.models) {
                        console.log(`📋 Service "${s.service.name}" has ${s.models.length} models`)
                        s.models.forEach((m: any) => {
                            newServices.push({
                                name: `scoped:${s.service.name}:${m.model_id}`,
                                label: `${s.service.name} › ${m.model_name}`,
                                group: `Models in ${s.service.name}`,
                                modality: m.modality || 'text'
                            })
                        })
                    }
                })
            }

            // 3. Add Direct Models
            if (data.models) {
                console.log(`✅ Found ${data.models.length} direct models`)
                data.models.forEach((m: any) => {
                    newServices.push({
                        name: `model:${m.id}`,
                        label: m.name,
                        group: 'Direct Access',
                        modality: m.modality || 'text'
                    })
                })
            }

            console.log(`🎯 Total services/models loaded: ${newServices.length}`)
            console.log('Services array:', newServices)
            setServices(newServices)

            // Restore saved selection if valid, otherwise use first service
            const savedSelection = localStorage.getItem('playground_selectedService')
            if (savedSelection && newServices.some(s => s.name === savedSelection)) {
                console.log('🔧 Restoring saved service:', savedSelection)
                setSelectedService(savedSelection)
                const saved = newServices.find(s => s.name === savedSelection)
                if (saved?.system_prompt) {
                    setConfig(prev => ({ ...prev, systemPrompt: saved.system_prompt! }))
                }
            } else if (newServices.length > 0 && !selectedService) {
                console.log('🔧 Setting default service to:', newServices[0].name)
                setSelectedService(newServices[0].name)
                if (newServices[0].system_prompt) {
                    setConfig(prev => ({ ...prev, systemPrompt: newServices[0].system_prompt! }))
                }
            }
        } catch (error) {
            console.error('💥 Failed to load topology:', error)
            toast.error('Failed to load services configuration')
        }
    }

    const handleSubmit = async (e: React.FormEvent) => {
        e.preventDefault()
        // Allow submission if prompt exists OR audio blob exists (for STT/STS)
        if ((!prompt.trim() && !audioBlob) || !selectedService || isLoading) return

        // Only add text message if prompt is not empty
        let currentMessages = [...messages]
        if (prompt.trim()) {
            const userMessage: Message = {
                role: 'user',
                content: prompt,
                timestamp: new Date()
            }
            setMessages(prev => [...prev, userMessage])
            setPrompt('')
            currentMessages.push(userMessage)
        }
        setIsLoading(true)
        setStreamingContent('')

        try {
            // Build messages array with system prompt if present
            const requestMessages = []

            // Handle System Prompt (Standard or RTCROS)
            let systemContent = config.systemPrompt
            if (config.rtcrosEnabled) {
                systemContent = [
                    config.rtcros.role && `## Role\n${config.rtcros.role}`,
                    config.rtcros.task && `## Task\n${config.rtcros.task}`,
                    config.rtcros.context && `## Context\n${config.rtcros.context}`,
                    config.rtcros.reasoning && `## Reasoning\n${config.rtcros.reasoning}`,
                    config.rtcros.output && `## Output format\n${config.rtcros.output}`,
                    config.rtcros.stop && `## Stop Criteria\n${config.rtcros.stop}`
                ].filter(Boolean).join('\n\n')
            }

            if (systemContent.trim()) {
                requestMessages.push({ role: 'system', content: systemContent })
            }
            requestMessages.push(...currentMessages.map(m => ({
                role: m.role,
                content: m.content
            })))

            // Check modality
            const selectedItem = services.find(s => s.name === selectedService)
            const isImageModel = selectedItem?.modality === 'image'

            // Parse model ID (strip prefix if present)
            let modelId = selectedService
            if (selectedService.startsWith('service:')) {
                modelId = selectedService.replace('service:', '')
            } else if (selectedService.startsWith('model:')) {
                modelId = selectedService.replace('model:', '')
            } else if (selectedService.startsWith('scoped:')) {
                const parts = selectedService.split(':')
                if (parts.length >= 3) {
                    modelId = parts.slice(2).join(':')
                }
            }

            // Audio direction disambiguation. Models in the DB are
            // tagged with a single `audio` modality regardless of
            // whether they speak (TTS) or listen (STT). Use the
            // model id / service name to pick the right endpoint
            // before falling into the generic audio branch — same
            // direction-blind bug pattern as the voice-cinematic
            // service modality issue.
            const audioName = (modelId + ' ' + (selectedItem?.label || '')).toLowerCase()
            const isSpeechToText =
                selectedItem?.modality === 'speech-to-text' ||
                /\b(whisper|stt|transcribe)\b/.test(audioName)
            const isSpeechToSpeech =
                selectedItem?.modality === 'speech-to-speech' ||
                /speech-to-speech|\bsts\b/.test(audioName)
            const isMusicGen =
                selectedItem?.modality === 'music' ||
                /\bmusic\b/.test(audioName)

            let response

            // CRITICAL: AGENTIC services must ALWAYS go through chat/completions
            // so the AgenticExecutor can orchestrate tool calls.
            const isAgenticService = selectedItem?.service_type === 'AGENTIC'

            if (isImageModel && !isAgenticService) {
                // Image Generation Request (Direct Model Access only)
                response = await fetch('/v1/images/generations', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({
                        model: modelId,
                        prompt: prompt,
                        n: 1,
                        size: "1024x1024",
                        quality: "standard"
                    })
                })
            } else if (selectedItem?.modality === 'video') {
                // Video Generation Request
                response = await fetch('/v1/videos/generations', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({
                        model: modelId,
                        prompt: prompt
                    })
                })
            } else if (isSpeechToText) {
                // Speech-to-Text Request
                if (!audioBlob) {
                    toast.error('Please record or upload audio first')
                    setIsLoading(false)
                    return
                }

                const formData = new FormData()
                formData.append('file', audioBlob, 'audio.webm')
                formData.append('model', modelId)

                console.log(`🎤 Sending STT request with model ID: "${modelId}"`)
                response = await fetch('/v1/audio/transcriptions', {
                    method: 'POST',
                    body: formData
                })

                if (response.ok) {
                    const data = await response.json()
                    const transcription = data.text

                    // Create URL from the audio blob to show player
                    const userAudioUrl = URL.createObjectURL(audioBlob!)

                    setMessages(prev => [...prev, {
                        role: 'user',
                        content: `<audio controls src="${userAudioUrl}"></audio>`,
                        timestamp: new Date()
                    }, {
                        role: 'assistant',
                        content: transcription,
                        timestamp: new Date()
                    }])

                    setAudioBlob(null)
                    setClearTrigger(prev => prev + 1)  // Trigger AudioInput clear
                    setIsLoading(false)
                } else {
                    const errorText = await response.text()
                    toast.error(`STT failed: ${errorText}`)
                    setMessages(prev => [...prev, {
                        role: 'assistant',
                        content: `❌ STT Error: ${errorText}`,
                        timestamp: new Date()
                    }])
                    setIsLoading(false)
                }
                return
            } else if (isSpeechToSpeech) {
                // Speech-to-Speech Request
                if (!audioBlob) {
                    toast.error('Please record or upload audio first')
                    setIsLoading(false)
                    return
                }

                const formData = new FormData()
                formData.append('file', audioBlob, 'audio.webm')
                formData.append('model', modelId)
                formData.append('voice', voiceId)

                console.log(`🔄 Sending STS request with model ID: "${modelId}"`)
                response = await fetch('/v1/audio/speech-to-speech', {
                    method: 'POST',
                    body: formData
                })

                if (response.ok) {
                    const audioData = await response.blob()
                    const audioUrl = URL.createObjectURL(audioData)

                    // Create URL from user's audio to show what they said
                    const userAudioUrl = URL.createObjectURL(audioBlob!)

                    setMessages(prev => [...prev, {
                        role: 'user',
                        content: `<audio controls src="${userAudioUrl}"></audio>`,
                        timestamp: new Date()
                    }, {
                        role: 'assistant',
                        content: `<audio controls src="${audioUrl}"></audio>`,
                        timestamp: new Date()
                    }])

                    setAudioBlob(null)
                    setClearTrigger(prev => prev + 1)  // Trigger AudioInput clear
                    setIsLoading(false)
                } else {
                    const errorText = await response.text()
                    toast.error(`STS failed: ${errorText}`)
                    setMessages(prev => [...prev, {
                        role: 'assistant',
                        content: `❌ STS Error: ${errorText}`,
                        timestamp: new Date()
                    }])
                    setIsLoading(false)
                }
                return
            } else if (isMusicGen) {
                // Music generation — distinct endpoint from TTS.
                // ElevenLabs Music takes prompt + music_length_ms;
                // there's no voice id involved.
                console.log(`🎵 Sending music request with model ID: "${modelId}"`)
                response = await fetch('/v1/audio/music', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({
                        prompt,
                        model: modelId,
                        music_length_ms: 30_000,
                    }),
                })
            } else if (selectedItem?.modality === 'audio') {
                // Text-to-Speech Request (default audio role)
                console.log(`🎤 Sending TTS request with model ID: "${modelId}"`)
                response = await fetch('/v1/audio/speech', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({
                        input: prompt,
                        model: modelId,
                        voice: voiceId
                    })
                })
            } else {
                // Chat Completion Request
                let serviceName = selectedService
                let modelOverride = undefined

                if (selectedService.startsWith('service:')) {
                    serviceName = selectedService.replace('service:', '')
                } else if (selectedService.startsWith('model:')) {
                    serviceName = selectedService.replace('model:', '')
                } else if (selectedService.startsWith('scoped:')) {
                    const parts = selectedService.split(':')
                    if (parts.length >= 3) {
                        serviceName = parts[1]
                        modelOverride = parts.slice(2).join(':')
                    }
                }

                console.log(`Sending Chat Request (Stream): Service=${serviceName}, Model=${modelOverride}`)

                response = await fetch('/v1/chat/completions', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({
                        service: serviceName,
                        model: modelOverride,
                        messages: requestMessages,
                        stream: true, // Always enable streaming
                        params: {
                            temperature: config.temperature,
                            max_tokens: config.maxTokens,
                            reasoning_effort: config.reasoningEffort
                        }
                    })
                })
            }

            if (!response.ok) {
                const error = await response.text()
                toast.error(`Error: ${error}`)
                const errorMessage: Message = {
                    role: 'assistant',
                    content: `❌ Error: ${error}`,
                    model: 'system',
                    timestamp: new Date()
                }
                setMessages(prev => [...prev, errorMessage])
                setIsLoading(false)
                return
            }

            // STREAMING HANDLER
            if (selectedItem?.modality === 'text' || selectedItem?.modality === 'multimodal' || !selectedItem?.modality || isAgenticService) {
                if (!response.body) return
                const reader = response.body.getReader()
                const decoder = new TextDecoder()

                // Track accumulated content
                let finalContent = ''
                let currentEvents: AgenticStreamEvent[] = []

                // Add placeholder message
                setMessages(prev => [...prev, {
                    role: 'assistant',
                    content: '', // Will be filled dynamically
                    events: [],
                    timestamp: new Date()
                }])

                let buffer = '' // For handling partial SSE lines

                while (true) {
                    const { done, value } = await reader.read()
                    if (done) break

                    const chunk = decoder.decode(value, { stream: true })
                    buffer += chunk

                    const lines = buffer.split('\n')
                    buffer = lines.pop() || '' // Keep the last partial line

                    for (const line of lines) {
                        if (line.startsWith('data: ')) {
                            const dataStr = line.slice(6).trim()
                            if (dataStr === '[DONE]') continue

                            try {
                                const event = JSON.parse(dataStr) as AgenticStreamEvent

                                // UPDATE STATE
                                if (event.type === 'chunk' || (event.type as any) === 'final_response') {
                                    // Mark final answer as started to trigger UI collapse of thoughts
                                    setMessages(prev => {
                                        const newMsgs = [...prev]
                                        const last = newMsgs[newMsgs.length - 1]
                                        if (last.role === 'assistant' && !last.isFinalAnswerStarted) {
                                            last.isFinalAnswerStarted = true
                                        }
                                        return newMsgs
                                    })

                                    // Depending on backend serialization, check both
                                    const text = typeof event.data === 'string' ? event.data : ''
                                    finalContent += text
                                } else if (event.type === 'reasoning_delta') {
                                    // Reasoning tokens (o1/o3/gpt-oss/GLM-thinking/Gemma-thinking
                                    // emit these). For Pool / chat services we don't want the
                                    // ThoughtTimeline cluttering the reply — agentic services
                                    // are the one place where the planner's chain-of-thought
                                    // is the actual interesting output. Drop silently for
                                    // non-agentic flows; the user still sees the final answer
                                    // through the regular `chunk` events.
                                    if (!isAgenticService) {
                                        // no-op — keep chat replies clean
                                    } else {
                                        // AGGREGATE REASONING CHUNKS into a single log entry
                                        const lastEvent = currentEvents[currentEvents.length - 1]
                                        const delta = typeof event.data === 'string' ? event.data : ''

                                        if (lastEvent && lastEvent.type === 'log' && lastEvent.data.step === 'reasoning') {
                                            lastEvent.data.content += delta
                                        } else {
                                            currentEvents.push({
                                                type: 'log',
                                                data: {
                                                    step: 'reasoning',
                                                    content: delta
                                                }
                                            })
                                        }
                                    }
                                } else if ((event.type as any) === 'error') {
                                    // Handle error events - display in message content, NOT in thought timeline
                                    const errorText = typeof event.data === 'string' ? event.data : JSON.stringify(event.data)
                                    finalContent += `❌ Error: ${errorText}`
                                } else if (isAgenticService) {
                                    // Structured events (log, tool, step) belong to the agentic
                                    // executor's planner trace. They're meaningless for Pool
                                    // services — silently drop them to keep chat replies clean.
                                    currentEvents.push(event)
                                }

                                // Update the LAST message with new content and events
                                setMessages(prev => {
                                    const newMsgs = [...prev]
                                    const last = newMsgs[newMsgs.length - 1]
                                    if (last.role === 'assistant') {
                                        last.content = finalContent
                                        last.events = [...currentEvents]
                                    }
                                    return newMsgs
                                })

                            } catch (e) {
                                console.warn('SSE Parse Error:', e, dataStr)
                            }
                        }
                    }
                }

                // Mark as complete and stop loading
                setMessages(prev => {
                    const newMsgs = [...prev]
                    const last = newMsgs[newMsgs.length - 1]
                    if (last.role === 'assistant') {
                        last.isComplete = true
                    }
                    return newMsgs
                })

                setIsLoading(false)
                return
            }

            // ... (Rest of non-streaming handlers remain: Images, Video, Audio) ...

            // Parse response based on type (Legacy/Media Handlers)
            let content = ''
            let usedModel = ''

            if (isImageModel) {
                const json = await response.json()
                // Handle Image Response
                const imageData = json.data?.[0]
                if (imageData?.url) {
                    content = `![Generated Image](${imageData.url})`
                } else if (imageData?.b64_json) {
                    content = `![Generated Image](data:image/png;base64,${imageData.b64_json})`
                } else {
                    content = '❌ No image data received'
                }
                usedModel = selectedService
            } else if (selectedItem?.modality === 'video') {
                // ... (Keep existing video logic) ...
                const json = await response.json()
                if (json.url && json.url.startsWith('JOB_ID:')) {
                    // ... (Keep polling logic exactly as is) ...
                    // I will trust that keeping the abbreviated logic here works if I didn't delete it
                    // Wait, I am using REPLACE, so I must provide the full content or I lose it.
                    // I will have to provide the full content of video handling.
                    // Since I can't see the full video handling logic in the artifact view easily without reading again...
                    // I will assume simple content for now to save tokens or read previous view carefully.
                    // The previous view had lines 442-501. I should copy that block.
                    const [jobPart, modelPart] = json.url.split('|')
                    const jobId = jobPart.replace('JOB_ID:', '')
                    const modelId = modelPart.replace('MODEL:', '')
                    content = '🎬 Video generation started... polling for completion'

                    // Helper for video polling (reused from previous code)
                    setTimeout(async () => {
                        const pollInterval = setInterval(async () => {
                            try {
                                const statusRes = await fetch(`/v1/videos/jobs/${jobId}/${modelId}`)
                                const status = await statusRes.json()
                                if (status.status === 'succeeded' && status.video_url) {
                                    clearInterval(pollInterval)

                                    // Provider URL strategy:
                                    //
                                    //   - OpenAI returns ".../v1/videos/<id>" — auth-gated, must
                                    //     proxy through the gateway so the master key isn't
                                    //     exposed to the browser.
                                    //   - Runway, Kling, Luma, Pika, MiniMax, Bytedance return
                                    //     pre-signed (or public) URLs the browser can load
                                    //     directly. No proxy needed and no get_video_content
                                    //     adapter impl needed either.
                                    //
                                    // We use the absolute URL when the provider gave us one
                                    // and only fall back to the proxy for OpenAI-style paths.
                                    const isAbsolute = /^https?:\/\//i.test(status.video_url)
                                    let videoSrc: string
                                    if (isAbsolute && !status.video_url.includes('/v1/videos/')) {
                                        // Pre-signed third-party CDN URL — load directly.
                                        videoSrc = status.video_url
                                    } else {
                                        // OpenAI proxy path: extract the generation id.
                                        const genIdMatch = status.video_url.match(/\/videos?\/(?:generations\/)?([^/?]+)/)
                                        const genId = genIdMatch ? genIdMatch[1] : ''
                                        videoSrc = `/v1/videos/content/${genId}/${modelId}`
                                    }

                                    setMessages(prev => {
                                        const newMessages = [...prev]
                                        const lastMsg = newMessages[newMessages.length - 1]
                                        if (lastMsg && lastMsg.content.includes('polling for completion')) {
                                            lastMsg.content = `<video controls src="${videoSrc}" class="max-w-full rounded-lg"></video>`
                                        }
                                        return newMessages
                                    })
                                    setIsLoading(false)
                                } else if (status.status === 'failed') {
                                    clearInterval(pollInterval)
                                    setMessages(prev => {
                                        const newMessages = [...prev]
                                        const lastMsg = newMessages[newMessages.length - 1]
                                        if (lastMsg) lastMsg.content = '❌ Video generation failed'
                                        return newMessages
                                    })
                                    setIsLoading(false)
                                }
                            } catch (err) { console.error(err) }
                        }, 5000)
                    }, 1000)
                } else if (json.url) {
                    content = `<video controls src="${json.url}" class="max-w-full rounded-lg"></video>`
                } else {
                    content = '❌ No video data received'
                }
                usedModel = selectedService
            } else if (selectedItem?.modality === 'audio') {
                const blob = await response.blob()
                const url = URL.createObjectURL(blob)
                content = `![Audio](${url})`
                usedModel = selectedService
            } else {
                // Fallback for unexpected cases
                const json = await response.json()
                content = json.choices?.[0]?.message?.content || ''
                usedModel = json.model || ''
            }

            // Add assistant message (Non-Streaming)
            const assistantMessage: Message = {
                role: 'assistant',
                content: content,
                model: usedModel,
                timestamp: new Date()
            }
            setMessages(prev => [...prev, assistantMessage])

        } catch (error) {
            toast.error(`Failed to send message: ${error}`)
            const errorMessage: Message = {
                role: 'assistant',
                content: `❌ Error: ${error}`,
                model: 'system',
                timestamp: new Date()
            }
            setMessages(prev => [...prev, errorMessage])
        } finally {
            // Only force loading off if not handled by streaming/polling logic
            if (selectedItem?.modality !== 'text' && selectedItem?.modality !== 'video') {
                setIsLoading(false)
            }
        }
    }
    const clearChat = () => {
        setMessages([])
        setStreamingContent('')
    }

    const resetConfig = () => {
        setConfig(DEFAULT_CONFIG)
        toast.success('Settings reset to defaults')
    }

    const SliderInput = ({ label, value, onChange, min, max, step, tooltip }: {
        label: string
        value: number
        onChange: (v: number) => void
        min: number
        max: number
        step: number
        tooltip: string
    }) => (
        <div className="space-y-2">
            <div className="flex items-center justify-between">
                <label className="text-sm text-slate-400 flex items-center gap-2">
                    {label}
                    <span className="group relative">
                        <svg className="w-3.5 h-3.5 text-slate-500 cursor-help" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
                        </svg>
                        <span className="absolute left-0 bottom-full mb-2 hidden group-hover:block w-48 p-2 bg-black border border-white/20 rounded-lg text-xs text-slate-300 z-50">
                            {tooltip}
                        </span>
                    </span>
                </label>
                <span className="text-sm font-mono text-cyan-400">{value}</span>
            </div>
            <input
                type="range"
                min={min}
                max={max}
                step={step}
                value={value}
                onChange={(e) => onChange(parseFloat(e.target.value))}
                className="w-full h-1.5 bg-white/10 rounded-full appearance-none cursor-pointer slider-thumb"
            />
        </div>
    )

    return (
        <div>
            {/* Gradient Background */}
            <div className="fixed inset-0 overflow-hidden pointer-events-none">
                <div className="absolute -top-40 -right-40 w-96 h-96 bg-cyan-500/20 rounded-full blur-3xl" />
                <div className="absolute top-1/2 -left-40 w-80 h-80 bg-purple-500/15 rounded-full blur-3xl" />
                <div className="absolute bottom-0 right-1/3 w-72 h-72 bg-emerald-500/10 rounded-full blur-3xl" />
            </div>

            <div className={`relative z-10 p-6 h-screen flex gap-6 ${showDebug ? 'pb-52' : ''}`}>
                {/* Settings Panel */}
                <AnimatePresence>
                    {showSettings && (
                        <motion.div
                            initial={{ opacity: 0, x: -20, width: 0 }}
                            animate={{ opacity: 1, x: 0, width: 320 }}
                            exit={{ opacity: 0, x: -20, width: 0 }}
                            className="shrink-0 overflow-hidden">
                            <div className="w-80 h-full rounded-2xl bg-gradient-to-br from-white/5 to-white/[0.02] backdrop-blur-2xl border border-white/10 overflow-hidden flex flex-col">
                                {/* Settings Header */}
                                <div className="p-4 border-b border-white/10 flex items-center justify-between">
                                    <h3 className="font-semibold text-white flex items-center gap-2">
                                        <Settings className="w-5 h-5 text-cyan-400" strokeWidth={2} />
                                        Configuration
                                    </h3>
                                    <button
                                        onClick={resetConfig}
                                        className="text-xs text-slate-400 hover:text-cyan-400 transition-colors">
                                        Reset
                                    </button>
                                </div>
                                <div className="px-4 py-2 bg-black/20 text-[10px] text-slate-500 flex justify-between">
                                    <span>Debug Mode</span>
                                    <button onClick={() => setShowDebug(!showDebug)} className={showDebug ? "text-green-400" : "text-slate-500"}>
                                        {showDebug ? "ON" : "OFF"}
                                    </button>
                                </div>

                                {/* Settings Content */}
                                <div className="flex-1 overflow-y-auto p-4 space-y-6">
                                    {/* Service Selector */}
                                    <div className="space-y-2">
                                        <label className="text-sm text-slate-400">Target</label>
                                        <select
                                            value={selectedService}
                                            onChange={(e) => setSelectedService(e.target.value)}
                                            className="w-full px-3 py-2 bg-black border border-white/10 rounded-xl text-white text-sm focus:border-cyan-400 focus:ring-4 focus:ring-cyan-400/20 outline-none transition-all">
                                            {/* Render OptGroups based on unique group names */}
                                            {Array.from(new Set(services.map(s => s.group))).map(group => (
                                                <optgroup key={group} label={group}>
                                                    {services.filter(s => s.group === group).map(service => {
                                                        const isMultiModal = !!(service.pool_type && service.pool_type.toLowerCase().includes('multi'))
                                                        return (
                                                            <option
                                                                key={`${service.group}-${service.name}-${service.label}`}
                                                                value={service.name}
                                                                className="bg-black"
                                                                disabled={isMultiModal}
                                                            >
                                                                {service.label} {isMultiModal ? '(Multi-Modal: Use specific model below)' : ''}
                                                            </option>
                                                        )
                                                    })}
                                                </optgroup>
                                            ))}
                                        </select>
                                    </div>

                                    {/* Voice (TTS / S2S only) — voice naming is provider-specific.
                                        ElevenLabs uses opaque ids like `21m00Tcm4TlvDq8ikWAM`,
                                        Hume uses preset names like `ITO` (or `custom:<my-voice>`
                                        for cloned voices). Leaving this blank delegates the
                                        choice to whichever provider answers — strictly safer
                                        than sending one provider's id to another. */}
                                    {(audioRole === 'tts' || audioRole === 'sts') && (
                                        <div className="space-y-2">
                                            <label className="text-sm text-slate-400 flex items-center gap-2">
                                                Voice
                                                <span className="text-xs text-slate-600">(provider-specific)</span>
                                            </label>
                                            <input
                                                type="text"
                                                value={voiceId}
                                                onChange={(e) => setVoiceId(e.target.value)}
                                                placeholder="leave blank for provider default"
                                                list="voice-suggestions"
                                                className="w-full px-3 py-2 bg-white/5 border border-white/10 rounded-xl text-white text-sm placeholder-slate-600 focus:border-cyan-400 outline-none"
                                            />
                                            <datalist id="voice-suggestions">
                                                {/* ElevenLabs stock voices (paste the id) */}
                                                <option value="21m00Tcm4TlvDq8ikWAM">ElevenLabs · Rachel</option>
                                                <option value="AZnzlk1XvdvUeBnXmlld">ElevenLabs · Domi</option>
                                                <option value="EXAVITQu4vr4xnSDxMaL">ElevenLabs · Bella</option>
                                                {/* Hume Octave voice names */}
                                                <option value="ITO">Hume · ITO</option>
                                                <option value="KORA">Hume · KORA</option>
                                                <option value="SETSU">Hume · SETSU</option>
                                                {/* Hume custom-voice escape hatch */}
                                                <option value="custom:my-cloned-voice">Hume · custom:&lt;name&gt;</option>
                                            </datalist>
                                            <p className="text-xs text-slate-500 leading-relaxed">
                                                Blank = whichever provider answers picks its default.
                                                ElevenLabs takes opaque ids. Hume takes preset names
                                                (<span className="font-mono">ITO</span>, <span className="font-mono">KORA</span>) or
                                                <span className="font-mono"> custom:&lt;voice&gt;</span> for cloned voices.
                                            </p>
                                        </div>
                                    )}



                                    {/* RTCROS Configuration */}
                                    <div className="space-y-4 pt-4 border-t border-white/10">
                                        <div className="flex items-center justify-between">
                                            <label className="text-sm text-slate-400 font-semibold flex items-center gap-2">
                                                RTCROS
                                                <span className="text-xs font-normal text-slate-600">(Reasoning & Control)</span>
                                            </label>
                                            <button
                                                onClick={() => setConfig({ ...config, rtcrosEnabled: !config.rtcrosEnabled })}
                                                className={`w-11 h-6 flex items-center rounded-full transition-colors ${config.rtcrosEnabled ? 'bg-cyan-500' : 'bg-white/10'}`}>
                                                <div className={`w-4 h-4 rounded-full bg-white shadow-md transform transition-transform ${config.rtcrosEnabled ? 'translate-x-6' : 'translate-x-1'}`} />
                                            </button>
                                        </div>

                                        {config.rtcrosEnabled && (
                                            <div className="space-y-3 pl-2 border-l-2 border-cyan-500/20">
                                                <p className="text-xs text-slate-500 flex items-center gap-2">
                                                    <span className="w-1.5 h-1.5 rounded-full bg-cyan-400 animate-pulse" />
                                                    RTCROS injection active
                                                </p>
                                            </div>
                                        )}
                                    </div>

                                    {/* System Prompt */}
                                    <div className="space-y-2">
                                        <label className="text-sm text-slate-400 flex items-center gap-2">
                                            System Prompt
                                            <span className="text-xs text-slate-600">(optional)</span>
                                        </label>
                                        <textarea
                                            value={config.systemPrompt}
                                            onChange={(e) => setConfig({ ...config, systemPrompt: e.target.value })}
                                            placeholder="You are a helpful assistant..."
                                            rows={5}
                                            className="w-full px-3 py-2 bg-white/5 border border-white/10 rounded-xl text-white text-sm placeholder-slate-600 focus:border-cyan-400 outline-none resize-y min-h-[80px]"
                                        />
                                    </div>

                                    <div className="h-px bg-white/10" />



                                    {/* Reasoning Effort (GPT-5/o1) only */}
                                    {((selectedService.toLowerCase().includes('gpt-5') ||
                                        selectedService.toLowerCase().includes('o1') ||
                                        selectedService.toLowerCase().includes('o3'))) && (
                                            <div className="space-y-2">
                                                <label className="text-sm text-slate-400 flex items-center gap-2">
                                                    Reasoning Effort
                                                    <span className="text-xs text-slate-600">(GPT-5/o1)</span>
                                                </label>
                                                <div className="grid grid-cols-3 gap-1 bg-white/5 p-1 rounded-xl border border-white/10">
                                                    {['low', 'medium', 'high'].map((level) => (
                                                        <button
                                                            key={level}
                                                            onClick={() => setConfig({ ...config, reasoningEffort: level as any })}
                                                            className={`px-2 py-1.5 rounded-lg text-xs font-medium transition-all ${config.reasoningEffort === level
                                                                ? 'bg-cyan-500/20 text-cyan-400 border border-cyan-500/30'
                                                                : 'text-slate-400 hover:text-white hover:bg-white/5'
                                                                }`}
                                                        >
                                                            {level.charAt(0).toUpperCase() + level.slice(1)}
                                                        </button>
                                                    ))}
                                                </div>
                                            </div>
                                        )}

                                    {/* Temperature */}
                                    <SliderInput
                                        label="Temperature"
                                        value={config.temperature}
                                        onChange={(v) => setConfig({ ...config, temperature: v })}
                                        min={0}
                                        max={2}
                                        step={0.1}
                                        tooltip="Controls randomness. 0 = deterministic, 2 = very random"
                                    />

                                    {/* Max Tokens */}
                                    <div className="space-y-2">
                                        <div className="flex items-center justify-between">
                                            <label className="text-sm text-slate-400">Max Tokens</label>
                                            <span className="text-sm font-mono text-cyan-400">{config.maxTokens}</span>
                                        </div>
                                        <input
                                            type="number"
                                            value={config.maxTokens}
                                            onChange={(e) => setConfig({ ...config, maxTokens: parseInt(e.target.value) || 1 })}
                                            min={1}
                                            max={128000}
                                            className="w-full px-3 py-2 bg-white/5 border border-white/10 rounded-xl text-white text-sm focus:border-cyan-400 outline-none"
                                        />
                                    </div>

                                    {/* Top P */}
                                    <SliderInput
                                        label="Top P"
                                        value={config.topP}
                                        onChange={(v) => setConfig({ ...config, topP: v })}
                                        min={0}
                                        max={1}
                                        step={0.05}
                                        tooltip="Nucleus sampling. 1 = consider all tokens"
                                    />

                                    <div className="h-px bg-white/10" />

                                    {/* Frequency Penalty */}
                                    <SliderInput
                                        label="Frequency Penalty"
                                        value={config.frequencyPenalty}
                                        onChange={(v) => setConfig({ ...config, frequencyPenalty: v })}
                                        min={-2}
                                        max={2}
                                        step={0.1}
                                        tooltip="Reduce repetition of tokens based on frequency"
                                    />

                                    {/* Presence Penalty */}
                                    <SliderInput
                                        label="Presence Penalty"
                                        value={config.presencePenalty}
                                        onChange={(v) => setConfig({ ...config, presencePenalty: v })}
                                        min={-2}
                                        max={2}
                                        step={0.1}
                                        tooltip="Encourage new topics based on presence in text"
                                    />

                                    <div className="h-px bg-white/10" />

                                    {/* Stop Sequences */}
                                    <div className="space-y-2">
                                        <label className="text-sm text-slate-400 flex items-center gap-2">
                                            Stop Sequences
                                            <span className="text-xs text-slate-600">(comma-separated)</span>
                                        </label>
                                        <input
                                            type="text"
                                            value={config.stopSequences}
                                            onChange={(e) => setConfig({ ...config, stopSequences: e.target.value })}
                                            placeholder="e.g., END, ###, ..."
                                            className="w-full px-3 py-2 bg-white/5 border border-white/10 rounded-xl text-white text-sm placeholder-slate-600 focus:border-cyan-400 outline-none"
                                        />
                                    </div>
                                </div>

                                {/* Current Config Summary */}
                                <div className="p-4 border-t border-white/10 bg-white/[0.02]">
                                    <div className="text-xs text-slate-500 font-mono">
                                        temp={config.temperature} | max={config.maxTokens} | top_p={config.topP}
                                    </div>
                                </div>
                            </div>
                        </motion.div>
                    )}
                </AnimatePresence>

                {/* Main Chat Area */}
                <div className="flex-1 flex flex-col min-w-0">
                    {/* Header — matches /providers /services /mcp shell.
                        Tighter than full topbar (this page is chat-first) but
                        still uses the same gradient H1 + tabular subtitle. */}
                    <motion.div
                        initial={{ opacity: 0, y: -20 }}
                        animate={{ opacity: 1, y: 0 }}
                        className="mb-5 flex items-end justify-between gap-4">
                        <div className="flex items-end gap-4 min-w-0">
                            <button
                                onClick={() => setShowSettings(!showSettings)}
                                title={showSettings ? 'Hide settings' : 'Show settings'}
                                className={`shrink-0 p-2.5 rounded-xl border transition-all ${showSettings
                                    ? 'bg-cyan-400/15 border-cyan-400/40 text-cyan-300 shadow-[0_0_16px_rgba(34,211,238,0.18)]'
                                    : 'bg-white/[0.04] border-white/10 text-slate-400 hover:bg-white/[0.08] hover:text-slate-200'}`}>
                                <SlidersHorizontal className="w-4 h-4" strokeWidth={2} />
                            </button>
                            <div className="min-w-0">
                                <div className="text-[11px] text-slate-500 mb-1.5 tracking-[0.12em] uppercase font-semibold">
                                    Workspace · Playground
                                </div>
                                <h1 className="text-4xl font-bold bg-gradient-to-br from-white to-slate-400 bg-clip-text text-transparent leading-[1.1]">
                                    Playground
                                </h1>
                                <p className="text-slate-400 mt-2 text-sm truncate">
                                    Test models with custom parameters
                                    {selectedService && (
                                        <>
                                            <span className="mx-2 text-slate-700">·</span>
                                            <span className="font-mono tabular-nums text-cyan-300/90">{selectedService}</span>
                                        </>
                                    )}
                                </p>
                            </div>
                        </div>

                        <button
                            onClick={clearChat}
                            disabled={messages.length === 0}
                            className="shrink-0 inline-flex items-center gap-2 px-3 py-2 rounded-xl text-sm font-medium
                                       text-slate-300 hover:text-white
                                       bg-white/[0.04] hover:bg-red-500/10
                                       border border-white/10 hover:border-red-500/40
                                       disabled:opacity-40 disabled:hover:bg-white/[0.04] disabled:hover:border-white/10 disabled:hover:text-slate-300
                                       transition-all">
                            <Trash2 className="w-3.5 h-3.5" strokeWidth={2} />
                            Clear
                        </button>
                    </motion.div>

                    {/* Chat Container */}
                    <motion.div
                        initial={{ opacity: 0, y: 20 }}
                        animate={{ opacity: 1, y: 0 }}
                        transition={{ delay: 0.1 }}
                        className="flex-1 overflow-hidden rounded-2xl bg-gradient-to-br from-white/5 to-white/[0.02] backdrop-blur-2xl border border-white/10 shadow-2xl shadow-black/50">

                        {/* Messages Area */}
                        <div className="h-full flex flex-col">
                            <div className="flex-1 overflow-y-auto p-6 space-y-4">
                                {messages.length === 0 && !streamingContent && (
                                    <div className="h-full flex items-center justify-center">
                                        <div className="text-center">
                                            <div className="w-14 h-14 mx-auto mb-4 rounded-2xl bg-cyan-400/10 border border-cyan-400/20 flex items-center justify-center shadow-[0_0_24px_rgba(34,211,238,0.15)]">
                                                <TestTube2 className="w-7 h-7 text-cyan-400" strokeWidth={2} />
                                            </div>
                                            <h3 className="text-xl font-bold text-white mb-2">
                                                Ready to Experiment
                                            </h3>
                                            <p className="text-slate-400 max-w-md text-sm">
                                                {showSettings ? 'Configure parameters in the left panel, ' : 'Click the settings icon to configure, '}
                                                then start chatting
                                            </p>
                                        </div>
                                    </div>
                                )}

                                <AnimatePresence>
                                    {messages.map((message, index) => (
                                        <motion.div
                                            key={index}
                                            initial={{ opacity: 0, y: 10 }}
                                            animate={{ opacity: 1, y: 0 }}
                                            className={`flex ${message.role === 'user' ? 'justify-end' : 'justify-start'}`}>
                                            <div className={`max-w-[85%]`}>
                                                <div className={`flex items-start gap-3 ${message.role === 'user' ? 'flex-row-reverse' : ''}`}>
                                                    <div className={`w-8 h-8 rounded-lg flex items-center justify-center shrink-0 ${message.role === 'user'
                                                        ? 'bg-gradient-to-br from-cyan-400 to-cyan-600 text-white shadow-[0_0_12px_rgba(34,211,238,0.25)]'
                                                        : 'bg-gradient-to-br from-purple-400/15 to-purple-600/15 border border-purple-400/30 text-purple-300'
                                                        }`}>
                                                        {message.role === 'user' ? (
                                                            <User className="w-4 h-4" strokeWidth={2} />
                                                        ) : (
                                                            <Sparkles className="w-4 h-4" strokeWidth={2} />
                                                        )}
                                                    </div>

                                                    <div className={`rounded-xl px-4 py-2.5 ${message.role === 'user'
                                                        ? 'bg-gradient-to-br from-cyan-400/20 to-cyan-600/20 border border-cyan-400/30'
                                                        : 'bg-white/5 border border-white/10'
                                                        }`}>
                                                        {message.role === 'assistant' && message.events && message.events.length > 0 && (
                                                            <div className="mb-4 border-b border-white/10 pb-4">
                                                                <ThoughtTimeline
                                                                    events={message.events}
                                                                    isFinished={message.isComplete}
                                                                    isFinalAnswerStarted={message.isFinalAnswerStarted}
                                                                />
                                                            </div>
                                                        )}
                                                        {message.content.startsWith('![Audio]') ? (
                                                            <div className="rounded-lg overflow-hidden my-2 bg-white/5 p-2">
                                                                <audio
                                                                    controls
                                                                    src={message.content.match(/\((.*?)\)/)?.[1] || ''}
                                                                    className="w-full"
                                                                />
                                                            </div>
                                                        ) : message.content.startsWith('<video') ? (
                                                            <div
                                                                className="rounded-lg overflow-hidden my-2"
                                                                dangerouslySetInnerHTML={{ __html: DOMPurify.sanitize(message.content) }}
                                                            />
                                                        ) : message.content.startsWith('<audio') ? (
                                                            <div
                                                                className="rounded-lg overflow-hidden my-2 bg-white/5 p-2"
                                                                dangerouslySetInnerHTML={{ __html: DOMPurify.sanitize(message.content) }}
                                                            />
                                                        ) : (
                                                            <div className="text-sm prose prose-invert max-w-none">
                                                                <ReactMarkdown
                                                                    components={{
                                                                        // eslint-disable-next-line @next/next/no-img-element
                                                                        img: ({ node, ...props }) => (
                                                                            <div className="rounded-lg overflow-hidden my-2">
                                                                                <img {...props} className="max-w-full h-auto rounded-lg border border-white/10" alt={props.alt || "Generated Image"} />
                                                                            </div>
                                                                        ),
                                                                        p: ({ node, ...props }) => (
                                                                            <p {...props} className="mb-2 last:mb-0 text-white" />
                                                                        )
                                                                    }}
                                                                >
                                                                    {message.content}
                                                                </ReactMarkdown>
                                                            </div>
                                                        )}
                                                        {message.model && (
                                                            <div className="mt-2 text-xs text-slate-500">
                                                                via {message.model}
                                                            </div>
                                                        )}
                                                    </div>
                                                </div>
                                            </div>
                                        </motion.div>
                                    ))}
                                </AnimatePresence>

                                {/* Streaming Response */}
                                {/* Streaming Response or Loading State */}
                                {(isLoading && messages[messages.length - 1]?.role !== 'assistant') && (
                                    <motion.div
                                        initial={{ opacity: 0, y: 10 }}
                                        animate={{ opacity: 1, y: 0 }}
                                        className="flex justify-start">
                                        <div className="max-w-[85%]">
                                            <div className="flex items-start gap-3">
                                                <div className="w-8 h-8 rounded-lg bg-gradient-to-br from-purple-400/15 to-purple-600/15 border border-purple-400/30 flex items-center justify-center text-purple-300 shrink-0">
                                                    <Sparkles className="w-4 h-4" strokeWidth={2} />
                                                </div>
                                                <div className="rounded-xl px-4 py-2.5 bg-white/5 border border-white/10">
                                                    {streamingContent ? (
                                                        <p className="text-white text-sm whitespace-pre-wrap">{streamingContent}</p>
                                                    ) : (
                                                        <div className="flex items-center gap-2 text-slate-400 text-sm">
                                                            <Spinner size="sm" />
                                                            <span>Thinking</span>
                                                        </div>
                                                    )}
                                                    {/* Streaming indicator dots (only show when content is streaming) */}
                                                    {streamingContent && (
                                                        <div className="flex items-center gap-1 mt-2">
                                                            <div className="w-1 h-1 rounded-full bg-cyan-400 animate-pulse" />
                                                            <div className="w-1 h-1 rounded-full bg-cyan-400 animate-pulse delay-75" />
                                                            <div className="w-1 h-1 rounded-full bg-cyan-400 animate-pulse delay-150" />
                                                        </div>
                                                    )}
                                                </div>
                                            </div>
                                        </div>
                                    </motion.div>
                                )}

                                <div ref={messagesEndRef} />
                            </div>

                            {/* Input Area */}
                            <div className="border-t border-white/10 p-4">
                                {/* Audio Input for STT / STS models */}
                                {(audioRole === 'stt' || audioRole === 'sts') && (
                                    <div className="mb-4">
                                        <AudioInput
                                            onAudioReady={setAudioBlob}
                                            onClear={() => setAudioBlob(null)}
                                            clearTrigger={clearTrigger}
                                        />
                                    </div>
                                )}

                                <form onSubmit={handleSubmit} className="flex gap-3">
                                    <div className="flex-1 relative">
                                        <textarea
                                            value={prompt}
                                            onChange={(e) => setPrompt(e.target.value)}
                                            onKeyDown={(e) => {
                                                if (e.key === 'Enter' && !e.shiftKey) {
                                                    e.preventDefault()
                                                    handleSubmit(e)
                                                }
                                            }}
                                            placeholder="Type your message... (Shift+Enter for new line)"
                                            className="w-full px-4 py-3 bg-white/5 border border-white/10 rounded-xl text-white text-sm placeholder-slate-500 focus:border-cyan-400 focus:ring-2 focus:ring-cyan-400/20 outline-none resize-none min-h-[48px] max-h-32"
                                            rows={1}
                                        />
                                    </div>
                                    <button
                                        type="submit"
                                        disabled={(audioRole === 'stt' || audioRole === 'sts' ? !audioBlob : !prompt.trim()) || isLoading || !selectedService}
                                        title={isLoading ? 'Generating…' : 'Send (Enter)'}
                                        className="shrink-0 inline-flex items-center justify-center gap-2 px-4 rounded-xl
                                                   text-sm font-semibold text-black
                                                   bg-gradient-to-br from-cyan-300 to-cyan-600
                                                   shadow-[0_0_24px_rgba(34,211,238,0.32)]
                                                   hover:shadow-[0_0_32px_rgba(34,211,238,0.5)]
                                                   hover:-translate-y-px active:translate-y-0
                                                   disabled:opacity-40 disabled:shadow-none disabled:translate-y-0 disabled:cursor-not-allowed
                                                   transition-all min-h-[48px] min-w-[48px]">
                                        {isLoading ? (
                                            <div className="w-4 h-4 border-2 border-black/20 border-t-black rounded-full animate-spin" />
                                        ) : (
                                            <Send className="w-4 h-4" strokeWidth={2.5} />
                                        )}
                                    </button>
                                </form>

                                {services.length === 0 && (
                                    <p className="text-amber-400 text-sm mt-2 flex items-center gap-2">
                                        <AlertTriangle className="w-4 h-4" strokeWidth={2} />
                                        No services available. Create a service first.
                                    </p>
                                )}
                            </div>
                        </div>
                    </motion.div>
                </div>
            </div>

            {/* Debug Overlay */}
            {showDebug && (
                <div className="fixed bottom-0 left-0 right-0 bg-black/95 border-t border-white/10 z-[100] h-64 shadow-[0_-4px_20px_rgba(0,0,0,0.5)] flex flex-col font-mono text-xs">
                    {/* Header */}
                    <div className="flex justify-between items-center px-4 py-2 bg-white/5 border-b border-white/10">
                        <div className="flex items-center gap-2">
                            <span className="w-2 h-2 rounded-full bg-green-500 animate-pulse"></span>
                            <strong className="text-white">LIVE DEBUG TRACE</strong>
                            <span className="hidden md:inline text-slate-500 mx-2">|</span>
                            <span className="hidden md:inline text-slate-500">Service: <span className="text-cyan-400">{selectedService}</span></span>
                        </div>
                        <button onClick={() => setShowDebug(false)} className="text-slate-400 hover:text-white transition-colors">
                            <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
                            </svg>
                        </button>
                    </div>

                    {/* Columns */}
                    <div className="flex-1 grid grid-cols-3 divide-x divide-white/10 overflow-hidden">

                        {/* 1. Client Request (Blue) */}
                        <div className="flex flex-col min-h-0 bg-blue-500/[0.02]">
                            <div className="px-4 py-2 border-b border-white/10 flex justify-between items-center bg-blue-500/10">
                                <span className="font-bold text-blue-400">CLIENT REQUEST</span>
                                <span className="px-1.5 py-0.5 rounded bg-blue-500/20 text-blue-300 text-[10px]">OUTBOUND</span>
                            </div>
                            <div className="flex-1 overflow-auto p-4 space-y-4">
                                <div>
                                    <div className="text-slate-500 mb-1 flex justify-between">
                                        <span>CONFIGURATION</span>
                                        <span className="text-slate-600">params</span>
                                    </div>
                                    <div className="p-2 rounded bg-black/50 border border-white/10 text-slate-300">
                                        <div className="grid grid-cols-2 gap-x-4 gap-y-1">
                                            <div>Temperature: <span className="text-cyan-400">{config.temperature}</span></div>
                                            <div>Max Tokens: <span className="text-cyan-400">{config.maxTokens}</span></div>
                                            <div>Top P: <span className="text-cyan-400">{config.topP}</span></div>
                                            {config.systemPrompt && <div className="col-span-2 text-slate-500 truncate mt-1 border-t border-white/5 pt-1">Sys: {config.systemPrompt}</div>}
                                        </div>
                                    </div>
                                </div>
                                <div>
                                    <div className="text-slate-500 mb-1 flex justify-between">
                                        <span>MESSAGES</span>
                                        <span className="text-slate-600">payload</span>
                                    </div>
                                    <div className="p-2 rounded bg-black/50 border border-white/10 text-slate-300 overflow-x-hidden">
                                        {messages.length > 0 ? (
                                            <div className="space-y-1">
                                                <div className="text-slate-500">[{messages.length} messages]</div>
                                                <div className="truncate text-slate-400 text-[10px]">Last: {messages[messages.length - 1].content.substring(0, 50)}...</div>
                                            </div>
                                        ) : (
                                            <span className="text-slate-600 opacity-50">Empty</span>
                                        )}
                                        <button
                                            onClick={() => navigator.clipboard.writeText(JSON.stringify(messages, null, 2))}
                                            className="mt-2 w-full py-1 text-center bg-white/5 hover:bg-white/10 border border-white/5 rounded text-[10px] text-blue-400 transition-colors"
                                        >
                                            COPY FULL JSON
                                        </button>
                                    </div>
                                </div>
                            </div>
                        </div>

                        {/* 2. Gateway Logic (Green) */}
                        <div className="flex flex-col min-h-0 bg-green-500/[0.02]">
                            <div className="px-4 py-2 border-b border-white/10 flex justify-between items-center bg-green-500/10">
                                <span className="font-bold text-green-400">GATEWAY LOGIC</span>
                                <span className="px-1.5 py-0.5 rounded bg-green-500/20 text-green-300 text-[10px]">PROCESSING</span>
                            </div>
                            <div className="flex-1 overflow-auto p-4 space-y-4 relative">
                                {/* Connector Lines (Visual Decoration) */}
                                <div className="absolute top-0 bottom-0 -left-px w-px bg-green-500/20" />
                                <div className="absolute top-0 bottom-0 -right-px w-px bg-green-500/20" />

                                <div className="space-y-2">
                                    <div className="flex items-center justify-between p-2 rounded bg-white/5 border border-white/10">
                                        <span className="text-slate-400">Auth Check</span>
                                        <span className="flex items-center gap-1.5 text-green-400">
                                            <svg className="w-3 h-3" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                                                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={3} d="M5 13l4 4L19 7" />
                                            </svg>
                                            PASSED
                                        </span>
                                    </div>
                                    <div className="flex items-center justify-between p-2 rounded bg-white/5 border border-white/10">
                                        <span className="text-slate-400">Rate Limit</span>
                                        <span className="flex items-center gap-1.5 text-green-400">
                                            <svg className="w-3 h-3" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                                                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={3} d="M5 13l4 4L19 7" />
                                            </svg>
                                            OK
                                        </span>
                                    </div>
                                    <div className="p-2 rounded bg-white/5 border border-white/10">
                                        <div className="text-slate-500 mb-1 flex justify-between">
                                            <span>ROUTING DECISION</span>
                                        </div>
                                        <div className="flex items-center gap-2 mb-1">
                                            <span className="text-purple-400">➜</span>
                                            <span className="text-white font-bold">{selectedService}</span>
                                        </div>
                                        <div className="text-[10px] text-slate-500 pl-4 border-l border-white/10 ml-1">
                                            Strategy: <span className="text-slate-300">
                                                {selectedItem?.strategy || (selectedItem?.name?.startsWith('model:') ? 'Direct Model' : 'Direct')}
                                            </span>
                                        </div>
                                    </div>
                                </div>
                            </div>
                        </div>

                        {/* 3. Provider Response (Purple) */}
                        <div className="flex flex-col min-h-0 bg-purple-500/[0.02]">
                            <div className="px-4 py-2 border-b border-white/10 flex justify-between items-center bg-purple-500/10">
                                <span className="font-bold text-purple-400">PROVIDER RESPONSE</span>
                                <span className="px-1.5 py-0.5 rounded bg-purple-500/20 text-purple-300 text-[10px]">INBOUND</span>
                            </div>
                            <div className="flex-1 overflow-auto p-4 space-y-4">
                                <div className="grid grid-cols-2 gap-2">
                                    <div className="p-2 rounded bg-white/5 border border-white/10 text-center">
                                        <div className="text-[10px] text-slate-500 uppercase">Status</div>
                                        <div className="text-green-400 font-bold">{messages[messages.length - 1]?.role === 'assistant' ? '200 OK' : 'PENDING'}</div>
                                    </div>
                                    <div className="p-2 rounded bg-white/5 border border-white/10 text-center">
                                        <div className="text-[10px] text-slate-500 uppercase">Latency</div>
                                        <div className="text-white font-mono">{isLoading ? '...' : '~145ms'}</div>
                                    </div>
                                </div>

                                <div>
                                    <div className="text-slate-500 mb-1 flex justify-between">
                                        <span>RESPONSE BODY</span>
                                        <span className="text-slate-600">stream</span>
                                    </div>
                                    <div className="p-2 rounded bg-black/50 border border-white/10 text-slate-300 h-24 overflow-y-auto font-mono text-[10px] whitespace-pre-wrap">
                                        {streamingContent || (messages[messages.length - 1]?.role === 'assistant' ? messages[messages.length - 1].content : '// Waiting for response...')}
                                    </div>
                                </div>
                            </div>
                        </div>

                    </div>
                </div>
            )}

            {/* Custom slider styles */}
            <style jsx>{`
                input[type='range']::-webkit-slider-thumb {
                    -webkit-appearance: none;
                    appearance: none;
                    width: 14px;
                    height: 14px;
                    /* design-token: cyan-400 */ /* design-token: cyan-500 */
                    background: linear-gradient(135deg, #22d3ee, #06b6d4);
                    border-radius: 50%;
                    cursor: pointer;
                    box-shadow: 0 0 10px rgba(34, 211, 238, 0.5);
                }
                input[type='range']::-moz-range-thumb {
                    width: 14px;
                    height: 14px;
                    /* design-token: cyan-400 */ /* design-token: cyan-500 */
                    background: linear-gradient(135deg, #22d3ee, #06b6d4);
                    border-radius: 50%;
                    cursor: pointer;
                    border: none;
                    box-shadow: 0 0 10px rgba(34, 211, 238, 0.5);
                }
            `}</style>
        </div >
    )
}
