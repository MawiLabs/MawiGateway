// API configuration - direct calls to backend
export const API_URL = process.env.NEXT_PUBLIC_API_URL || 'http://127.0.0.1:8030'

export function api(path: string): string {
    return `${API_URL}${path}`
}
