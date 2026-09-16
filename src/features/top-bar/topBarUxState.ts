export type TopBarIdentityState = {
  placesLabel: string;
  programsLabel: string;
  indexLabel: string;
  workspaceLabel: string;
};

export function shouldAnimateTerminalCommand(commandText: string): boolean {
  const command = commandText.trim().toLocaleLowerCase();
  if (!command) return false;

  const firstCommand = command.split(/\s*(?:&&|\|\||;)\s*/)[0] ?? command;
  const executable = firstCommand.match(/^(?:\w+=\S+\s+)*(?:bunx|npx|pnpm|yarn|npm\s+exec|uvx|cargo\s+run\s+--)?\s*([.\w:/\\-]+)/)?.[1] ?? firstCommand;
  const basename = executable.replace(/\\/g, '/').split('/').pop() ?? executable;

  return (
    /^(?:mvn|mvnw)$/.test(basename) ||
    /^(?:pi|codex|claude|gemini|aider|opencode)$/.test(basename) ||
    /\b(?:mvn|mvnw)\s+(?:clean\s+)?(?:install|package|verify|test|deploy)\b/.test(command)
  );
}

export type CalendarDayCell = {
  key: string;
  day: number;
  date: Date;
  inCurrentMonth: boolean;
  isToday: boolean;
  isSelected: boolean;
};

export type CalendarMonthModel = {
  monthLabel: string;
  year: number;
  weekdayLabels: string[];
  weeks: CalendarDayCell[][];
};

export type CalendarWheelNavigationState = {
  accumulatedDelta: number;
  direction: -1 | 0 | 1;
  lastInputTime: number | null;
  lastNavigationTime: number | null;
};

export type CalendarWheelNavigationInput = {
  deltaX: number;
  deltaY: number;
  deltaMode: number;
  timeStamp: number;
};

export type CalendarWheelNavigationResult = {
  state: CalendarWheelNavigationState;
  monthDelta: -1 | 0 | 1;
  consumed: boolean;
  preventDefault: boolean;
};

const CALENDAR_WHEEL_PIXEL_THRESHOLD = 100;
const CALENDAR_WHEEL_LINE_PIXELS = 40;
const CALENDAR_WHEEL_PAGE_PIXELS = 800;
const CALENDAR_WHEEL_THROTTLE_MS = 500;
const CALENDAR_WHEEL_IDLE_RESET_MS = 700;

export function createCalendarWheelNavigationState(): CalendarWheelNavigationState {
  return { accumulatedDelta: 0, direction: 0, lastInputTime: null, lastNavigationTime: null };
}

export function reduceCalendarWheelNavigation(
  state: CalendarWheelNavigationState,
  input: CalendarWheelNavigationInput
): CalendarWheelNavigationResult {
  const modeScale = input.deltaMode === 1
    ? CALENDAR_WHEEL_LINE_PIXELS
    : input.deltaMode === 2
      ? CALENDAR_WHEEL_PAGE_PIXELS
      : 1;
  const deltaX = input.deltaX * modeScale;
  const deltaY = input.deltaY * modeScale;

  if (!Number.isFinite(deltaX) || !Number.isFinite(deltaY) || deltaY === 0 || Math.abs(deltaX) > Math.abs(deltaY)) {
    return { state, monthDelta: 0, consumed: false, preventDefault: false };
  }

  const direction: -1 | 1 = deltaY > 0 ? 1 : -1;
  const idleReset = state.lastInputTime !== null
    && input.timeStamp - state.lastInputTime > CALENDAR_WHEEL_IDLE_RESET_MS;
  const directionChanged = state.direction !== 0 && state.direction !== direction;
  const baseAccumulation = idleReset || directionChanged ? 0 : state.accumulatedDelta;
  const inCooldown = state.lastNavigationTime !== null
    && input.timeStamp - state.lastNavigationTime < CALENDAR_WHEEL_THROTTLE_MS;

  if (inCooldown) {
    return {
      state: { ...state, accumulatedDelta: 0, direction, lastInputTime: input.timeStamp },
      monthDelta: 0,
      consumed: true,
      preventDefault: true
    };
  }

  const accumulatedDelta = baseAccumulation + deltaY;
  if (Math.abs(accumulatedDelta) < CALENDAR_WHEEL_PIXEL_THRESHOLD) {
    return {
      state: { ...state, accumulatedDelta, direction, lastInputTime: input.timeStamp },
      monthDelta: 0,
      consumed: true,
      preventDefault: true
    };
  }

  return {
    state: {
      accumulatedDelta: 0,
      direction,
      lastInputTime: input.timeStamp,
      lastNavigationTime: input.timeStamp
    },
    monthDelta: direction,
    consumed: true,
    preventDefault: true
  };
}

export function topBarIdentityState(
  pinnedPlaceCount: number,
  programCount: number,
  searchStatus: string
): TopBarIdentityState {
  return {
    placesLabel: pinnedPlaceCount === 1 ? '1 place' : `${pinnedPlaceCount} places`,
    programsLabel: programCount === 1 ? '1 app' : `${programCount} apps`,
    indexLabel: searchStatus.toLocaleLowerCase().includes('searching')
      ? 'Indexing'
      : 'Index ready',
    workspaceLabel: 'Workspace later'
  };
}

export function calendarMonthModel(
  viewDate: Date,
  options: {
    selectedDate?: Date;
    today?: Date;
    locale?: string;
  } = {}
): CalendarMonthModel {
  const locale = options.locale;
  const selectedDate = startOfLocalDay(options.selectedDate ?? viewDate);
  const today = startOfLocalDay(options.today ?? new Date());
  const firstOfMonth = new Date(viewDate.getFullYear(), viewDate.getMonth(), 1);
  const gridStart = new Date(firstOfMonth);
  gridStart.setDate(firstOfMonth.getDate() - firstOfMonth.getDay());

  const weeks: CalendarDayCell[][] = [];
  for (let weekIndex = 0; weekIndex < 6; weekIndex += 1) {
    const week: CalendarDayCell[] = [];
    for (let dayIndex = 0; dayIndex < 7; dayIndex += 1) {
      const date = new Date(gridStart);
      date.setDate(gridStart.getDate() + (weekIndex * 7) + dayIndex);
      week.push({
        key: calendarDateKey(date),
        day: date.getDate(),
        date,
        inCurrentMonth: date.getMonth() === viewDate.getMonth(),
        isToday: sameLocalDay(date, today),
        isSelected: sameLocalDay(date, selectedDate)
      });
    }
    weeks.push(week);
  }

  return {
    monthLabel: new Intl.DateTimeFormat(locale, { month: 'long', year: 'numeric' }).format(firstOfMonth),
    year: firstOfMonth.getFullYear(),
    weekdayLabels: weekdayLabels(locale),
    weeks
  };
}

export function addCalendarMonths(date: Date, monthDelta: number): Date {
  return new Date(date.getFullYear(), date.getMonth() + monthDelta, 1);
}

export function calendarDateKey(date: Date): string {
  return [
    date.getFullYear(),
    `${date.getMonth() + 1}`.padStart(2, '0'),
    `${date.getDate()}`.padStart(2, '0')
  ].join('-');
}

export function formatCalendarLongDate(date: Date, locale?: string): string {
  return new Intl.DateTimeFormat(locale, {
    weekday: 'long',
    month: 'long',
    day: 'numeric',
    year: 'numeric'
  }).format(date);
}

export function formatCalendarTimezone(date: Date = new Date(), locale?: string): string {
  const timeZoneName = new Intl.DateTimeFormat(locale, { timeZoneName: 'long' })
    .formatToParts(date)
    .find((part) => part.type === 'timeZoneName')?.value ?? 'Local time';
  const offsetMinutes = -date.getTimezoneOffset();
  const sign = offsetMinutes >= 0 ? '+' : '-';
  const absoluteOffset = Math.abs(offsetMinutes);
  const hours = `${Math.floor(absoluteOffset / 60)}`.padStart(2, '0');
  const minutes = `${absoluteOffset % 60}`.padStart(2, '0');
  return `${timeZoneName} (UTC${sign}${hours}:${minutes})`;
}

function weekdayLabels(locale?: string): string[] {
  const sunday = new Date(2026, 0, 4);
  return Array.from({ length: 7 }, (_value, index) => {
    const date = new Date(sunday);
    date.setDate(sunday.getDate() + index);
    return new Intl.DateTimeFormat(locale, { weekday: 'short' }).format(date);
  });
}

function startOfLocalDay(date: Date): Date {
  return new Date(date.getFullYear(), date.getMonth(), date.getDate());
}

function sameLocalDay(left: Date, right: Date): boolean {
  return calendarDateKey(left) === calendarDateKey(right);
}
