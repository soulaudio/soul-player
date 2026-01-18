import { build } from 'vite'

try {
  console.log('Starting build...')
  const result = await build({
    configFile: './vite.config.ts',
    logLevel: 'info'
  })
  console.log('Build result:', result)
  console.log('Build completed successfully')
} catch (error) {
  console.error('Build error:', error)
  process.exit(1)
}
