import type {
  AppearanceSettings,
  BrandingSettings,
  DataSettings,
  DateFormatPreference,
  DisplayConfig,
  PageSizePreference,
  PaginationMode,
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

export const DEFAULT_DATA_SETTINGS: DataSettings = {
  pageSize: 50,
  paginationMode: 'infinite',
};

export function isPageSizePreference(value: unknown): value is PageSizePreference {
  return value === 50 || value === 100 || value === 250 || value === 500;
}

export function isPaginationMode(value: unknown): value is PaginationMode {
  return value === 'infinite' || value === 'paged';
}

export function parseDataSettings(settings: SettingsEntries): DataSettings {
  const rawValue = settings['data.page_size'];
  const numericValue = typeof rawValue === 'string' ? Number(rawValue) : rawValue;
  const pageSize: PageSizePreference = isPageSizePreference(numericValue)
    ? numericValue
    : DEFAULT_DATA_SETTINGS.pageSize;

  const paginationMode = isPaginationMode(settings['data.pagination_mode'])
    ? settings['data.pagination_mode']
    : DEFAULT_DATA_SETTINGS.paginationMode;

  return { pageSize, paginationMode };
}

export function buildDataSettingsEntries(data: DataSettings): SettingsEntries {
  return {
    'data.page_size': data.pageSize,
    'data.pagination_mode': data.paginationMode,
  };
}
