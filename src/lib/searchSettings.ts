export const SEARCH_MODES = ['topRight', 'centeredHotkey'] as const;
export type SearchMode = (typeof SEARCH_MODES)[number];

export const EVERYTHING_INSTALL_MODES = ['ask', 'disabled', 'managed'] as const;
export type EverythingInstallMode = (typeof EVERYTHING_INSTALL_MODES)[number];

export const EVERYTHING_SDK_SOURCES = ['bundled', 'system'] as const;
export type EverythingSdkSource = (typeof EVERYTHING_SDK_SOURCES)[number];

export const EVERYTHING_SORT_MODES = ['nameAsc', 'pathAsc', 'dateModifiedDesc', 'runCountDesc'] as const;
export type EverythingSortMode = (typeof EVERYTHING_SORT_MODES)[number];

export const EVERYTHING_SETUP_ACTIONS = [
  'launchInstalled',
  'downloadInstaller',
  'runBundledInstaller',
  'openOfficialDownload'
] as const;
export type EverythingSetupAction = (typeof EVERYTHING_SETUP_ACTIONS)[number];

export const EVERYTHING_SETUP_STATUSES = ['declined', 'launched', 'installed', 'blocked', 'failed'] as const;
export type EverythingSetupStatus = (typeof EVERYTHING_SETUP_STATUSES)[number];

export type SearchSettingsContract = {
  ui: {
    searchMode: SearchMode;
  };
  search: {
    resultLimit: number;
    everything: {
      enabled: boolean;
      installMode: EverythingInstallMode;
      sdkSource: EverythingSdkSource;
      maxResults: number;
      fullPathSearch: boolean;
      sort: EverythingSortMode;
      contentSearchEnabled: boolean;
    };
  };
};

export type EverythingSetupConsentRequest = {
  action: EverythingSetupAction;
  consent: boolean;
  officialUrl: string;
  artifactName?: string;
  version?: string;
  sha256?: string;
  licenseApproved: boolean;
  provenanceApproved: boolean;
  requiresAdmin: boolean;
  explainsFilenameExposure: boolean;
};

export type EverythingSetupResult = {
  status: EverythingSetupStatus;
  health: unknown;
  reasonCode?: string;
  message: string;
};

export const DEFAULT_SEARCH_SETTINGS: SearchSettingsContract = {
    ui: {
      searchMode: 'centeredHotkey'
    },
    search: {
    resultLimit: 50,
    everything: {
      enabled: true,
      installMode: 'ask',
      sdkSource: 'system',
      maxResults: 100,
      fullPathSearch: true,
      sort: 'nameAsc',
      contentSearchEnabled: false
    }
  }
};

const RESULT_LIMIT_MIN = 1;
const RESULT_LIMIT_MAX = 100;
const EVERYTHING_MAX_RESULTS_MIN = 1;
const EVERYTHING_MAX_RESULTS_MAX = 200;

export function defaultSearchSettings(): SearchSettingsContract {
  return cloneSearchSettings(DEFAULT_SEARCH_SETTINGS);
}

export function coerceSearchSettings(value: unknown): SearchSettingsContract {
  const source = asRecord(value);
  const ui = asRecord(source?.ui);
  const search = asRecord(source?.search);
  const everything = asRecord(search?.everything);
  const defaults = DEFAULT_SEARCH_SETTINGS;

  return {
    ui: {
      searchMode: enumValue(SEARCH_MODES, ui?.searchMode, defaults.ui.searchMode)
    },
    search: {
      resultLimit: boundedInteger(
        search?.resultLimit,
        RESULT_LIMIT_MIN,
        RESULT_LIMIT_MAX,
        defaults.search.resultLimit
      ),
      everything: {
        enabled: booleanValue(everything?.enabled, defaults.search.everything.enabled),
        installMode: enumValue(
          EVERYTHING_INSTALL_MODES,
          everything?.installMode,
          defaults.search.everything.installMode
        ),
        sdkSource: enumValue(
          EVERYTHING_SDK_SOURCES,
          everything?.sdkSource,
          defaults.search.everything.sdkSource
        ),
        maxResults: boundedInteger(
          everything?.maxResults,
          EVERYTHING_MAX_RESULTS_MIN,
          EVERYTHING_MAX_RESULTS_MAX,
          defaults.search.everything.maxResults
        ),
        fullPathSearch: booleanValue(
          everything?.fullPathSearch,
          defaults.search.everything.fullPathSearch
        ),
        sort: enumValue(EVERYTHING_SORT_MODES, everything?.sort, defaults.search.everything.sort),
        contentSearchEnabled: booleanValue(
          everything?.contentSearchEnabled,
          defaults.search.everything.contentSearchEnabled
        )
      }
    }
  };
}

export function isEverythingSetupConsentAllowed(request: EverythingSetupConsentRequest): boolean {
  if (!EVERYTHING_SETUP_ACTIONS.includes(request.action)) {
    return false;
  }
  if (!request.consent) {
    return false;
  }
  if (!request.explainsFilenameExposure) {
    return false;
  }
  if (request.action === 'openOfficialDownload') {
    return isOfficialVoidtoolsUrl(request.officialUrl);
  }
  return (
    isOfficialVoidtoolsUrl(request.officialUrl) &&
    nonEmptyString(request.artifactName) &&
    nonEmptyString(request.version) &&
    isSha256(request.sha256) &&
    request.licenseApproved &&
    request.provenanceApproved
  );
}

function cloneSearchSettings(settings: SearchSettingsContract): SearchSettingsContract {
  return {
    ui: { ...settings.ui },
    search: {
      resultLimit: settings.search.resultLimit,
      everything: { ...settings.search.everything }
    }
  };
}

function enumValue<const T extends readonly string[]>(values: T, value: unknown, fallback: T[number]): T[number] {
  return typeof value === 'string' && values.includes(value) ? value : fallback;
}

function booleanValue(value: unknown, fallback: boolean): boolean {
  return typeof value === 'boolean' ? value : fallback;
}

function boundedInteger(value: unknown, min: number, max: number, fallback: number): number {
  return typeof value === 'number' && Number.isInteger(value) && value >= min && value <= max
    ? value
    : fallback;
}

function asRecord(value: unknown): Record<string, unknown> | null {
  return value && typeof value === 'object' && !Array.isArray(value)
    ? (value as Record<string, unknown>)
    : null;
}

function isOfficialVoidtoolsUrl(value: string): boolean {
  try {
    const url = new URL(value);
    return url.protocol === 'https:' && url.hostname === 'www.voidtools.com';
  } catch {
    return false;
  }
}

function isSha256(value: unknown): boolean {
  return typeof value === 'string' && /^[a-f0-9]{64}$/iu.test(value);
}

function nonEmptyString(value: unknown): boolean {
  return typeof value === 'string' && value.trim().length > 0;
}
