/** @type {import('next').NextConfig} */
//
// MawiGateway runs everything from a single binary now: the Rust
// gateway serves both /v1/* (API) AND / (this static SPA) on port
// 8030. So Next.js builds a static export that the gateway embeds —
// no Node runtime in production, no separate mawi-web container.
//
// Trade-offs we already took:
//   * `output: 'export'` ⇒ no SSR, no Image Optimization, no
//     route handlers (`app/api/*`). The UI is pure client-side React,
//     which it already was.
//   * No `rewrites` — the gateway IS the origin, so /v1/* paths hit
//     the same host and port as the UI. CORS, cookies, samesite all
//     simplify.
//   * No `middleware.ts` — Next export disallows it, and ours was a
//     no-op anyway.
const nextConfig = {
  output: 'export',
  images: { unoptimized: true },
  // Trailing slash makes the gateway's static-file router happier:
  // a directory request for /services becomes /services/ which then
  // serves /services/index.html.
  trailingSlash: true,
}

module.exports = nextConfig
