/**
 * Shared SettingsPage - works on both desktop and marketing demo
 * Uses FeatureGate for platform-specific features
 */

import { useState, useEffect, useRef } from 'react'
import { useTranslation } from 'react-i18next'
import { ThemePicker } from '../theme'
import { Kbd } from '../components/ui/Kbd'
import { usePlatform } from '../contexts/PlatformContext'
import { useBackend } from '../contexts/BackendContext'
import { AudioSettingsPage } from '../components/settings/AudioSettingsPage'
import { LibrarySettingsPage } from '../components/settings/LibrarySettingsPage'
import { ReportBugSettingsPage } from '../components/settings/ReportBugSettingsPage'
import {
  Settings,
  Volume2,
  Keyboard,
  Info,
  ChevronDown,
  FolderOpen,
  Bug,
} from 'lucide-react'
import { debug } from '../utils/debug'
import { Select } from '../components/ui/select'

type SettingsTab = 'general' | 'library' | 'audio' | 'shortcuts' | 'reportBug' | 'about'

interface NavItem {
  id: SettingsTab
  labelKey: string
  icon: React.ComponentType<{ className?: string }>
  featureRequired?: keyof ReturnType<typeof usePlatform>['features']
}

// Settings handlers interface - platform provides these
export interface SettingsHandlers {
  loadSettings?: () => Promise<void>
  handleLanguageChange?: (locale: string) => Promise<void>
  handleAutoUpdateChange?: (enabled: boolean) => Promise<void>
  handleSilentUpdateChange?: (enabled: boolean) => Promise<void>
  checkForUpdates?: () => Promise<void>
  showKeyboardShortcuts?: boolean
  setShowKeyboardShortcuts?: (show: boolean) => void
  hideWindowControls?: boolean
  setHideWindowControls?: (hide: boolean) => void
  autoUpdate?: boolean
  silentUpdate?: boolean
  checking?: boolean
}

// Shortcuts settings component - can be overridden by desktop
export interface ShortcutsSettingsProps {
  disabled?: boolean
}

interface SettingsPageProps {
  handlers?: SettingsHandlers
  ShortcutsSettingsComponent?: React.ComponentType<ShortcutsSettingsProps>
}

export function SettingsPage({ handlers, ShortcutsSettingsComponent }: SettingsPageProps) {
  const { t } = useTranslation()
  const { features } = usePlatform()
  const [activeTab, setActiveTab] = useState<SettingsTab>('general')
  const [mobileMenuOpen, setMobileMenuOpen] = useState(false)
  const mobileMenuRef = useRef<HTMLDivElement>(null)

  // Close mobile menu when clicking outside
  useEffect(() => {
    if (!mobileMenuOpen) return

    const handleClickOutside = (event: MouseEvent) => {
      if (mobileMenuRef.current && !mobileMenuRef.current.contains(event.target as Node)) {
        setMobileMenuOpen(false)
      }
    }

    // Add listener after a brief delay to avoid closing immediately on open
    const timeoutId = setTimeout(() => {
      document.addEventListener('mousedown', handleClickOutside)
    }, 0)

    return () => {
      clearTimeout(timeoutId)
      document.removeEventListener('mousedown', handleClickOutside)
    }
  }, [mobileMenuOpen])

  // Build navigation items based on features
  const navigationItems: NavItem[] = [
    { id: 'general', labelKey: 'settings.general', icon: Settings },
    { id: 'library', labelKey: 'settings.library', icon: FolderOpen, featureRequired: 'hasLibrarySettings' },
    { id: 'audio', labelKey: 'settings.audio.title', icon: Volume2 },
    { id: 'shortcuts', labelKey: 'settings.shortcuts', icon: Keyboard },
    { id: 'reportBug', labelKey: 'settings.reportBug', icon: Bug },
    { id: 'about', labelKey: 'settings.about', icon: Info },
  ]

  // Filter navigation items based on features
  const visibleNavItems = navigationItems.filter(
    (item) => !item.featureRequired || features[item.featureRequired]
  )

  const activeItem = visibleNavItems.find((item) => item.id === activeTab)

  return (
    <div className="h-full flex flex-col md:flex-row">
      {/* Desktop Sidebar - hidden on mobile */}
      <aside className="hidden md:flex w-56 flex-shrink-0 flex-col">
        <nav className="p-4 h-full flex flex-col">
          <ul className="space-y-1">
            {visibleNavItems.map((item) => {
              const Icon = item.icon
              const isActive = activeTab === item.id

              return (
                <li key={item.id}>
                  <button
                    onClick={() => setActiveTab(item.id)}
                    className={`
                      w-full flex items-center gap-3 px-3 py-2 rounded-lg text-sm
                      transition-colors duration-150 text-left
                      ${
                        isActive
                          ? 'bg-primary text-primary-foreground font-medium'
                          : 'text-muted-foreground hover:bg-foreground/[var(--hover-bg-opacity)] hover:opacity-[var(--hover-text-opacity)] transition-opacity duration-[var(--transition-duration)]'
                      }
                    `}
                  >
                    <Icon className="w-4 h-4 flex-shrink-0" />
                    <span>{t(item.labelKey)}</span>
                  </button>
                </li>
              )
            })}
          </ul>
        </nav>
      </aside>

      {/* Mobile Header - visible only on mobile */}
      <div className="md:hidden">
        <div className="relative" ref={mobileMenuRef}>
          <button
            onClick={() => setMobileMenuOpen(!mobileMenuOpen)}
            className="w-full flex items-center justify-between px-4 py-3 text-left"
          >
            <div className="flex items-center gap-3">
              {activeItem && <activeItem.icon className="w-5 h-5" />}
              <span className="font-medium">{activeItem ? t(activeItem.labelKey) : ''}</span>
            </div>
            <ChevronDown
              className={`w-5 h-5 text-muted-foreground transition-transform ${
                mobileMenuOpen ? 'rotate-180' : ''
              }`}
            />
          </button>

          {/* Mobile Dropdown Menu */}
          {mobileMenuOpen && (
            <div className="absolute top-full left-0 right-0 bg-background shadow-lg z-50 border border-border">
              {visibleNavItems.map((item) => {
                const Icon = item.icon
                const isActive = activeTab === item.id

                return (
                  <button
                    key={item.id}
                    onClick={() => {
                      setActiveTab(item.id)
                      setMobileMenuOpen(false)
                    }}
                    className={`
                      w-full flex items-center gap-3 px-4 py-3 text-left
                      transition-colors duration-150
                      ${
                        isActive
                          ? 'bg-primary/10 text-primary font-medium'
                          : 'text-foreground hover:bg-foreground/[var(--hover-bg-opacity)]'
                      }
                    `}
                  >
                    <Icon className="w-4 h-4 flex-shrink-0" />
                    <span>{t(item.labelKey)}</span>
                  </button>
                )
              })}
            </div>
          )}
        </div>
      </div>

      {/* Main Content */}
      <main className="flex-1 overflow-y-auto">
        <div className="max-w-4xl mx-auto p-4 md:p-8">
          {activeTab === 'general' && <GeneralSettings handlers={handlers} />}
          {activeTab === 'library' && features.hasLibrarySettings && <LibrarySettingsPage />}
          {activeTab === 'audio' && <AudioSettingsPage />}
          {activeTab === 'shortcuts' && (
            ShortcutsSettingsComponent ? (
              <ShortcutsSettingsComponent disabled={!features.hasShortcutSettings} />
            ) : (
              <DefaultShortcutsSettings disabled={!features.hasShortcutSettings} />
            )
          )}
          {activeTab === 'reportBug' && <ReportBugSettingsPage />}
          {activeTab === 'about' && <AboutSettings />}
        </div>
      </main>
    </div>
  )
}

// Home Page Settings Component
function HomePageSettings() {
  const { t } = useTranslation()
  const backend = useBackend()
  const [homeEnabled, setHomeEnabled] = useState(true)
  const [refreshMinutes, setRefreshMinutes] = useState<string>('')

  // Load settings on mount
  useEffect(() => {
    Promise.all([
      backend.getUserSetting('home.enabled'),
      backend.getUserSetting('home.refresh_minutes'),
    ])
      .then(([enabled, minutes]) => {
        setHomeEnabled(enabled ?? true)
        setRefreshMinutes(minutes !== null && minutes !== undefined ? String(minutes) : '')
      })
      .catch(err => debug.error('Failed to load home settings:', err))
  }, [backend])

  const handleEnabledChange = (enabled: boolean) => {
    setHomeEnabled(enabled)
    backend.setUserSetting('home.enabled', enabled)
      .then(() => {
        // Dispatch event to notify LeftSidebar to refresh
        window.dispatchEvent(new CustomEvent('home-enabled-changed', { detail: { enabled } }))
      })
      .catch(err => debug.error('Failed to save home enabled:', err))
  }

  const handleRefreshChange = (value: string) => {
    setRefreshMinutes(value)
    const minutes = value === '' ? null : parseInt(value, 10)
    if (value === '' || !isNaN(minutes!)) {
      backend.setUserSetting('home.refresh_minutes', minutes)
        .catch(err => debug.error('Failed to save refresh minutes:', err))
    }
  }

  return (
    <section>
      <h2 className="text-xl font-semibold mb-4">{t('settings.homePage')}</h2>
      <div className="space-y-6">
        <label className="flex items-start space-x-3 cursor-pointer">
          <input
            type="checkbox"
            checked={homeEnabled}
            onChange={(e) => handleEnabledChange(e.target.checked)}
            className="w-4 h-4 mt-0.5"
          />
          <div>
            <span className="text-sm font-medium block">{t('settings.homePageEnabled')}</span>
            <p className="text-xs text-muted-foreground mt-1">
              {t('settings.homePageEnabledDescription')}
            </p>
          </div>
        </label>

        <div>
          <label className="block text-sm font-medium mb-2">
            {t('settings.homeRefreshInterval')}
          </label>
          <div className="flex items-center gap-3">
            <input
              type="number"
              min="1"
              step="1"
              value={refreshMinutes}
              onChange={(e) => handleRefreshChange(e.target.value)}
              placeholder={t('settings.homeRefreshOnVisit')}
              className="w-32 px-3 py-2 rounded-lg bg-muted text-sm [appearance:textfield] [&::-webkit-outer-spin-button]:appearance-none [&::-webkit-inner-spin-button]:appearance-none"
            />
            {refreshMinutes && (
              <span className="text-sm text-muted-foreground">{t('settings.homeRefreshMinutes')}</span>
            )}
          </div>
          <p className="text-xs text-muted-foreground mt-1">
            {t('settings.homeRefreshIntervalDescription')}
          </p>
        </div>
      </div>
    </section>
  )
}

// General Settings Tab Content
function GeneralSettings({ handlers }: { handlers?: SettingsHandlers }) {
  const { t, i18n } = useTranslation()
  const { features } = usePlatform()

  const canChangeLanguage = features.hasLanguageSettings
  const canChangeUpdates = features.hasUpdateSettings
  const canImportExportThemes = features.hasThemeImportExport

  return (
    <div className="space-y-8">
      {/* Home Page Section */}
      <HomePageSettings />

      {/* Appearance Section */}
      <section>
        <h2 className="text-xl font-semibold mb-4">{t('settings.appearance')}</h2>
        <div className="space-y-6">
          <div>
            <label className="block text-sm font-medium mb-2">{t('settings.theme')}</label>
            <ThemePicker
              showImportExport={canImportExportThemes}
              showAccessibilityInfo={true}
            />
          </div>

          <div>
            <label className="block text-sm font-medium mb-2">{t('settings.language')}</label>
            <Select
              value={i18n.language}
              onChange={(value) => handlers?.handleLanguageChange?.(value)}
              disabled={!canChangeLanguage}
              options={[
                { value: 'en-US', label: 'English (US)' },
                { value: 'de', label: 'Deutsch' },
                { value: 'ja', label: '日本語' },
              ]}
            />
            {!canChangeLanguage && (
              <p className="text-xs text-muted-foreground mt-1">
                {t('settings.demoDisabled')}
              </p>
            )}
          </div>

          <div>
            <label className="flex items-start space-x-3 cursor-pointer">
              <input
                type="checkbox"
                checked={handlers?.hideWindowControls ?? false}
                onChange={(e) => handlers?.setHideWindowControls?.(e.target.checked)}
                className="w-4 h-4 mt-0.5"
              />
              <div>
                <span className="text-sm font-medium block">{t('settings.hideWindowControls')}</span>
                <p className="text-xs text-muted-foreground mt-1">
                  {t('settings.hideWindowControlsDescription')}
                </p>
              </div>
            </label>
          </div>

          <div>
            <label className={`flex items-start space-x-3 ${
              !features.hasShortcutSettings ? 'cursor-not-allowed opacity-60' : 'cursor-pointer'
            }`}>
              <input
                type="checkbox"
                checked={handlers?.showKeyboardShortcuts ?? true}
                onChange={(e) => handlers?.setShowKeyboardShortcuts?.(e.target.checked)}
                disabled={!features.hasShortcutSettings}
                className="w-4 h-4 mt-0.5"
              />
              <div>
                <span className="text-sm font-medium block">{t('settings.showShortcuts')}</span>
                <p className="text-xs text-muted-foreground mt-1">
                  {t('settings.showShortcutsDescription')} <Kbd keys={['mod', 'k']} size="sm" />
                </p>
              </div>
            </label>
          </div>
        </div>
      </section>

      {/* Updates Section */}
      <section>
        <h2 className="text-xl font-semibold mb-4">{t('settings.updates')}</h2>
        <div className="space-y-4">
          <label className={`flex items-center space-x-3 ${
            !canChangeUpdates ? 'cursor-not-allowed opacity-60' : 'cursor-pointer'
          }`}>
            <input
              type="checkbox"
              checked={handlers?.autoUpdate ?? true}
              onChange={(e) => handlers?.handleAutoUpdateChange?.(e.target.checked)}
              disabled={!canChangeUpdates}
              className="w-4 h-4"
              data-testid="auto-update-toggle"
            />
            <span className="text-sm">{t('settings.autoUpdate')}</span>
          </label>

          <label className={`flex items-center space-x-3 ${
            !canChangeUpdates ? 'cursor-not-allowed opacity-60' : 'cursor-pointer'
          }`}>
            <input
              type="checkbox"
              checked={handlers?.silentUpdate ?? false}
              onChange={(e) => handlers?.handleSilentUpdateChange?.(e.target.checked)}
              disabled={!canChangeUpdates || !handlers?.autoUpdate}
              className="w-4 h-4 disabled:opacity-[var(--disabled-opacity)]"
              data-testid="silent-update-toggle"
            />
            <span className={`text-sm ${!handlers?.autoUpdate ? 'text-muted-foreground' : ''}`}>
              {t('settings.silentUpdate')}
            </span>
          </label>

          <button
            onClick={() => handlers?.checkForUpdates?.()}
            disabled={!canChangeUpdates || handlers?.checking}
            className={`px-4 py-2 bg-primary text-primary-foreground rounded-lg ${
              canChangeUpdates ? 'hover:opacity-[var(--hover-button-opacity)] transition-opacity' : 'opacity-50 cursor-not-allowed'
            } disabled:opacity-[var(--disabled-opacity)]`}
            data-testid="check-for-updates-button"
          >
            {handlers?.checking ? t('settings.checking') : t('settings.checkNow')}
          </button>
          {!canChangeUpdates && (
            <p className="text-xs text-muted-foreground">
              {t('settings.demoDisabled')}
            </p>
          )}
        </div>
      </section>
    </div>
  )
}

// Default Shortcuts Settings - read-only display
function DefaultShortcutsSettings({ disabled }: ShortcutsSettingsProps) {
  const { t } = useTranslation()
  const shortcuts = [
    { action: 'play_pause', labelKey: 'shortcuts.playPause', keys: ['space'] },
    { action: 'next', labelKey: 'shortcuts.next', keys: ['mod', 'right'] },
    { action: 'previous', labelKey: 'shortcuts.previous', keys: ['mod', 'left'] },
    { action: 'volume_up', labelKey: 'shortcuts.volumeUp', keys: ['mod', 'up'] },
    { action: 'volume_down', labelKey: 'shortcuts.volumeDown', keys: ['mod', 'down'] },
    { action: 'mute', labelKey: 'shortcuts.mute', keys: ['mod', 'm'] },
    { action: 'toggle_shuffle', labelKey: 'shortcuts.toggleShuffle', keys: ['mod', 's'] },
    { action: 'toggle_repeat', labelKey: 'shortcuts.toggleRepeat', keys: ['mod', 'r'] },
  ]

  return (
    <div className="max-w-2xl">
      <div className="mb-6">
        <h2 className="text-2xl font-semibold mb-2">{t('settings.shortcuts')}</h2>
        <p className="text-sm text-muted-foreground">
          {t('settings.shortcutsDescription', 'Configure keyboard shortcuts for playback control. Shortcuts only work when the app is focused and are disabled when typing in text fields.')}
        </p>
      </div>

      <div className="bg-card rounded-lg border border-border p-4 mb-6">
        {shortcuts.map((shortcut, index) => (
          <div
            key={shortcut.action}
            className={`flex items-center justify-between py-3 ${
              index !== shortcuts.length - 1 ? 'border-b border-border' : ''
            }`}
          >
            <span className="text-sm font-medium">{t(shortcut.labelKey)}</span>
            <button
              className={`min-w-[120px] px-3 py-1.5 rounded-md text-sm bg-muted ${
                disabled ? 'cursor-not-allowed opacity-70' : ''
              }`}
              disabled={disabled}
            >
              <Kbd keys={shortcut.keys} size="sm" />
            </button>
          </div>
        ))}
      </div>

      <button
        className={`px-4 py-2 border border-border rounded-lg text-sm ${
          disabled ? 'opacity-50 cursor-not-allowed' : ''
        }`}
        disabled={disabled}
      >
        {t('settings.resetShortcuts')}
      </button>
      {disabled && (
        <p className="text-xs text-muted-foreground mt-2">
          {t('settings.demoDisabled')}
        </p>
      )}
    </div>
  )
}

// About Settings Tab Content
function AboutSettings() {
  const { t } = useTranslation()
  const { isWeb } = usePlatform()
  const backend = useBackend()
  const [version, setVersion] = useState<string>('...')

  // Load version on mount
  useEffect(() => {
    backend.getVersion()
      .then(v => setVersion(v))
      .catch(err => {
        debug.error('Failed to load version:', err)
        setVersion('Unknown')
      })
  }, [backend])

  return (
    <div className="space-y-8">
      <section>
        <div className="flex items-center gap-4 mb-4">
          <div className="w-14 h-14 bg-primary/10 rounded-xl flex items-center justify-center">
            <Volume2 className="w-7 h-7 text-primary" />
          </div>
          <div>
            <h3 className="text-lg font-semibold">{t('app.title', 'Soul Player')}</h3>
            <p className="text-sm text-muted-foreground">
              {t('settings.version')} {version}
            </p>
          </div>
        </div>
        <p className="text-sm text-muted-foreground">
          {t('settings.aboutDescription')}
        </p>
      </section>

      {/* Features list - shown on web demo */}
      {isWeb && (
        <section>
          <h2 className="text-sm font-medium mb-3">{t('marketing.features.title', 'Features')}</h2>
          <ul className="space-y-2 text-sm text-muted-foreground">
            <li className="flex items-center gap-2">
              <span className="text-green-500">✓</span>
              Symphonia-powered audio decoding (FLAC, MP3, WAV, AAC, OGG)
            </li>
            <li className="flex items-center gap-2">
              <span className="text-green-500">✓</span>
              High-quality DSP with EQ, compressor, and limiter
            </li>
            <li className="flex items-center gap-2">
              <span className="text-green-500">✓</span>
              Professional-grade resampling (r8brain algorithm)
            </li>
            <li className="flex items-center gap-2">
              <span className="text-green-500">✓</span>
              Gapless playback with crossfade support
            </li>
            <li className="flex items-center gap-2">
              <span className="text-green-500">✓</span>
              ReplayGain and EBU R128 volume leveling
            </li>
            <li className="flex items-center gap-2">
              <span className="text-green-500">✓</span>
              ASIO and JACK support for low-latency audio
            </li>
          </ul>
        </section>
      )}

      <section>
        <h2 className="text-sm font-medium mb-3">{t('settings.links')}</h2>
        <div className="space-y-2">
          <a
            href="https://github.com/soulaudio/soul-player"
            target="_blank"
            rel="noopener noreferrer"
            className="block text-sm text-primary hover:underline"
          >
            {t('settings.github')}
          </a>
          <a
            href="https://soulplayer.app"
            target="_blank"
            rel="noopener noreferrer"
            className="block text-sm text-primary hover:underline"
          >
            {t('settings.website')}
          </a>
        </div>
      </section>
    </div>
  )
}
