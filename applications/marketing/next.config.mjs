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
  turbopack: {},
  webpack: (config) => {
    config.resolve.fallback = {
      ...config.resolve.fallback,
      fs: false,
    }
    return config
  }
}

export default nextConfig
