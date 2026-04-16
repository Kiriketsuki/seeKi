import { describe, expect, it } from 'vitest';
import {
  buildAppearanceSettingsEntries,
  buildBrandingSettingsEntries,
  parseAppearanceSettings,
  parseBrandingSettings,
} from './settings';
import type { DisplayConfig } from './types';

const displayConfig: DisplayConfig = {
  branding: {
    title: 'SeeKi',
    subtitle: 'Database Viewer',
  },
  tables: {},
};

describe('parseBrandingSettings', () => {
  it('uses saved branding values when present', () => {
    const result = parseBrandingSettings(
      {
        'branding.title': 'Fleet DB',
        'branding.subtitle': 'Operations',
      },
      displayConfig,
    );

    expect(result).toEqual({
      title: 'Fleet DB',
      subtitle: 'Operations',
    });
  });

  it('falls back to display config branding when settings are absent', () => {
    expect(parseBrandingSettings({}, displayConfig)).toEqual({
      title: 'SeeKi',
      subtitle: 'Database Viewer',
    });
  });
});

describe('parseAppearanceSettings', () => {
  it('returns supported saved appearance values', () => {
    expect(
      parseAppearanceSettings({
        'appearance.date_format': 'DD/MM/YYYY',
        'appearance.row_density': 'compact',
      }),
    ).toEqual({
      dateFormat: 'DD/MM/YYYY',
      rowDensity: 'compact',
    });
  });

  it('falls back to defaults for invalid values', () => {
    expect(
      parseAppearanceSettings({
        'appearance.date_format': 'RFC3339',
        'appearance.row_density': 'dense',
      }),
    ).toEqual({
      dateFormat: 'system',
      rowDensity: 'comfortable',
    });
  });
});

describe('settings entry builders', () => {
  it('serializes branding entries', () => {
    expect(
      buildBrandingSettingsEntries({
        title: ' Fleet DB ',
        subtitle: ' Ops ',
      }),
    ).toEqual({
      'branding.title': 'Fleet DB',
      'branding.subtitle': 'Ops',
    });
  });

  it('serializes appearance entries', () => {
    expect(
      buildAppearanceSettingsEntries({
        dateFormat: 'YYYY-MM-DD',
        rowDensity: 'compact',
      }),
    ).toEqual({
      'appearance.date_format': 'YYYY-MM-DD',
      'appearance.row_density': 'compact',
    });
  });
});
