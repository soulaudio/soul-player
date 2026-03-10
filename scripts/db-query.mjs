// Quick DB diagnostic script
// Run: node --experimental-sqlite scripts/db-query.mjs
import { DatabaseSync } from 'node:sqlite';

const DB_PATH = 'C:/Users/sebas/AppData/Roaming/Soul Player/soul-player.db';
const db = new DatabaseSync(DB_PATH, { open: true });

// Schema peek
const trackCols = db.prepare("PRAGMA table_info(tracks)").all();
const hasFolderPath = trackCols.some(c => c.name === 'folder_path');
console.log('tracks columns:', trackCols.map(c => c.name).join(', '));
const albumCols = db.prepare("PRAGMA table_info(albums)").all();
console.log('albums columns:', albumCols.map(c => c.name).join(', '));

// --- Duplicate albums ---
console.log('\n=== DUPLICATE ALBUMS (same title+artist) ===');
const dupes = db.prepare(`
  SELECT al.title, ar.name AS artist, COUNT(*) AS cnt, GROUP_CONCAT(al.id) AS ids
  FROM albums al
  LEFT JOIN artists ar ON al.artist_id = ar.id
  GROUP BY LOWER(TRIM(al.title)), al.artist_id
  HAVING cnt > 1
  ORDER BY cnt DESC
  LIMIT 30
`).all();
if (dupes.length === 0) console.log('  (none — no duplicates!)');
else dupes.forEach(r => console.log(`  ${r.cnt}x | ${r.artist} - ${r.title} | ids: ${r.ids}`));
console.log('  Total dupe groups:', dupes.length);

// --- Mid-Air Thief albums ---
console.log('\n=== MID-AIR THIEF ALBUMS ===');
const midAir = db.prepare(`
  SELECT al.id, al.title, ar.name AS artist,
         (SELECT COUNT(*) FROM tracks t WHERE t.album_id = al.id) AS track_count
  FROM albums al
  LEFT JOIN artists ar ON al.artist_id = ar.id
  WHERE LOWER(ar.name) LIKE '%mid-air%'
  ORDER BY al.title
`).all();
midAir.forEach(r => console.log(`  [${r.id}] "${r.title}" | tracks: ${r.track_count}`));
if (midAir.length === 0) console.log('  (none found)');

// --- 기다림 specifically ---
console.log('\n=== ALBUMS CONTAINING 기다림 OR Waiting ===');
const waiting = db.prepare(`
  SELECT al.id, al.title, ar.name AS artist,
         (SELECT COUNT(*) FROM tracks t WHERE t.album_id = al.id) AS track_count
  FROM albums al
  LEFT JOIN artists ar ON al.artist_id = ar.id
  WHERE al.title LIKE '%기다림%' OR al.title LIKE '%Waiting%'
`).all();
if (waiting.length === 0) console.log('  (not found in DB at all)');
else waiting.forEach(r => console.log(`  [${r.id}] "${r.title}" | artist: ${r.artist} | tracks: ${r.track_count}`));

// --- Albums with 0 tracks ---
console.log('\n=== ALL ALBUMS WITH 0 TRACKS ===');
const zeroTracks = db.prepare(`
  SELECT al.id, al.title, ar.name AS artist
  FROM albums al
  LEFT JOIN artists ar ON al.artist_id = ar.id
  WHERE (SELECT COUNT(*) FROM tracks t WHERE t.album_id = al.id) = 0
  ORDER BY ar.name, al.title
`).all();
zeroTracks.forEach(r => console.log(`  [${r.id}] ${r.artist} - "${r.title}"`));
console.log(`  Total: ${zeroTracks.length}`);

// --- Tracks that reference 기다림 in file path ---
console.log('\n=== TRACKS WITH 기다림 IN FILE PATH ===');
const kTracks = db.prepare(`
  SELECT t.id, t.title, t.file_path, t.album_id
  FROM tracks t
  WHERE t.file_path LIKE '%기다림%'
`).all();
if (kTracks.length === 0) console.log('  (none — folder may not be scanned or path encoding issue)');
else kTracks.forEach(r => console.log(`  [${r.id}] "${r.title}" | album: ${r.album_id} | ${r.file_path}`));

// --- Check what tracks exist from Mid-Air Thief area ---
console.log('\n=== TRACKS FROM Mid-Air Thief albums ===');
const matTracks = db.prepare(`
  SELECT t.id, t.title, t.file_path, al.title AS album
  FROM tracks t
  JOIN albums al ON t.album_id = al.id
  JOIN artists ar ON al.artist_id = ar.id
  WHERE LOWER(ar.name) LIKE '%mid-air%'
  ORDER BY al.title, t.track_number
  LIMIT 40
`).all();
if (matTracks.length === 0) console.log('  (none)');
else matTracks.forEach(r => console.log(`  [${r.id}] ${r.album} - "${r.title}" | ${r.file_path}`));

// --- Library sources ---
console.log('\n=== LIBRARY SOURCES ===');
const sources = db.prepare(`SELECT id, path, device_id, last_scanned_at FROM library_sources`).all();
sources.forEach(r => console.log(`  id: ${r.id} | path: ${r.path} | device: ${r.device_id} | scanned: ${r.last_scanned_at}`));

// --- Total counts ---
console.log('\n=== TOTALS ===');
const totals = db.prepare(`
  SELECT
    (SELECT COUNT(*) FROM albums) AS albums,
    (SELECT COUNT(*) FROM artists) AS artists,
    (SELECT COUNT(*) FROM tracks) AS tracks
`).get();
console.log(`  Albums: ${totals.albums}, Artists: ${totals.artists}, Tracks: ${totals.tracks}`);

db.close();
