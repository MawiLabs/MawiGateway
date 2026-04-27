'use client'

import { useMemo, useState } from 'react'
import { Search, Check, CornerDownLeft, Plus } from 'lucide-react'
import {
    PROVIDER_CATALOG,
    PROVIDER_BY_ID,
    CATEGORY_LABELS,
    type ProviderCategory,
    type ProviderInfo,
} from '@/lib/providerCatalog'

interface ProviderPickerProps {
    value: string
    onChange: (providerId: string) => void
    /**
     * IDs of providers to surface in the "Recent" hero row at the top.
     * Empty array hides the section entirely. Order is preserved.
     */
    recentIds?: string[]
}

/**
 * Searchable card-grid provider picker.
 *
 * Designed to scale from 4 providers to a few hundred without changing
 * shape:
 *   - Hero cards at the top for the user's actually-used providers
 *     (driven from request_logs upstream of this component)
 *   - Tag chips per category for fast browse-by-type
 *   - 3-column scrollable card grid for everything else
 *   - Search input filters all of the above
 *
 * The selected card gets a strong cyan glow + checkmark badge so the
 * picker doubles as the "currently chosen" display — no separate chip
 * needed elsewhere in the form.
 */
export function ProviderPicker({ value, onChange, recentIds = [] }: ProviderPickerProps) {
    const [search, setSearch] = useState('')
    const [category, setCategory] = useState<ProviderCategory | 'all'>('all')

    // Resolve recent IDs into actual provider objects, dropping any that
    // don't exist in the catalog (defensive — the upstream might pass an
    // ID for a provider that was renamed or deprecated).
    const recents = useMemo(
        () =>
            recentIds
                .map((id) => PROVIDER_BY_ID.get(id))
                .filter((p): p is ProviderInfo => Boolean(p)),
        [recentIds],
    )
    const recentIdSet = useMemo(() => new Set(recents.map((r) => r.id)), [recents])

    // Filter the catalog by search + category. We never filter recents
    // by category — the user wants to see "what I just used" regardless
    // of the chip they clicked.
    const filtered = useMemo(() => {
        const q = search.trim().toLowerCase()
        return PROVIDER_CATALOG.filter((p) => {
            if (category !== 'all' && p.category !== category) return false
            if (!q) return true
            return (
                p.name.toLowerCase().includes(q) ||
                p.summary.toLowerCase().includes(q) ||
                p.id.toLowerCase().includes(q)
            )
        }).filter((p) => !recentIdSet.has(p.id))
    }, [search, category, recentIdSet])

    // Counts for the tag chips. Updates as search changes so chips
    // accurately reflect "how many results in each category right now".
    const counts = useMemo(() => {
        const q = search.trim().toLowerCase()
        const matchingSearch = (p: ProviderInfo) =>
            !q ||
            p.name.toLowerCase().includes(q) ||
            p.summary.toLowerCase().includes(q) ||
            p.id.toLowerCase().includes(q)
        const map: Record<string, number> = { all: 0 }
        for (const p of PROVIDER_CATALOG) {
            if (!matchingSearch(p)) continue
            map.all = (map.all || 0) + 1
            map[p.category] = (map[p.category] || 0) + 1
        }
        return map
    }, [search])

    return (
        <div className="rounded-xl border border-white/10 bg-gradient-to-b from-[#0a0a0a] to-[#050505] overflow-hidden">
            {/* Search bar */}
            <div className="flex items-center gap-2.5 px-4 py-3 border-b border-white/10 bg-white/[0.015]">
                <Search className="w-4 h-4 text-cyan-400 flex-shrink-0" strokeWidth={2} />
                <input
                    type="text"
                    value={search}
                    onChange={(e) => setSearch(e.target.value)}
                    placeholder={`Search ${PROVIDER_CATALOG.length} providers by name or capability...`}
                    className="flex-1 bg-transparent border-none outline-none text-white text-sm placeholder-slate-500"
                />
                <span className="font-mono text-[10px] font-semibold px-1.5 py-0.5 rounded bg-white/5 border border-white/15 text-slate-400">
                    ⌘K
                </span>
            </div>

            {/* Tag chips */}
            <div className="flex gap-1.5 px-4 py-2.5 border-b border-white/10 overflow-x-auto scrollbar-none">
                <CategoryChip
                    active={category === 'all'}
                    onClick={() => setCategory('all')}
                    label="All"
                    count={counts.all || 0}
                />
                {(Object.keys(CATEGORY_LABELS) as ProviderCategory[]).map((cat) => (
                    <CategoryChip
                        key={cat}
                        active={category === cat}
                        onClick={() => setCategory(cat)}
                        label={CATEGORY_LABELS[cat]}
                        count={counts[cat] || 0}
                        category={cat}
                    />
                ))}
            </div>

            {/* Recent hero row */}
            {recents.length > 0 && category === 'all' && !search && (
                <>
                    <div className="px-4 pt-3 pb-2 flex items-center gap-3">
                        <span className="text-[10px] font-bold text-slate-600 uppercase tracking-wider">
                            Recent
                        </span>
                        <div className="flex-1 h-px bg-gradient-to-r from-white/10 to-transparent" />
                        <span className="text-[10px] text-slate-600">
                            used in last 30 days
                        </span>
                    </div>
                    <div className="grid grid-cols-2 gap-2.5 px-4">
                        {recents.map((p) => (
                            <HeroCard
                                key={p.id}
                                provider={p}
                                selected={value === p.id}
                                onSelect={() => onChange(p.id)}
                            />
                        ))}
                    </div>
                </>
            )}

            {/* All providers grid */}
            <div className="px-4 pt-3 pb-2 flex items-center gap-3">
                <span className="text-[10px] font-bold text-slate-600 uppercase tracking-wider">
                    {category === 'all' ? 'All providers' : CATEGORY_LABELS[category]}
                </span>
                <div className="flex-1 h-px bg-gradient-to-r from-white/10 to-transparent" />
                <span className="text-[10px] text-slate-600">
                    {filtered.length} {filtered.length === 1 ? 'result' : 'results'}
                </span>
            </div>
            <div className="grid grid-cols-3 gap-2 px-4 pb-3 max-h-[280px] overflow-y-auto">
                {filtered.length === 0 ? (
                    <div className="col-span-3 py-8 text-center text-sm text-slate-500">
                        No providers match. Try a different search or
                        <span className="text-cyan-400 ml-1">request a new one</span>.
                    </div>
                ) : (
                    filtered.map((p) => (
                        <GridCard
                            key={p.id}
                            provider={p}
                            selected={value === p.id}
                            onSelect={() => onChange(p.id)}
                        />
                    ))
                )}
            </div>

            {/* Footer */}
            <div className="flex items-center justify-between px-4 py-2.5 border-t border-white/10 bg-black/40 text-[11px] text-slate-500">
                <div className="flex gap-3">
                    <span className="inline-flex items-center gap-1">
                        <Kbd>↑↓←→</Kbd> navigate
                    </span>
                    <span className="inline-flex items-center gap-1">
                        <Kbd>
                            <CornerDownLeft className="w-2.5 h-2.5" strokeWidth={2.5} />
                        </Kbd>
                        select
                    </span>
                </div>
                <button
                    type="button"
                    className="inline-flex items-center gap-1 text-cyan-400 hover:text-cyan-300 font-semibold transition-colors"
                >
                    <Plus className="w-3 h-3" strokeWidth={2.5} />
                    Request a provider
                </button>
            </div>
        </div>
    )
}

function CategoryChip({
    active,
    onClick,
    label,
    count,
    category,
}: {
    active: boolean
    onClick: () => void
    label: string
    count: number
    category?: ProviderCategory
}) {
    const activeStyle =
        'bg-cyan-400/10 text-cyan-300 border-cyan-400/40 shadow-[0_0_12px_rgba(34,211,238,0.2)]'
    const idleStyle =
        'bg-white/[0.04] text-slate-400 border-white/10 hover:bg-white/[0.07] hover:text-slate-200'
    return (
        <button
            type="button"
            onClick={onClick}
            className={`flex-shrink-0 inline-flex items-center gap-1.5 text-[11px] font-semibold px-2.5 py-1 rounded-full border transition-all ${
                active ? activeStyle : idleStyle
            }`}
        >
            {label}
            <span className={active ? 'text-cyan-400/80' : 'text-slate-600'}>
                {count}
            </span>
        </button>
    )
}

function HeroCard({
    provider,
    selected,
    onSelect,
}: {
    provider: ProviderInfo
    selected: boolean
    onSelect: () => void
}) {
    return (
        <button
            type="button"
            onClick={onSelect}
            aria-pressed={selected}
            className={`group relative text-left p-3.5 rounded-xl overflow-hidden transition-all
        bg-gradient-to-br from-[#0f0f0f] to-[#080808]
        ${
            selected
                ? 'border-cyan-400 ring-1 ring-cyan-400/50 shadow-[0_0_32px_rgba(34,211,238,0.3)] bg-gradient-to-br from-cyan-400/[0.08] to-[#080808]'
                : 'border-white/15 border hover:border-cyan-400/50 hover:-translate-y-0.5 hover:shadow-[0_8px_24px_rgba(0,0,0,0.4),0_0_32px_rgba(34,211,238,0.18)]'
        }`}
        >
            {/* Subtle radial glow corner */}
            <div
                aria-hidden
                className="pointer-events-none absolute inset-0 opacity-60"
                style={{
                    background:
                        'radial-gradient(120px 80px at 80% -10%, rgba(34,211,238,0.15), transparent 70%)',
                }}
            />
            <div className="relative flex items-center gap-3">
                <div className="w-9 h-9 flex-shrink-0 rounded-lg bg-white/[0.03] border border-white/10 flex items-center justify-center text-white">
                    <div className="w-5 h-5">{provider.logo}</div>
                </div>
                <div className="flex-1 min-w-0">
                    <div className="text-sm font-semibold text-white">{provider.name}</div>
                    <div className="text-[10px] font-semibold text-cyan-300/80 uppercase tracking-wider mt-0.5 truncate">
                        {CATEGORY_LABELS[provider.category]} · {provider.summary}
                    </div>
                </div>
                {selected && (
                    <div className="flex-shrink-0 w-5 h-5 rounded-full bg-cyan-400 text-black flex items-center justify-center shadow-[0_0_12px_rgba(34,211,238,0.6)]">
                        <Check className="w-3 h-3" strokeWidth={3} />
                    </div>
                )}
            </div>
        </button>
    )
}

function GridCard({
    provider,
    selected,
    onSelect,
}: {
    provider: ProviderInfo
    selected: boolean
    onSelect: () => void
}) {
    const tagClass: Record<ProviderCategory, string> = {
        foundation: 'bg-cyan-400/10 text-cyan-300 border-cyan-400/20',
        hosted: 'bg-violet-400/10 text-violet-300 border-violet-400/20',
        selfhosted: 'bg-emerald-400/10 text-emerald-300 border-emerald-400/20',
        audio: 'bg-pink-400/10 text-pink-300 border-pink-400/20',
        image: 'bg-amber-400/10 text-amber-200 border-amber-400/20',
        aggregator: 'bg-slate-400/10 text-slate-300 border-slate-400/15',
    }
    return (
        <button
            type="button"
            onClick={onSelect}
            aria-pressed={selected}
            className={`relative group flex flex-col items-center gap-2 p-3 rounded-xl border transition-all text-center
        ${
            selected
                ? 'border-cyan-400 bg-gradient-to-br from-cyan-400/[0.06] to-[#0a0a0a] ring-1 ring-cyan-400/40 shadow-[0_0_24px_rgba(34,211,238,0.2)]'
                : 'border-white/10 bg-[#0a0a0a] hover:bg-[#0f0f0f] hover:border-cyan-400/40 hover:-translate-y-0.5 hover:shadow-[0_0_20px_rgba(34,211,238,0.12)]'
        }`}
        >
            <div className="w-8 h-8 flex items-center justify-center text-white">
                <div className="w-6 h-6">{provider.logo}</div>
            </div>
            <div className="text-xs font-semibold text-slate-200 leading-tight">{provider.name}</div>
            <div
                className={`text-[9px] font-semibold uppercase tracking-wider px-1.5 py-0.5 rounded-full border ${tagClass[provider.category]}`}
            >
                {CATEGORY_LABELS[provider.category]}
            </div>
            {selected && (
                <div className="absolute top-1.5 right-1.5 w-4 h-4 rounded-full bg-cyan-400 text-black flex items-center justify-center shadow-[0_0_8px_rgba(34,211,238,0.5)]">
                    <Check className="w-2.5 h-2.5" strokeWidth={3} />
                </div>
            )}
        </button>
    )
}

function Kbd({ children }: { children: React.ReactNode }) {
    return (
        <span className="font-mono text-[10px] font-semibold px-1 py-0.5 rounded bg-white/5 border border-white/10 text-slate-300 inline-flex items-center">
            {children}
        </span>
    )
}
