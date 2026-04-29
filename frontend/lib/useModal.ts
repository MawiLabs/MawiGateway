'use client'

import { RefObject, useEffect, useRef } from 'react'

/**
 * useModal — accessibility primitives every modal in mawi-web needs.
 *
 * Wires three things every Notion / Linear / Stripe modal does and ours
 * didn't:
 *
 *   1. ESC key closes the modal.
 *   2. Tab and Shift+Tab cycle focus inside the modal (focus trap),
 *      so keyboard users don't fall back to the page underneath.
 *   3. The first focusable element auto-receives focus when the modal
 *      opens, so the user can start typing immediately.
 *
 * Usage:
 *
 *   const ref = useRef<HTMLDivElement>(null)
 *   useModal({ isOpen, onClose, ref })
 *   return <div ref={ref} role="dialog" aria-modal="true">...</div>
 *
 * The hook is safe to call when `isOpen` is false (it cleans up its own
 * listeners and does nothing). Calling it on a non-mounted ref is a no-op.
 */
export interface UseModalArgs {
    isOpen: boolean
    onClose: () => void
    ref: RefObject<HTMLElement>
    /**
     * If true (default), focus the first focusable element inside the
     * modal when it opens. Set false for modals that should land on
     * the close button or a confirmation, not on a text input.
     */
    autoFocus?: boolean
}

const FOCUSABLE_SELECTOR = [
    'a[href]',
    'button:not([disabled])',
    'textarea:not([disabled])',
    'input:not([disabled]):not([type="hidden"])',
    'select:not([disabled])',
    '[tabindex]:not([tabindex="-1"])',
].join(',')

export function useModal({ isOpen, onClose, ref, autoFocus = true }: UseModalArgs) {
    // Track the element that was focused before the modal opened so we
    // can restore focus when it closes. Better than letting focus snap
    // back to the body — the user keeps their place in the page.
    const previouslyFocusedRef = useRef<HTMLElement | null>(null)

    useEffect(() => {
        if (!isOpen) return

        previouslyFocusedRef.current =
            (document.activeElement as HTMLElement | null) ?? null

        // Auto-focus the first focusable element inside the modal.
        if (autoFocus) {
            const root = ref.current
            if (root) {
                // Skip the close button (typically the first focusable in
                // a modal header) and prefer the first input — that's
                // what the user actually wants to type into.
                const focusables = Array.from(
                    root.querySelectorAll<HTMLElement>(FOCUSABLE_SELECTOR),
                )
                const firstInput = focusables.find(
                    (el) => el.tagName === 'INPUT' || el.tagName === 'TEXTAREA',
                )
                ;(firstInput ?? focusables[0])?.focus()
            }
        }

        const handleKey = (e: KeyboardEvent) => {
            if (e.key === 'Escape') {
                e.preventDefault()
                onClose()
                return
            }
            if (e.key !== 'Tab') return

            // Focus trap. Wrap from last → first on Tab and first → last
            // on Shift+Tab so focus never escapes the modal.
            const root = ref.current
            if (!root) return
            const focusables = Array.from(
                root.querySelectorAll<HTMLElement>(FOCUSABLE_SELECTOR),
            ).filter((el) => !el.hasAttribute('aria-hidden'))
            if (focusables.length === 0) return

            const first = focusables[0]
            const last = focusables[focusables.length - 1]
            const active = document.activeElement as HTMLElement | null

            if (e.shiftKey && active === first) {
                e.preventDefault()
                last.focus()
            } else if (!e.shiftKey && active === last) {
                e.preventDefault()
                first.focus()
            }
        }

        document.addEventListener('keydown', handleKey)
        return () => {
            document.removeEventListener('keydown', handleKey)
            // Restore focus to whatever was focused before the modal
            // opened — preserves keyboard-user place in the page.
            previouslyFocusedRef.current?.focus?.()
        }
    }, [isOpen, onClose, ref, autoFocus])
}
