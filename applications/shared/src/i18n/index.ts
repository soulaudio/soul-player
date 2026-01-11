import i18n from 'i18next';
import { initReactI18next } from 'react-i18next';
import enUS from './en-US.json';
import de from './de.json';
import ja from './ja.json';

// Flag to track if i18n has been initialized
let initialized = false;

/**
 * Initialize i18n with react-i18next.
 * Safe to call multiple times - will only initialize once.
 */
export function initI18n() {
  if (initialized) {
    return i18n;
  }

  i18n
    .use(initReactI18next)
    .init({
      resources: {
        'en-US': { translation: enUS },
        'de': { translation: de },
        'ja': { translation: ja },
      },
      lng: 'en-US',
      fallbackLng: 'en-US',
      interpolation: {
        escapeValue: false, // React already escapes values
      },
    });

  initialized = true;
  return i18n;
}

// Export the i18n instance for direct usage
export default i18n;

// Re-export commonly used hooks and functions from react-i18next
export { useTranslation, Trans, I18nextProvider } from 'react-i18next';
