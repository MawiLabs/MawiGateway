/**
 * Provider catalog — the static list of upstream providers MawiGateway
 * supports, each with a category tag and a logo glyph (inline SVG path
 * data so we don't ship 24 separate icon files).
 *
 * Add a new provider by appending an entry. The picker UI reads this
 * directly; nothing else needs to change. The `id` field matches what
 * the gateway's `provider_type` column expects.
 */

import type { ReactElement } from 'react'

export type ProviderCategory =
    | 'foundation'
    | 'hosted'
    | 'selfhosted'
    | 'audio'
    | 'image'
    | 'aggregator'

export interface ProviderInfo {
    id: string
    name: string
    category: ProviderCategory
    /** One-line capability summary shown under the name in hero cards. */
    summary: string
    /** SVG path/group rendered at 24px in cards. Use stroke or fill on the group. */
    logo: ReactElement
}

/** Tag chip color tokens, mirrored in CSS classes inside the picker. */
export const CATEGORY_LABELS: Record<ProviderCategory, string> = {
    foundation: 'Foundation',
    hosted: 'Hosted',
    selfhosted: 'Self-hosted',
    audio: 'Audio',
    image: 'Image',
    aggregator: 'Aggregator',
}

// Inline SVGs are simple geometric marks — they're recognisable
// silhouettes, not pixel-perfect brand reproductions. Brand teams that
// want exact marks can swap individual entries; the data shape is the
// same. SVGs use `currentColor` so card hover/selected can tint them.
const svg = (paths: ReactElement) => paths

export const PROVIDER_CATALOG: ProviderInfo[] = [
    {
        id: 'openai',
        name: 'OpenAI',
        category: 'foundation',
        summary: 'GPT-4o, GPT-4 Turbo, o1',
        logo: svg(
            <svg viewBox="0 0 24 24" fill="currentColor" xmlns="http://www.w3.org/2000/svg">
                <path d="M22.282 9.821a5.985 5.985 0 0 0-.516-4.91 6.046 6.046 0 0 0-6.51-2.9A6.065 6.065 0 0 0 4.981 4.18a5.985 5.985 0 0 0-3.998 2.9 6.046 6.046 0 0 0 .743 7.097 5.98 5.98 0 0 0 .51 4.911 6.051 6.051 0 0 0 6.515 2.9A5.985 5.985 0 0 0 13.26 24a6.056 6.056 0 0 0 5.772-4.206 5.99 5.99 0 0 0 3.997-2.9 6.056 6.056 0 0 0-.747-7.073zM13.26 22.43a4.476 4.476 0 0 1-2.876-1.04l.141-.081 4.779-2.758a.795.795 0 0 0 .392-.681v-6.737l2.02 1.168a.071.071 0 0 1 .038.052v5.583a4.504 4.504 0 0 1-4.494 4.494z" />
            </svg>,
        ),
    },
    {
        id: 'anthropic',
        name: 'Anthropic',
        category: 'foundation',
        summary: 'Claude 3.5 Sonnet, Claude 3 Opus',
        logo: svg(
            <svg viewBox="0 0 24 24" fill="currentColor" xmlns="http://www.w3.org/2000/svg">
                <path d="M13.832 4h3.236L23.34 20h-3.235l-1.273-3.295h-6.762L10.797 20H7.563L13.832 4zm-9.664 0h3.265L13.7 20h-3.293l-1.236-3.266H2.518L1.282 20H-2L4.168 4zm10.864 2.795l-2.252 5.84h4.504l-2.252-5.84zm-9.696 0L2.985 13.91h4.526l-2.175-7.115z" />
            </svg>,
        ),
    },
    {
        id: 'google',
        name: 'Google AI',
        category: 'foundation',
        summary: 'Gemini 2.0 Flash, Gemini 1.5 Pro',
        logo: svg(
            <svg viewBox="0 0 24 24" fill="currentColor" xmlns="http://www.w3.org/2000/svg">
                <path d="M12 11v3.6h5.04c-.21 1.32-1.5 3.87-5.04 3.87-3.03 0-5.5-2.51-5.5-5.6S8.97 7.27 12 7.27c1.72 0 2.88.74 3.54 1.37l2.42-2.33C16.45 4.85 14.4 4 12 4 7.58 4 4 7.58 4 12s3.58 8 8 8c4.62 0 7.68-3.25 7.68-7.83 0-.53-.06-.94-.13-1.34H12z" />
            </svg>,
        ),
    },
    {
        id: 'azure',
        name: 'Azure OpenAI',
        category: 'hosted',
        summary: 'OpenAI models on Azure',
        logo: svg(
            <svg viewBox="0 0 24 24" fill="currentColor" xmlns="http://www.w3.org/2000/svg">
                <path d="M5.483 18.333L12.51 6.05l5.97 12.282-3.39.038L8.17 18.39l-2.687-.057zm-3.5-.183l4.62-7.91L11.6 4.99 5.84 18.087l-3.86.063z" />
            </svg>,
        ),
    },
    {
        id: 'mistral',
        name: 'Mistral',
        category: 'foundation',
        summary: 'Mistral Large, Mixtral',
        logo: svg(
            <svg viewBox="0 0 24 24" fill="currentColor" xmlns="http://www.w3.org/2000/svg">
                <path d="M22 7L12 13 2 7l10-5 10 5zM2 17l10 5 10-5M2 12l10 5 10-5" />
            </svg>,
        ),
    },
    {
        id: 'deepseek',
        name: 'DeepSeek',
        category: 'foundation',
        summary: 'DeepSeek V3, R1',
        logo: svg(
            <svg viewBox="0 0 24 24" fill="currentColor" xmlns="http://www.w3.org/2000/svg">
                <path d="M12 2L2 7v10l10 5 10-5V7L12 2zm0 2.18l7.27 3.64L12 11.45 4.73 7.82 12 4.18zM4 9.41l7 3.5v7.46l-7-3.5V9.41z" />
            </svg>,
        ),
    },
    {
        id: 'perplexity',
        name: 'Perplexity',
        category: 'foundation',
        summary: 'Sonar, online search',
        logo: svg(
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={2} xmlns="http://www.w3.org/2000/svg">
                <circle cx="11" cy="11" r="8" />
                <line x1="21" y1="21" x2="16.65" y2="16.65" />
            </svg>,
        ),
    },
    {
        id: 'xai',
        name: 'xAI',
        category: 'foundation',
        summary: 'Grok',
        logo: svg(
            <svg viewBox="0 0 24 24" fill="currentColor" xmlns="http://www.w3.org/2000/svg">
                <path d="M3 3h6v18H3zM15 3h6v18h-6z" />
            </svg>,
        ),
    },
    {
        id: 'cohere',
        name: 'Cohere',
        category: 'foundation',
        summary: 'Command R+',
        logo: svg(
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={2} xmlns="http://www.w3.org/2000/svg">
                <path d="M2 6h20M2 12h20M2 18h20" />
            </svg>,
        ),
    },
    {
        id: 'fireworks',
        name: 'Fireworks AI',
        category: 'hosted',
        summary: 'Open-weight models, fast',
        logo: svg(
            <svg viewBox="0 0 24 24" fill="currentColor" xmlns="http://www.w3.org/2000/svg">
                <path d="M12 2L2 22h20L12 2z" />
            </svg>,
        ),
    },
    {
        id: 'groq',
        name: 'Groq',
        category: 'hosted',
        summary: 'LPU inference',
        logo: svg(
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={2} xmlns="http://www.w3.org/2000/svg">
                <path d="M3 12h18M3 6h18M3 18h18" />
            </svg>,
        ),
    },
    {
        id: 'replicate',
        name: 'Replicate',
        category: 'hosted',
        summary: 'Open-source models on demand',
        logo: svg(
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={2} xmlns="http://www.w3.org/2000/svg">
                <polygon points="12 2 22 8.5 22 15.5 12 22 2 15.5 2 8.5 12 2" />
            </svg>,
        ),
    },
    {
        id: 'together',
        name: 'Together AI',
        category: 'hosted',
        summary: 'Hosted open models',
        logo: svg(
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={2} xmlns="http://www.w3.org/2000/svg">
                <path d="M3 12h6l3-9 3 18 3-9h3" />
            </svg>,
        ),
    },
    {
        id: 'ollama',
        name: 'Ollama',
        category: 'selfhosted',
        summary: 'Local models, zero cost',
        logo: svg(
            <svg viewBox="0 0 24 24" fill="currentColor" xmlns="http://www.w3.org/2000/svg">
                <circle cx="12" cy="12" r="10" />
                <circle cx="12" cy="12" r="4" fill="black" />
            </svg>,
        ),
    },
    {
        id: 'vllm',
        name: 'vLLM',
        category: 'selfhosted',
        summary: 'Self-hosted inference engine',
        logo: svg(
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={2} xmlns="http://www.w3.org/2000/svg">
                <path d="M2 12l5-9 5 9-5 9-5-9zM12 12l5-9 5 9-5 9-5-9z" />
            </svg>,
        ),
    },
    {
        id: 'elevenlabs',
        name: 'ElevenLabs',
        category: 'audio',
        summary: 'TTS, voice cloning',
        logo: svg(
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={2} xmlns="http://www.w3.org/2000/svg">
                <path d="M9 12l2 2 4-4" />
                <circle cx="12" cy="12" r="10" />
            </svg>,
        ),
    },
    {
        id: 'stability',
        name: 'Stability AI',
        category: 'image',
        summary: 'Stable Diffusion 3.5',
        logo: svg(
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={2} xmlns="http://www.w3.org/2000/svg">
                <circle cx="12" cy="12" r="10" />
                <polygon points="10 8 16 12 10 16" fill="currentColor" />
            </svg>,
        ),
    },
    {
        id: 'openrouter',
        name: 'OpenRouter',
        category: 'aggregator',
        summary: '100+ models via one key',
        logo: svg(
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={2} xmlns="http://www.w3.org/2000/svg">
                <path d="M3 6l3 12h12l3-12" />
                <circle cx="9" cy="18" r="2" />
                <circle cx="20" cy="18" r="2" />
            </svg>,
        ),
    },
]

export const PROVIDER_BY_ID = new Map(PROVIDER_CATALOG.map((p) => [p.id, p]))
