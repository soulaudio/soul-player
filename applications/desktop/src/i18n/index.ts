import i18n from 'i18next';
import { initReactI18next } from 'react-i18next';
import enUS from './en-US.json';
import de from './de.json';
import ja from './ja.json';

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

export default i18n;
