// Test script to verify Vite multi-entry output structure
import { execSync } from 'child_process';
import { readdirSync, statSync } from 'fs';
import { join } from 'path';

console.log('Building with Vite...');
try {
  // Run the build from applications/desktop
  execSync('cd applications/desktop && yarn build', { stdio: 'inherit', shell: true });

  console.log('\n=== Checking dist folder structure ===');
  const distPath = 'applications/desktop/dist';

  function listFiles(dir, prefix = '') {
    const items = readdirSync(dir);
    for (const item of items) {
      const fullPath = join(dir, item);
      const stat = statSync(fullPath);
      if (stat.isDirectory()) {
        console.log(`${prefix}📁 ${item}/`);
        listFiles(fullPath, prefix + '  ');
      } else {
        console.log(`${prefix}📄 ${item} (${stat.size} bytes)`);
      }
    }
  }

  listFiles(distPath);

} catch (error) {
  console.error('Build failed:', error.message);
  process.exit(1);
}
