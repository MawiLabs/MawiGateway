/** @type {import('next').NextConfig} */
const nextConfig = {
  output: 'standalone',
  async rewrites() {
    const apiUrl = process.env.INTERNAL_API_URL || 'http://mawi-api:8030';
    // console.log('MAWI-WEB REWRITES CONFIGURATION');
    return [
      {
        source: '/v1/:path*',
        destination: `${apiUrl}/v1/:path*`,
      },
    ]
  },
}

module.exports = nextConfig
