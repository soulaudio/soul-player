import { download } from 'edgedriver';

try {
  console.log('Downloading msedgedriver (latest stable version)...');
  // Download a specific stable version
  const driverPath = await download('131.0.2903.112');
  console.log(`msedgedriver downloaded to: ${driverPath}`);
} catch (error) {
  console.error('Failed to download msedgedriver:', error);
  process.exit(1);
}
