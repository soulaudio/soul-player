import { MainLayout } from '../layouts/MainLayout';

/**
 * Demo view for marketing site showcase
 * Shows the actual Soul Player interface in a non-interactive state
 */
export function DemoView() {
  return (
    <MainLayout>
      <div className="space-y-4">
        <h2 className="text-2xl font-bold">Your Library</h2>
        <p className="text-muted-foreground">
          This is a live preview of the Soul Player desktop interface
        </p>

        {/* Placeholder content showing the interface */}
        <div className="grid grid-cols-4 gap-4 mt-6">
          {[1, 2, 3, 4, 5, 6, 7, 8].map((i) => (
            <div key={i} className="aspect-square bg-accent rounded-lg animate-pulse" />
          ))}
        </div>
      </div>
    </MainLayout>
  );
}
