'use client'

import { useState, useRef, useEffect, useCallback } from 'react'
import { motion, AnimatePresence } from 'framer-motion'
import { createPortal } from 'react-dom'

interface Mention {
    id: string
    label: string
    type: 'model' | 'tool'
    icon?: string
    logo?: string
    color?: string
}

interface RichPromptEditorProps {
    value: string
    onChange: (value: string) => void
    mentions: Mention[]
    placeholder?: string
    minHeight?: string
    className?: string
    paddingTop?: string | number
}

export function RichPromptEditor({ value, onChange, mentions, placeholder, minHeight = '120px', className = '', paddingTop = '12px' }: RichPromptEditorProps) {
    const editorRef = useRef<HTMLDivElement>(null)
    const [showMentions, setShowMentions] = useState(false)
    const [mentionFilter, setMentionFilter] = useState('')
    const [cursorPosition, setCursorPosition] = useState({ top: 0, left: 0 })
    const [activeIndex, setActiveIndex] = useState(0)
    const [isEmpty, setIsEmpty] = useState(!value)

    const filteredMentions = mentions.filter(m =>
        m.label.toLowerCase().includes(mentionFilter.toLowerCase()) ||
        m.id.toLowerCase().includes(mentionFilter.toLowerCase())
    )

    useEffect(() => {
        setIsEmpty(!value)
    }, [value])

    // Initial render: Convert Text -> HTML
    useEffect(() => {
        if (editorRef.current && editorRef.current.innerText !== value) {
            // Only update if significantly different to avoid cursor jumping
            // Basic parsing: replace [Type:Label] with chips
            // Regex: \[((?:Model|Tool)):(.*?)\]
            const html = value
                .replace(/\n/g, '<br>')
                .replace(/\[((?:Model|Tool)):(.*?)\]/g, (match, type, label) => {
                    const mention = mentions.find(m => m.label === label)
                    const icon = mention?.icon || (type === 'Model' ? '🤖' : '🛠️')
                    const logo = mention?.logo
                    const color = mention?.color || (type === 'Model' ? 'cyan' : 'purple')
                    const bgClass = type === 'Model' ? 'bg-cyan-400/20 text-cyan-400 border-cyan-400/30' : 'bg-purple-400/20 text-purple-400 border-purple-400/30'

                    const iconContent = logo
                        ? `<img src="${logo}" class="w-3.5 h-3.5 object-contain rounded-sm" />`
                        : `<span class="text-sm">${icon}</span>`

                    return `<span contenteditable="false" data-mention-type="${type}" data-mention-label="${label}" class="inline-flex items-center gap-1 px-1.5 py-0.5 mx-1 rounded-md text-xs font-medium border ${bgClass} select-none align-middle">${iconContent}${label}</span>`
                })

            if (editorRef.current.innerHTML !== html) {
                editorRef.current.innerHTML = html
            }
            // Update empty state from DOM to be sure
            setIsEmpty(editorRef.current.textContent?.length === 0)
        }
    }, [])

    const handleInput = () => {
        if (!editorRef.current) return

        setIsEmpty(editorRef.current.textContent?.length === 0)

        // Convert HTML -> Text
        // We traverse nodes. Text nodes are text. Element nodes with data-mention are [Type:Label]. <br> is \n.
        let text = ''
        const traverse = (node: Node) => {
            if (node.nodeType === Node.TEXT_NODE) {
                text += node.textContent
            } else if (node.nodeType === Node.ELEMENT_NODE) {
                const el = node as HTMLElement
                if (el.dataset.mentionType) {
                    text += `[${el.dataset.mentionType}:${el.dataset.mentionLabel}]`
                } else if (el.tagName === 'BR') {
                    text += '\n'
                } else if (el.tagName === 'DIV') {
                    text += '\n' // divs often imply newlines in contenteditable
                    el.childNodes.forEach(traverse)
                } else {
                    el.childNodes.forEach(traverse)
                }
            }
        }

        editorRef.current.childNodes.forEach(traverse)

        // Detect @ mention trigger
        const selection = window.getSelection()
        if (selection && selection.rangeCount > 0) {
            const range = selection.getRangeAt(0)
            const textNode = range.startContainer
            if (textNode.nodeType === Node.TEXT_NODE) {
                const textContent = textNode.textContent || ''
                const beforeCursor = textContent.slice(0, range.startOffset)
                const lastAt = beforeCursor.lastIndexOf('@')

                if (lastAt !== -1) {
                    const query = beforeCursor.slice(lastAt + 1)
                    // Check if there are spaces, implying we are not mentioning anymore (unless we allow spaces in query)
                    if (!query.includes(' ')) {
                        const rect = range.getBoundingClientRect()
                        setCursorPosition({ top: rect.bottom + window.scrollY, left: rect.left + window.scrollX })
                        setMentionFilter(query)
                        setShowMentions(true)
                        setActiveIndex(0)
                        return
                    }
                }
            }
        }

        setShowMentions(false)
        onChange(text)
    }

    const insertMention = (mention: Mention) => {
        if (!editorRef.current) return

        const selection = window.getSelection()
        if (!selection || selection.rangeCount === 0) return

        const range = selection.getRangeAt(0)

        // Find the @ text node and replace it
        const textNode = range.startContainer
        if (textNode.nodeType === Node.TEXT_NODE) {
            const textContent = textNode.textContent || ''
            const beforeCursor = textContent.slice(0, range.startOffset)
            const lastAt = beforeCursor.lastIndexOf('@')

            if (lastAt !== -1) {
                // Remove the @query part
                const afterCursor = textContent.slice(range.startOffset)
                const newTextBefore = beforeCursor.slice(0, lastAt)
                textNode.textContent = newTextBefore + afterCursor

                // Set cursor to insertion point
                range.setStart(textNode, lastAt)
                range.collapse(true)
            }
        }

        const type = mention.type === 'model' ? 'Model' : 'Tool'
        const icon = mention.icon || (type === 'Model' ? '🤖' : '🛠️')
        const bgClass = type === 'Model' ? 'bg-cyan-400/20 text-cyan-400 border-cyan-400/30' : 'bg-purple-400/20 text-purple-400 border-purple-400/30'

        const span = document.createElement('span')
        span.contentEditable = 'false'
        span.className = `inline-flex items-center gap-1 px-1.5 py-0.5 mx-1 rounded-md text-xs font-medium border ${bgClass} select-none align-middle`
        span.dataset.mentionType = type
        span.dataset.mentionLabel = mention.label

        const iconContent = mention.logo
            ? `<img src="${mention.logo}" class="w-3.5 h-3.5 object-contain rounded-sm" />`
            : `<span class="text-sm">${icon}</span>`

        span.innerHTML = `${iconContent}${mention.label}`

        range.insertNode(span)

        // Add a space after
        const space = document.createTextNode('\u00A0')
        range.setStartAfter(span)
        range.insertNode(space)
        range.setStartAfter(space)
        range.collapse(true)

        selection.removeAllRanges()
        selection.addRange(range)

        setShowMentions(false)
        handleInput() // Update state
    }

    const handleKeyDown = (e: React.KeyboardEvent) => {
        if (showMentions) {
            if (e.key === 'ArrowDown') {
                e.preventDefault()
                setActiveIndex(prev => (prev + 1) % filteredMentions.length)
            } else if (e.key === 'ArrowUp') {
                e.preventDefault()
                setActiveIndex(prev => (prev - 1 + filteredMentions.length) % filteredMentions.length)
            } else if (e.key === 'Enter' || e.key === 'Tab') {
                e.preventDefault()
                if (filteredMentions[activeIndex]) {
                    insertMention(filteredMentions[activeIndex])
                }
            } else if (e.key === 'Escape') {
                setShowMentions(false)
            }
        }
    }

    return (
        <div className="relative font-mono">
            <div
                ref={editorRef}
                contentEditable
                onInput={handleInput}
                onKeyDown={handleKeyDown}
                className={`w-full px-4 bg-black border border-white/10 rounded-xl text-white text-sm leading-relaxed focus:border-cyan-400 focus:ring-4 focus:ring-cyan-400/20 outline-none resize-y transition-all overflow-y-auto whitespace-pre-wrap ${className}`}
                style={{ minHeight, paddingTop, paddingBottom: '12px' }}
                data-placeholder={placeholder}
            />
            {isEmpty && (
                <div
                    className="absolute left-4 text-slate-700 pointer-events-none text-sm"
                    style={{ top: paddingTop }}
                >
                    {placeholder}
                </div>
            )}

            {/* Mention Dropdown */}
            {showMentions && filteredMentions.length > 0 && createPortal(
                <div
                    className="fixed z-[100] w-64 bg-[#0a0a0a] border border-white/20 rounded-xl shadow-2xl overflow-hidden backdrop-blur-xl"
                    style={{ top: cursorPosition.top + 8, left: cursorPosition.left }}
                >
                    <div className="px-3 py-2 border-b border-white/10 text-xs text-slate-400 font-semibold bg-white/5">
                        SUGGESTIONS
                    </div>
                    <div className="max-h-60 overflow-y-auto p-1">
                        {filteredMentions.map((mention, idx) => (
                            <button
                                key={mention.id}
                                onClick={() => insertMention(mention)}
                                className={`w-full flex items-center gap-3 px-3 py-2 text-sm rounded-lg transition-colors ${idx === activeIndex ? 'bg-cyan-400/20 text-white' : 'text-slate-300 hover:bg-white/5'
                                    }`}
                                onMouseEnter={() => setActiveIndex(idx)}
                            >
                                <div className={`w-6 h-6 rounded flex items-center justify-center ${mention.type === 'model' ? 'bg-cyan-900/50' : 'bg-purple-900/50'} overflow-hidden`}>
                                    {mention.logo ? (
                                        <img src={mention.logo} className="w-4 h-4 object-contain" alt="" />
                                    ) : (
                                        mention.icon || (mention.type === 'model' ? '🤖' : '🛠️')
                                    )}
                                </div>
                                <div className="flex-1 text-left">
                                    <div className="font-medium">{mention.label}</div>
                                    <div className="text-[10px] text-slate-500 uppercase">{mention.type}</div>
                                </div>
                            </button>
                        ))}
                    </div>
                </div>,
                document.body
            )}
        </div>
    )
}
