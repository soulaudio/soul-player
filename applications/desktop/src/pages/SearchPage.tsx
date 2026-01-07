import { useState } from 'react';

export function SearchPage() {
  const [query, setQuery] = useState('');

  return (
    <div>
      <h1 className="text-3xl font-bold mb-6">Search</h1>
      <div className="max-w-2xl">
        <input
          type="text"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder="Search for tracks, artists, albums..."
          className="w-full px-4 py-3 border rounded-lg bg-background text-foreground"
          autoFocus
        />
        {query && (
          <div className="mt-4">
            <p className="text-muted-foreground">Search results will appear here...</p>
          </div>
        )}
      </div>
    </div>
  );
}
