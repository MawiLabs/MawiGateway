'use client'

import { useState } from 'react'
import { motion, AnimatePresence } from 'framer-motion'

export interface AgenticStreamEvent {
    type: 'log' | 'chunk' | 'tool_call' | 'tool_result' | 'step' | 'reasoning_delta' | 'final_response'
    data: any
}

interface ThoughtTimelineProps {
    events: AgenticStreamEvent[]
    isFinished?: boolean
    isFinalAnswerStarted?: boolean
}

export function ThoughtTimeline({ events, isFinished, isFinalAnswerStarted }: ThoughtTimelineProps) {
    const [isCollapsed, setIsCollapsed] = useState(false)
    const [userHasToggled, setUserHasToggled] = useState(false)

    const shouldCollapse = userHasToggled ? isCollapsed : (isFinalAnswerStarted || isCollapsed)

    const handleToggle = () => {
        setUserHasToggled(true)
        setIsCollapsed(!shouldCollapse)
    }

    if (!events || events.length === 0) return null

    return (
        <div className="mb-4">
            {/* Professional header */}
            <button
                onClick={handleToggle}
                className="group flex items-center gap-2.5 px-3 py-2 rounded-lg bg-white/[0.03] hover:bg-white/[0.06] border border-white/[0.06] hover:border-white/10 transition-all duration-200 mb-3"
            >
                <span className="text-base">🧠</span>
                <span className="text-sm font-medium text-slate-300">Reasoning</span>
                <span className="text-xs text-slate-500 tabular-nums">{events.length} steps</span>
                {!isFinished && <span className="w-1.5 h-1.5 rounded-full bg-cyan-400 animate-pulse ml-1" />}
                <span className="text-slate-500 text-xs ml-auto group-hover:text-slate-400 transition-colors">
                    {shouldCollapse ? '▶' : '▼'}
                </span>
            </button>

            <AnimatePresence>
                {!shouldCollapse && (
                    <motion.div
                        initial={{ opacity: 0, height: 0 }}
                        animate={{ opacity: 1, height: 'auto' }}
                        exit={{ opacity: 0, height: 0 }}
                        transition={{ duration: 0.2 }}
                        className="overflow-hidden"
                    >
                        {/* Timeline with ladder */}
                        <div className="relative ml-4 pl-5">
                            {/* Vertical ladder line */}
                            <div className="absolute left-0 top-2 bottom-2 w-px bg-gradient-to-b from-cyan-500/50 via-cyan-500/20 to-transparent" />

                            <div className="space-y-1">
                                {events.map((event, index) => (
                                    <TimelineEvent key={index} event={event} index={index} isLast={index === events.length - 1} />
                                ))}
                            </div>
                        </div>
                    </motion.div>
                )}
            </AnimatePresence>
        </div>
    )
}

function TimelineEvent({ event, index, isLast }: { event: AgenticStreamEvent; index: number; isLast: boolean }) {
    const data = event.data
    let content = typeof data === 'string' ? data : data?.content || ''

    // Strip leading emoji if content already has one
    const emojiRegex = /^[\u{1F300}-\u{1F9FF}\u{2600}-\u{26FF}\u{2700}-\u{27BF}✅✨⚡️🚀📋🎯⚙️📦📍]\s*/u
    const cleanContent = content.replace(emojiRegex, '').trim()

    // Professional emojis
    let emoji = '📍'

    if (content.includes('TOOL[') || content.includes('Executing')) {
        emoji = '⚡️'
    } else if (content.includes('VERIFIED') || content.includes('✅')) {
        emoji = '✅'
    } else if (content.includes('Synthesizing') || content.includes('final')) {
        emoji = '✨'
    } else if (content.includes('plan') || content.includes('Generating') || content.includes('strategic')) {
        emoji = '📋'
    } else if (content.includes('constraint')) {
        emoji = '🎯'
    } else if (content.includes('activated')) {
        emoji = '🚀'
    } else if (content.includes('Initializing') || content.includes('planner')) {
        emoji = '⚙️'
    } else if (content.includes('config') || content.includes('Loaded')) {
        emoji = '📦'
    }

    // Use cleaned content to avoid double emoji
    const displayContent = cleanContent || content

    return (
        <motion.div
            initial={{ opacity: 0, x: -8 }}
            animate={{ opacity: 1, x: 0 }}
            transition={{ delay: index * 0.03 }}
            className="relative flex items-start gap-3 py-1.5"
        >
            {/* Dot on the ladder - centered on the line */}
            <div className="absolute -left-[23px] top-2.5 w-2 h-2 rounded-full bg-cyan-500/60 border border-cyan-400/30" />

            <span className="text-sm flex-shrink-0">{emoji}</span>
            <p className="text-sm text-slate-400 leading-relaxed">{displayContent}</p>
        </motion.div>
    )
}
