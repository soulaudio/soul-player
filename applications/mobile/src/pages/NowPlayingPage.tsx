export function NowPlayingPage() {
  return (
    <div className="flex flex-col items-center justify-center h-full p-6">
      <div className="w-full max-w-sm">
        {/* Album Art */}
        <div className="aspect-square bg-muted rounded-lg mb-6 flex items-center justify-center">
          <span className="text-6xl">🎵</span>
        </div>

        {/* Track Info */}
        <div className="text-center mb-6">
          <h2 className="text-2xl font-bold mb-2">No Track</h2>
          <p className="text-muted-foreground">Not playing</p>
        </div>

        {/* Progress Bar */}
        <div className="mb-6">
          <div className="h-1 bg-muted rounded-full overflow-hidden">
            <div className="h-full bg-primary w-0" />
          </div>
          <div className="flex justify-between text-xs text-muted-foreground mt-1">
            <span>0:00</span>
            <span>0:00</span>
          </div>
        </div>

        {/* Controls */}
        <div className="flex items-center justify-center gap-6">
          <button className="p-3 hover:bg-accent rounded-full">
            <span className="text-2xl">⏮</span>
          </button>
          <button className="p-4 bg-primary hover:bg-primary/90 rounded-full">
            <span className="text-3xl text-primary-foreground">▶</span>
          </button>
          <button className="p-3 hover:bg-accent rounded-full">
            <span className="text-2xl">⏭</span>
          </button>
        </div>
      </div>
    </div>
  );
}
