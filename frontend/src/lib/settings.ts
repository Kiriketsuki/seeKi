import type {
  AppearanceSettings,
  BrandingSettings,
  DateFormatPreference,
  DisplayConfig,
  RowDensityPreference,
  SettingsEntries,
} from './types';

export const DEFAULT_APPEARANCE_SETTINGS: AppearanceSettings = {
  dateFormat: 'system',
  rowDensity: 'comfortable',
};

export function parseBrandingSettings(
  settings: SettingsEntries,
  displayConfig: DisplayConfig | null,
): BrandingSettings {
  const fallbackTitle = displayConfig?.branding.title ?? 'SeeKi';
  const fallbackSubtitle = displayConfig?.branding.subtitle ?? '';
  const title = typeof settings['branding.title'] === 'string'
    ? settings['branding.title'].trim()
    : fallbackTitle;
  const subtitle = typeof settings['branding.subtitle'] === 'string'
    ? settings['branding.subtitle']
    : fallbackSubtitle;

  return {
    title: title.length > 0 ? title : fallbackTitle,
    subtitle,
  };
}

export function parseAppearanceSettings(settings: SettingsEntries): AppearanceSettings {
  const dateFormat = isDateFormatPreference(settings['appearance.date_format'])
    ? settings['appearance.date_format']
    : DEFAULT_APPEARANCE_SETTINGS.dateFormat;
  const rowDensity = isRowDensityPreference(settings['appearance.row_density'])
    ? settings['appearance.row_density']
    : DEFAULT_APPEARANCE_SETTINGS.rowDensity;

  return {
    dateFormat,
    rowDensity,
  };
}

export function buildBrandingSettingsEntries(
  branding: BrandingSettings,
): SettingsEntries {
  return {
    'branding.title': branding.title.trim(),
    'branding.subtitle': branding.subtitle.trim(),
  };
}

export function buildAppearanceSettingsEntries(
  appearance: AppearanceSettings,
): SettingsEntries {
  return {
    'appearance.date_format': appearance.dateFormat,
    'appearance.row_density': appearance.rowDensity,
  };
}

export function isDateFormatPreference(
  value: unknown,
): value is DateFormatPreference {
  return value === 'system'
    || value === 'YYYY-MM-DD'
    || value === 'DD/MM/YYYY'
    || value === 'MM/DD/YYYY';
}

export function isRowDensityPreference(
  value: unknown,
): value is RowDensityPreference {
  return value === 'comfortable' || value === 'compact';
}
