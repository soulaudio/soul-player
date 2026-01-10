/** @type {import('next').NextConfig} */
const nextConfig = {
  output: 'export',
  reactStrictMode: true,
  transpilePackages: ['@soul-player/shared'],
  // Configure base path for GitHub Pages if deploying to repo subdirectory
  // basePath: process.env.NODE_ENV === 'production' ? '/soul-player' : '',
  images: {
    unoptimized: true,
  },

  // Turbopack configuration (Next.js 16 default)
  turbopack: {
    // Replace Node.js modules with empty module for browser builds
    resolveAlias: {
      fs: { browser: './empty.ts' },
    },
  },

  // Webpack configuration (fallback if using --webpack flag)
  webpack: (config, { isServer }) => {
    if (!isServer) {
      config.resolve.fallback = {
        ...config.resolve.fallback,
        fs: false,
      }
    }
    return config
  }
}

export default nextConfig
