// API base URL.
//
// In the single-image deployment the Rust gateway serves both this
// SPA and the /v1/* API on the same origin, so the default is empty
// → fetch('/v1/...') stays on whichever host the user typed in
// the browser. Set NEXT_PUBLIC_API_URL only if you're deploying the
// frontend separately from the gateway (split-deployment legacy).
export const API_URL = process.env.NEXT_PUBLIC_API_URL ?? ''

export function api(path: string): string {
    return `${API_URL}${path}`
}
