import { NextResponse } from 'next/server'
import type { NextRequest } from 'next/server'

// NOTE: Middleware temporarily disabled to prevent redirect loops
// Authentication is handled by AuthContext on the client side
export function middleware(request: NextRequest) {
    // Allow all requests - AuthContext handles authentication
    return NextResponse.next()
}

export const config = {
    matcher: [
        /*
         * Match all request paths except for the ones starting with:
         * - _next/static (static files)
         * - _next/image (image optimization files)
         * - favicon.ico (favicon file)
         * - public (public files)
         */
        '/((?!_next/static|_next/image|favicon.ico|public).*)',
    ],
}
