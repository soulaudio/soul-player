import { useState, useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import { invoke } from '@tauri-apps/api/core';
import {
  FolderOpen,
  FolderInput,
  ArrowRight,
  ArrowLeft,
  Check,
  Plus,
  Trash2,
  Loader2,
  Sun,
  Moon,
  Waves,
  Leaf,
  Globe,
} from 'lucide-react';

type SetupType = 'watched' | 'managed' | 'both' | null;
type Step = 'theme' | 'strategy' | 'setup' | 'complete';

interface OnboardingPageProps {
  onComplete: () => void;
}

// Theme definitions with visual previews
const THEMES = [
  {
    id: 'light',
    name: 'Light',
    icon: Sun,
    colors: {
      bg: '#ffffff',
      card: '#f8fafc',
      primary: '#6366f1',
      text: '#0f172a',
      muted: '#64748b',
    },
  },
  {
    id: 'dark',
    name: 'Dark',
    icon: Moon,
    colors: {
      bg: '#0f172a',
      card: '#1e293b',
      primary: '#818cf8',
      text: '#f8fafc',
      muted: '#94a3b8',
    },
  },
  {
    id: 'ocean',
    name: 'Ocean',
    icon: Waves,
    colors: {
      bg: '#0c1222',
      card: '#162032',
      primary: '#38bdf8',
      text: '#e0f2fe',
      muted: '#7dd3fc',
    },
  },
  {
    id: 'earth',
    name: 'Earth',
    icon: Leaf,
    colors: {
      bg: '#1a1a1a',
      card: '#262626',
      primary: '#84cc16',
      text: '#fafaf9',
      muted: '#a3a3a3',
    },
  },
];

export function OnboardingPage({ onComplete }: OnboardingPageProps) {
  const { t, i18n } = useTranslation();
  const [step, setStep] = useState<Step>('theme');
  const [selectedTheme, setSelectedTheme] = useState<string>('dark');
  const [setupType, setSetupType] = useState<SetupType>(null);
  const [loading, setLoading] = useState(false);

  // Watched folders state
  const [watchedFolders, setWatchedFolders] = useState<{ name: string; path: string }[]>([]);

  // Managed library state
  const [libraryPath, setLibraryPath] = useState('');
  const [pathTemplate, setPathTemplate] = useState('{AlbumArtist}/{Year} - {Album}/{TrackNo} - {Title}');

  // Load default library path and check for existing theme
  useEffect(() => {
    invoke<string>('get_default_library_path').then(setLibraryPath);

    // Check if theme was already set
    invoke<string | null>('get_user_setting', { key: 'ui.theme' }).then((theme) => {
      if (theme) {
        const parsed = JSON.parse(theme);
        setSelectedTheme(parsed);
      }
    });
  }, []);

  const handleLanguageChange = (locale: string) => {
    i18n.changeLanguage(locale);
    invoke('set_user_setting', {
      key: 'ui.locale',
      value: JSON.stringify(locale),
    });
  };

  const handleThemeSelect = async (themeId: string) => {
    setSelectedTheme(themeId);
    // Apply theme immediately
    await invoke('set_user_setting', {
      key: 'ui.theme',
      value: JSON.stringify(themeId),
    });
    // Trigger theme change in the app
    document.documentElement.setAttribute('data-theme', themeId);
  };

  const handleBrowseFolder = async () => {
    try {
      const folder = await invoke<string | null>('open_folder_dialog');
      if (folder) {
        // Extract folder name from path
        const parts = folder.split(/[/\\]/);
        const name = parts[parts.length - 1] || 'Music Folder';
        setWatchedFolders([...watchedFolders, { name, path: folder }]);
      }
    } catch (error) {
      console.error('Failed to open folder dialog:', error);
    }
  };

  const handleBrowseLibraryPath = async () => {
    try {
      const folder = await invoke<string | null>('open_folder_dialog');
      if (folder) {
        setLibraryPath(folder);
      }
    } catch (error) {
      console.error('Failed to open folder dialog:', error);
    }
  };

  const handleRemoveFolder = (index: number) => {
    setWatchedFolders(watchedFolders.filter((_, i) => i !== index));
  };

  const handleComplete = async () => {
    if (!setupType) return;

    setLoading(true);
    try {
      // Add watched folders if selected
      if (setupType === 'watched' || setupType === 'both') {
        for (const folder of watchedFolders) {
          await invoke('add_library_source', {
            name: folder.name,
            path: folder.path,
            syncDeletes: true,
          });
        }
      }

      // Set up managed library if selected
      if (setupType === 'managed' || setupType === 'both') {
        await invoke('set_managed_library_settings', {
          libraryPath,
          pathTemplate,
          importAction: 'copy',
        });
      }

      // Mark onboarding as complete
      await invoke('complete_onboarding', { setupType });

      // Trigger initial scan if we have watched folders
      if ((setupType === 'watched' || setupType === 'both') && watchedFolders.length > 0) {
        invoke('rescan_all_sources');
      }

      setStep('complete');
      setTimeout(onComplete, 1500);
    } catch (error) {
      console.error('Failed to complete onboarding:', error);
    } finally {
      setLoading(false);
    }
  };

  const handleStrategySelect = (type: SetupType) => {
    setSetupType(type);
    setStep('setup');
  };

  const canProceedSetup = () => {
    if (setupType === 'watched') return watchedFolders.length > 0;
    if (setupType === 'managed') return libraryPath.trim() !== '';
    if (setupType === 'both') return libraryPath.trim() !== '';
    return false;
  };

  const handleSkip = () => {
    invoke('complete_onboarding', { setupType: 'watched' });
    onComplete();
  };

  return (
    <div className="min-h-screen bg-background flex flex-col">
      {/* Content */}
      <main className="flex-1 flex items-center justify-center p-8">
        <div className="w-full max-w-3xl">
          {step === 'theme' && (
            <ThemeStep
              selectedTheme={selectedTheme}
              onThemeSelect={handleThemeSelect}
              onContinue={() => setStep('strategy')}
              t={t}
            />
          )}

          {step === 'strategy' && (
            <StrategyStep
              setupType={setupType}
              onStrategySelect={handleStrategySelect}
              onBack={() => setStep('theme')}
              onSkip={handleSkip}
              t={t}
            />
          )}

          {step === 'setup' && (
            <SetupStep
              setupType={setupType!}
              watchedFolders={watchedFolders}
              libraryPath={libraryPath}
              pathTemplate={pathTemplate}
              onBrowseFolder={handleBrowseFolder}
              onRemoveFolder={handleRemoveFolder}
              onBrowseLibraryPath={handleBrowseLibraryPath}
              onLibraryPathChange={setLibraryPath}
              onPathTemplateChange={setPathTemplate}
              onBack={() => setStep('strategy')}
              onComplete={handleComplete}
              canProceed={canProceedSetup()}
              loading={loading}
              t={t}
            />
          )}

          {step === 'complete' && <CompleteStep t={t} />}
        </div>
      </main>

      {/* Footer with language selector */}
      {step !== 'complete' && (
        <footer className="p-6 flex justify-center">
          <div className="flex items-center gap-2 text-sm text-muted-foreground">
            <Globe className="w-4 h-4" />
            <select
              value={i18n.language}
              onChange={(e) => handleLanguageChange(e.target.value)}
              className="bg-transparent border-none focus:outline-none cursor-pointer"
            >
              <option value="en-US">English</option>
              <option value="de">Deutsch</option>
              <option value="ja">Japanese</option>
            </select>
          </div>
        </footer>
      )}
    </div>
  );
}

// Theme Step Component
interface ThemeStepProps {
  selectedTheme: string;
  onThemeSelect: (themeId: string) => void;
  onContinue: () => void;
  t: (key: string) => string;
}

function ThemeStep({ selectedTheme, onThemeSelect, onContinue, t }: ThemeStepProps) {
  return (
    <div className="space-y-8">
      <div className="text-center space-y-2">
        <h1 className="text-3xl font-bold">{t('onboarding.chooseTheme')}</h1>
        <p className="text-muted-foreground">{t('onboarding.chooseThemeDescription')}</p>
      </div>

      <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
        {THEMES.map((theme) => {
          const Icon = theme.icon;
          const isSelected = selectedTheme === theme.id;

          return (
            <button
              key={theme.id}
              onClick={() => onThemeSelect(theme.id)}
              className={`relative rounded-xl overflow-hidden transition-all ${
                isSelected ? 'ring-2 ring-primary ring-offset-2 ring-offset-background' : ''
              }`}
            >
              {/* Theme Preview Card */}
              <div
                className="aspect-[4/3] p-3"
                style={{ backgroundColor: theme.colors.bg }}
              >
                {/* Mini app preview */}
                <div
                  className="h-full rounded-lg p-2 flex flex-col gap-1"
                  style={{ backgroundColor: theme.colors.card }}
                >
                  {/* Header bar */}
                  <div className="flex items-center gap-1">
                    <div
                      className="w-2 h-2 rounded-full"
                      style={{ backgroundColor: theme.colors.primary }}
                    />
                    <div
                      className="flex-1 h-1.5 rounded"
                      style={{ backgroundColor: theme.colors.muted, opacity: 0.3 }}
                    />
                  </div>
                  {/* Content lines */}
                  <div className="flex-1 flex flex-col gap-1 mt-1">
                    <div
                      className="h-1 rounded w-3/4"
                      style={{ backgroundColor: theme.colors.text, opacity: 0.2 }}
                    />
                    <div
                      className="h-1 rounded w-1/2"
                      style={{ backgroundColor: theme.colors.text, opacity: 0.15 }}
                    />
                  </div>
                  {/* Footer bar */}
                  <div
                    className="h-2 rounded"
                    style={{ backgroundColor: theme.colors.primary, opacity: 0.5 }}
                  />
                </div>
              </div>

              {/* Theme Name & Checkbox */}
              <div
                className="p-3 flex items-center justify-between"
                style={{ backgroundColor: theme.colors.card }}
              >
                <div className="flex items-center gap-2">
                  <Icon className="w-4 h-4" style={{ color: theme.colors.primary }} />
                  <span className="text-sm font-medium" style={{ color: theme.colors.text }}>
                    {theme.name}
                  </span>
                </div>
                <div
                  className={`w-5 h-5 rounded-full border-2 flex items-center justify-center ${
                    isSelected ? 'border-primary bg-primary' : 'border-muted-foreground'
                  }`}
                >
                  {isSelected && <Check className="w-3 h-3 text-primary-foreground" />}
                </div>
              </div>
            </button>
          );
        })}
      </div>

      <div className="flex justify-center">
        <button
          onClick={onContinue}
          className="flex items-center gap-2 px-8 py-3 bg-primary text-primary-foreground rounded-lg hover:bg-primary/90 transition-colors font-medium"
        >
          {t('common.continue')}
          <ArrowRight className="w-4 h-4" />
        </button>
      </div>
    </div>
  );
}

// Strategy Step Component
interface StrategyStepProps {
  setupType: SetupType;
  onStrategySelect: (type: SetupType) => void;
  onBack: () => void;
  onSkip: () => void;
  t: (key: string) => string;
}

function StrategyStep({ setupType, onStrategySelect, onBack, onSkip, t }: StrategyStepProps) {
  const [selected, setSelected] = useState<SetupType>(setupType);

  return (
    <div className="space-y-8">
      <div className="text-center space-y-2">
        <h1 className="text-3xl font-bold">{t('onboarding.welcome')}</h1>
        <p className="text-muted-foreground">{t('onboarding.welcomeSubtitle')}</p>
      </div>

      <div className="grid md:grid-cols-2 gap-4 max-w-2xl mx-auto">
        {/* Watched Folders Option */}
        <button
          onClick={() => setSelected('watched')}
          className={`p-6 rounded-xl border-2 text-left transition-all ${
            selected === 'watched'
              ? 'border-primary bg-primary/5'
              : 'border-border hover:border-primary/50'
          }`}
        >
          <div className="flex items-start justify-between mb-4">
            <FolderOpen className="w-8 h-8 text-primary" />
            <div
              className={`w-5 h-5 rounded-full border-2 flex items-center justify-center ${
                selected === 'watched' ? 'border-primary bg-primary' : 'border-muted-foreground'
              }`}
            >
              {selected === 'watched' && <Check className="w-3 h-3 text-primary-foreground" />}
            </div>
          </div>
          <h3 className="font-semibold mb-2">{t('onboarding.useExistingFolders')}</h3>
          <p className="text-sm text-muted-foreground mb-4">
            {t('onboarding.useExistingFoldersDescription')}
          </p>
          <ul className="text-sm space-y-1">
            <li className="flex items-center gap-2 text-muted-foreground">
              <Check className="w-4 h-4 text-green-500 flex-shrink-0" />
              {t('onboarding.nonDestructive')}
            </li>
            <li className="flex items-center gap-2 text-muted-foreground">
              <Check className="w-4 h-4 text-green-500 flex-shrink-0" />
              {t('onboarding.keepsStructure')}
            </li>
          </ul>
        </button>

        {/* Managed Library Option */}
        <button
          onClick={() => setSelected('managed')}
          className={`p-6 rounded-xl border-2 text-left transition-all ${
            selected === 'managed'
              ? 'border-primary bg-primary/5'
              : 'border-border hover:border-primary/50'
          }`}
        >
          <div className="flex items-start justify-between mb-4">
            <FolderInput className="w-8 h-8 text-primary" />
            <div
              className={`w-5 h-5 rounded-full border-2 flex items-center justify-center ${
                selected === 'managed' ? 'border-primary bg-primary' : 'border-muted-foreground'
              }`}
            >
              {selected === 'managed' && <Check className="w-3 h-3 text-primary-foreground" />}
            </div>
          </div>
          <h3 className="font-semibold mb-2">{t('onboarding.letSoulOrganize')}</h3>
          <p className="text-sm text-muted-foreground mb-4">
            {t('onboarding.letSoulOrganizeDescription')}
          </p>
          <ul className="text-sm space-y-1">
            <li className="flex items-center gap-2 text-muted-foreground">
              <Check className="w-4 h-4 text-green-500 flex-shrink-0" />
              {t('onboarding.autoOrganized')}
            </li>
            <li className="flex items-center gap-2 text-muted-foreground">
              <Check className="w-4 h-4 text-green-500 flex-shrink-0" />
              {t('onboarding.easyImport')}
            </li>
          </ul>
        </button>
      </div>

      {/* Both option */}
      <div className="flex justify-center">
        <label className="flex items-center gap-3 cursor-pointer">
          <div
            className={`w-5 h-5 rounded border-2 flex items-center justify-center ${
              selected === 'both' ? 'border-primary bg-primary' : 'border-muted-foreground'
            }`}
            onClick={() => setSelected(selected === 'both' ? null : 'both')}
          >
            {selected === 'both' && <Check className="w-3 h-3 text-primary-foreground" />}
          </div>
          <span className="text-sm text-muted-foreground">{t('onboarding.useBoth')}</span>
        </label>
      </div>

      {/* Navigation */}
      <div className="flex items-center justify-between">
        <button
          onClick={onBack}
          className="flex items-center gap-2 text-muted-foreground hover:text-foreground transition-colors"
        >
          <ArrowLeft className="w-4 h-4" />
          {t('common.back')}
        </button>

        <div className="flex items-center gap-4">
          <button onClick={onSkip} className="text-sm text-muted-foreground hover:text-foreground">
            {t('onboarding.changeLaterSkip')}
          </button>
          <button
            onClick={() => selected && onStrategySelect(selected)}
            disabled={!selected}
            className="flex items-center gap-2 px-6 py-2 bg-primary text-primary-foreground rounded-lg hover:bg-primary/90 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
          >
            {t('common.continue')}
            <ArrowRight className="w-4 h-4" />
          </button>
        </div>
      </div>
    </div>
  );
}

// Setup Step Component
interface SetupStepProps {
  setupType: SetupType;
  watchedFolders: { name: string; path: string }[];
  libraryPath: string;
  pathTemplate: string;
  onBrowseFolder: () => void;
  onRemoveFolder: (index: number) => void;
  onBrowseLibraryPath: () => void;
  onLibraryPathChange: (value: string) => void;
  onPathTemplateChange: (value: string) => void;
  onBack: () => void;
  onComplete: () => void;
  canProceed: boolean;
  loading: boolean;
  t: (key: string) => string;
}

function SetupStep({
  setupType,
  watchedFolders,
  libraryPath,
  pathTemplate,
  onBrowseFolder,
  onRemoveFolder,
  onBrowseLibraryPath,
  onLibraryPathChange,
  onPathTemplateChange,
  onBack,
  onComplete,
  canProceed,
  loading,
  t,
}: SetupStepProps) {
  const showWatched = setupType === 'watched' || setupType === 'both';
  const showManaged = setupType === 'managed' || setupType === 'both';

  return (
    <div className="space-y-8">
      {showWatched && (
        <section className="space-y-4">
          <div className="text-center">
            <h2 className="text-2xl font-bold">{t('onboarding.addMusicFolders')}</h2>
            <p className="text-muted-foreground mt-1">
              {t('onboarding.addMusicFoldersDescription')}
            </p>
          </div>

          {/* Folder list */}
          {watchedFolders.length > 0 && (
            <div className="space-y-2 max-w-lg mx-auto">
              {watchedFolders.map((folder, index) => (
                <div
                  key={index}
                  className="flex items-center gap-3 p-3 bg-muted/30 rounded-lg"
                >
                  <FolderOpen className="w-5 h-5 text-primary flex-shrink-0" />
                  <div className="flex-1 min-w-0">
                    <p className="font-medium truncate">{folder.name}</p>
                    <p className="text-sm text-muted-foreground truncate">{folder.path}</p>
                  </div>
                  <button
                    onClick={() => onRemoveFolder(index)}
                    className="p-2 text-muted-foreground hover:text-destructive transition-colors"
                  >
                    <Trash2 className="w-4 h-4" />
                  </button>
                </div>
              ))}
            </div>
          )}

          {/* Add folder button */}
          <div className="flex justify-center">
            <button
              onClick={onBrowseFolder}
              className="flex items-center gap-2 px-6 py-3 border-2 border-dashed border-border rounded-lg hover:border-primary hover:bg-primary/5 transition-colors"
            >
              <Plus className="w-5 h-5" />
              {t('onboarding.browseForFolder')}
            </button>
          </div>
        </section>
      )}

      {showManaged && (
        <section className="space-y-4">
          <div className="text-center">
            <h2 className="text-2xl font-bold">{t('onboarding.chooseLibraryLocation')}</h2>
            <p className="text-muted-foreground mt-1">
              {t('onboarding.chooseLibraryLocationDescription')}
            </p>
          </div>

          <div className="max-w-lg mx-auto space-y-4">
            <div>
              <label className="block text-sm font-medium mb-2">
                {t('librarySettings.libraryPath')}
              </label>
              <div className="flex gap-2">
                <input
                  type="text"
                  value={libraryPath}
                  onChange={(e) => onLibraryPathChange(e.target.value)}
                  className="flex-1 px-3 py-2 rounded-lg bg-muted border border-border"
                  readOnly
                />
                <button
                  onClick={onBrowseLibraryPath}
                  className="px-4 py-2 bg-muted rounded-lg hover:bg-muted/80 transition-colors"
                >
                  {t('onboarding.browse')}
                </button>
              </div>
            </div>

            <div>
              <label className="block text-sm font-medium mb-2">
                {t('onboarding.organizationStyle')}
              </label>
              <div className="space-y-2">
                <label className="flex items-center gap-3 p-3 rounded-lg bg-muted/30 cursor-pointer hover:bg-muted/50 transition-colors">
                  <input
                    type="radio"
                    checked={pathTemplate === '{AlbumArtist}/{Year} - {Album}/{TrackNo} - {Title}'}
                    onChange={() =>
                      onPathTemplateChange('{AlbumArtist}/{Year} - {Album}/{TrackNo} - {Title}')
                    }
                    className="w-4 h-4"
                  />
                  <div className="flex-1">
                    <p className="font-medium text-sm">{t('librarySettings.preset.audiophile')}</p>
                    <p className="text-xs text-muted-foreground font-mono">
                      Artist/1977 - Album/01 - Track.flac
                    </p>
                  </div>
                </label>
                <label className="flex items-center gap-3 p-3 rounded-lg bg-muted/30 cursor-pointer hover:bg-muted/50 transition-colors">
                  <input
                    type="radio"
                    checked={pathTemplate === '{AlbumArtist}/{Album}/{TrackNo} - {Title}'}
                    onChange={() =>
                      onPathTemplateChange('{AlbumArtist}/{Album}/{TrackNo} - {Title}')
                    }
                    className="w-4 h-4"
                  />
                  <div className="flex-1">
                    <p className="font-medium text-sm">{t('librarySettings.preset.simple')}</p>
                    <p className="text-xs text-muted-foreground font-mono">
                      Artist/Album/01 - Track.flac
                    </p>
                  </div>
                </label>
              </div>
            </div>
          </div>
        </section>
      )}

      {/* Navigation */}
      <div className="flex items-center justify-between pt-4">
        <button
          onClick={onBack}
          className="flex items-center gap-2 text-muted-foreground hover:text-foreground transition-colors"
        >
          <ArrowLeft className="w-4 h-4" />
          {t('common.back')}
        </button>

        <button
          onClick={onComplete}
          disabled={!canProceed || loading}
          className="flex items-center gap-2 px-6 py-2 bg-primary text-primary-foreground rounded-lg hover:bg-primary/90 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
        >
          {loading ? (
            <Loader2 className="w-4 h-4 animate-spin" />
          ) : (
            <>
              {t('onboarding.finish')}
              <ArrowRight className="w-4 h-4" />
            </>
          )}
        </button>
      </div>
    </div>
  );
}

// Complete Step Component
function CompleteStep({ t }: { t: (key: string) => string }) {
  return (
    <div className="text-center space-y-6 py-12">
      <div className="flex justify-center">
        <div className="w-16 h-16 rounded-full bg-green-500/10 flex items-center justify-center">
          <Check className="w-8 h-8 text-green-500" />
        </div>
      </div>
      <div className="space-y-2">
        <h2 className="text-2xl font-bold">{t('onboarding.allSet')}</h2>
        <p className="text-muted-foreground">{t('onboarding.allSetDescription')}</p>
      </div>
      <Loader2 className="w-6 h-6 animate-spin mx-auto text-muted-foreground" />
    </div>
  );
}
