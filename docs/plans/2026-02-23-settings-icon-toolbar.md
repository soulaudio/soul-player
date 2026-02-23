# Settings Icon Toolbar Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace the desktop settings text sidebar with a Figma-style icon-only toolbar — narrow, vertically centered, with Radix UI tooltips on hover delay.

**Architecture:** Two files change. `SettingsLayout.tsx` shrinks the sidebar width. `SettingsSidebar.tsx` is rewritten to show icon-only buttons wrapped in Radix `Tooltip` with `side="right"` and `delayDuration={600}`. No routing, no other pages touched.

**Tech Stack:** React, React Router `Link`, Radix UI `@radix-ui/react-tooltip` (already installed), Tailwind CSS v4, `lucide-react` icons, `react-i18next`

---

### Task 1: Shrink the sidebar container

**Files:**
- Modify: `applications/shared/src/components/settings/SettingsLayout.tsx`

**Step 1: Read the file**

Read `applications/shared/src/components/settings/SettingsLayout.tsx` to confirm current state.

**Step 2: Change sidebar width**

In `SettingsLayout.tsx`, change:
```tsx
<aside className="w-56 border-r border-border bg-card/30 flex-shrink-0">
```
to:
```tsx
<aside className="w-14 border-r border-border bg-card/30 flex-shrink-0">
```

`w-14` = 56px — just enough for a comfortably padded icon button.

**Step 3: Verify no other width references**

Confirm there's nothing else in the file setting a fixed width on the sidebar.

**Step 4: Commit**

```bash
git add applications/shared/src/components/settings/SettingsLayout.tsx
git commit -m "feat(settings): narrow sidebar to icon-toolbar width"
```

---

### Task 2: Rewrite SettingsSidebar with Radix tooltips

**Files:**
- Modify: `applications/shared/src/components/settings/SettingsSidebar.tsx`

**Step 1: Read the file**

Read `applications/shared/src/components/settings/SettingsSidebar.tsx` to confirm current state.

**Step 2: Rewrite the file**

Replace the entire file content with:

```tsx
// Settings sidebar navigation — icon-only toolbar with Radix tooltips

import { useTranslation } from 'react-i18next';
import { useLocation, Link } from 'react-router-dom';
import * as Tooltip from '@radix-ui/react-tooltip';
import {
  Volume2,
  Palette,
  Music,
  Zap,
  Info,
  Keyboard,
  Bug,
  Database,
} from 'lucide-react';

interface NavItem {
  id: string;
  labelKey: string;
  path: string;
  icon: React.ComponentType<{ className?: string }>;
}

const navigationItems: NavItem[] = [
  { id: 'audio',          labelKey: 'settings.sections.audio',          path: '/settings/audio',           icon: Volume2   },
  { id: 'library',        labelKey: 'settings.sections.library',        path: '/settings/library',         icon: Music     },
  { id: 'playback',       labelKey: 'settings.sections.playback',       path: '/settings/playback',        icon: Zap       },
  { id: 'appearance',     labelKey: 'settings.sections.appearance',     path: '/settings/appearance',      icon: Palette   },
  { id: 'shortcuts',      labelKey: 'settings.sections.shortcuts',      path: '/settings/shortcuts',       icon: Keyboard  },
  { id: 'dataManagement', labelKey: 'settings.sections.dataManagement', path: '/settings/data-management', icon: Database  },
  { id: 'reportBug',      labelKey: 'settings.sections.reportBug',      path: '/settings/report-bug',      icon: Bug       },
  { id: 'about',          labelKey: 'settings.sections.about',          path: '/settings/about',           icon: Info      },
];

export function SettingsSidebar() {
  const { t } = useTranslation();
  const location = useLocation();

  return (
    <Tooltip.Provider delayDuration={600}>
      <nav className="h-full flex flex-col items-center justify-center py-4 gap-1">
        {navigationItems.map((item) => {
          const Icon = item.icon;
          const isActive = location.pathname === item.path;

          return (
            <Tooltip.Root key={item.id}>
              <Tooltip.Trigger asChild>
                <Link
                  to={item.path}
                  data-state={isActive ? 'active' : 'inactive'}
                  aria-current={isActive ? 'page' : undefined}
                  className={[
                    'p-3 rounded-lg transition-all',
                    isActive
                      ? 'text-primary bg-accent/20'
                      : 'text-muted-foreground hover:opacity-80 hover:bg-foreground/10',
                  ].join(' ')}
                >
                  <Icon className="w-5 h-5" />
                </Link>
              </Tooltip.Trigger>
              <Tooltip.Portal>
                <Tooltip.Content
                  side="right"
                  sideOffset={8}
                  className="z-50 px-2 py-1 text-xs rounded-md bg-popover text-popover-foreground shadow-md select-none"
                >
                  {t(item.labelKey)}
                  <Tooltip.Arrow className="fill-popover" />
                </Tooltip.Content>
              </Tooltip.Portal>
            </Tooltip.Root>
          );
        })}
      </nav>
    </Tooltip.Provider>
  );
}
```

**Key decisions in this code:**
- `Tooltip.Provider` wraps all items with a shared `delayDuration={600}` — no per-item overhead
- `Tooltip.Trigger asChild` passes the ref to the `Link` so routing still works normally
- `Tooltip.Portal` renders the tooltip outside the sidebar DOM so it's never clipped by `overflow` on the sidebar container
- Active/hover states follow CLAUDE.md patterns exactly: `accent/20` background + `text-primary` for active, `foreground/10` + `opacity-80` for hover
- `justify-center` vertically centers the icon group; `gap-1` keeps spacing tight like Figma's toolbar
- Icons bumped to `w-5 h-5` (from `w-4 h-4`) since there's no label competing for space

**Step 3: Commit**

```bash
git add applications/shared/src/components/settings/SettingsSidebar.tsx
git commit -m "feat(settings): icon-only toolbar with Radix tooltips"
```

---

### Task 3: Smoke test in the running app

**Step 1: Start the desktop app**

```bash
cargo xtask dev desktop
```

**Step 2: Navigate to Settings**

Open settings and verify:
- [ ] Sidebar is narrow (~56px) — no text labels visible
- [ ] Icons are vertically centered in the sidebar
- [ ] Clicking each icon navigates to the correct page
- [ ] Active icon has `text-primary` color + subtle background
- [ ] Hovering an icon after ~600ms shows the label tooltip to the right
- [ ] Tooltip disappears when mouse leaves
- [ ] Tooltip is never clipped by the sidebar edge

**Step 3: Run TypeScript check**

```bash
cargo xtask check typescript
```

Expected: no errors.

**Step 4: Final commit if any fixups were needed**

```bash
git add -p
git commit -m "fix(settings): sidebar tooltip fixups"
```

---

### Summary

| File | Change |
|---|---|
| `applications/shared/src/components/settings/SettingsLayout.tsx` | `w-56` → `w-14` |
| `applications/shared/src/components/settings/SettingsSidebar.tsx` | Full rewrite — icon toolbar with Radix tooltips |

No routing changes. No shared `SettingsPage.tsx` changes (that's the marketing/web variant). No new dependencies.
