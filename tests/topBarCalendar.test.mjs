import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import {
  addCalendarMonths,
  calendarMonthModel,
  formatCalendarLongDate,
  formatCalendarTimezone
} from '../dist-tests/features/top-bar/topBarUxState.js';

const topBarSource = readFileSync(new URL('../src/components/TopBar.svelte', import.meta.url), 'utf8');
const topBarCss = readFileSync(new URL('../src/components/TopBar.css', import.meta.url), 'utf8');
const calendarPanelSource = readFileSync(new URL('../src/components/CalendarPanelSurface.svelte', import.meta.url), 'utf8');
const calendarPanelCss = readFileSync(new URL('../src/components/CalendarPanelSurface.css', import.meta.url), 'utf8');
const calendarPanelWrapper = readFileSync(new URL('../src/lib/calendarPanel.ts', import.meta.url), 'utf8');
const appSource = readFileSync(new URL('../src/App.svelte', import.meta.url), 'utf8');
const surfaceLoaderSource = readFileSync(new URL('../src/lib/surfaceLoader.ts', import.meta.url), 'utf8');
const shellSurfaceSource = readFileSync(new URL('../src/lib/shellSurface.ts', import.meta.url), 'utf8');
const shellWindowsSource = readFileSync(new URL('../src-tauri/src/shell_windows.rs', import.meta.url), 'utf8');
const mainRustSource = readFileSync(new URL('../src-tauri/src/main.rs', import.meta.url), 'utf8');
const calendarPanelRustSource = readFileSync(new URL('../src-tauri/src/calendar_panel.rs', import.meta.url), 'utf8');

test('calendar month model builds a Sunday-first six-week grid with adjacent month days', () => {
  const model = calendarMonthModel(new Date(2026, 4, 7), {
    selectedDate: new Date(2026, 4, 7),
    today: new Date(2026, 4, 7),
    locale: 'en-US'
  });

  assert.equal(model.monthLabel, 'May 2026');
  assert.deepEqual(model.weekdayLabels, ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat']);
  assert.equal(model.weeks.length, 6);
  assert.equal(model.weeks[0].length, 7);
  assert.equal(model.weeks[0][0].key, '2026-04-26');
  assert.equal(model.weeks[0][5].key, '2026-05-01');
  assert.equal(model.weeks[1][4].key, '2026-05-07');
  assert.equal(model.weeks[1][4].isToday, true);
  assert.equal(model.weeks[1][4].isSelected, true);
  assert.equal(model.weeks[0][0].inCurrentMonth, false);
});

test('calendar month navigation supports other years without losing weekday labels', () => {
  const nextYear = addCalendarMonths(new Date(2026, 4, 7), 12);
  const model = calendarMonthModel(nextYear, {
    selectedDate: new Date(2026, 4, 7),
    today: new Date(2026, 4, 7),
    locale: 'en-US'
  });

  assert.equal(model.monthLabel, 'May 2027');
  assert.equal(model.year, 2027);
  assert.deepEqual(model.weekdayLabels, ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat']);
});

test('calendar labels expose long date and local timezone information', () => {
  assert.equal(formatCalendarLongDate(new Date(2026, 4, 7), 'en-US'), 'Thursday, May 7, 2026');
  assert.match(formatCalendarTimezone(new Date(2026, 4, 7), 'en-US'), /UTC[+-]\d{2}:\d{2}/);
});

test('top bar time pill owns an Explorer-like scrollable calendar flyout', () => {
  assert.match(topBarSource, /toggleCalendarPanel/);
  assert.match(topBarSource, /class="time-pill"[\s\S]*ariaHaspopup="dialog"[\s\S]*ariaExpanded=\{calendarOpen\}/);
  assert.match(topBarSource, /showCalendarPanel\(\{/);
  assert.match(topBarSource, /hideCalendarPanel\(\)/);
  assert.match(topBarSource, /CALENDAR_PANEL_CLOSED_EVENT/);
  assert.match(topBarCss, /\.top-bar \.time-control \{[\s\S]*flex: 0 0 10\.5rem;/);
  assert.match(topBarCss, /\.top-bar \.time-pill \{[\s\S]*width: 100%;/);
  assert.match(calendarPanelSource, /id="calendar-panel"[\s\S]*role="dialog"/);
  assert.match(calendarPanelSource, /on:wheel=\{handleCalendarWheel\}/);
  assert.match(calendarPanelSource, /jumpCalendarMonths\(-12\)/);
  assert.match(calendarPanelSource, /jumpCalendarMonths\(12\)/);
  assert.match(calendarPanelSource, /formatCalendarTimezone/);
  assert.match(calendarPanelCss, /\.calendar-panel/);
  assert.match(calendarPanelCss, /overflow-y: auto/);
  assert.match(topBarCss, /\.top-bar \.time-control \{[\s\S]*flex: 0 0 10\.5rem;[\s\S]*width: 10\.5rem;/);
  assert.match(topBarCss, /@media \(max-width: 520px\) \{[\s\S]*\.top-bar \.time-control \{[\s\S]*flex-basis: 4\.8rem;[\s\S]*width: 4\.8rem;/);
  assert.doesNotMatch(`${topBarCss}\n${calendarPanelCss}`, /linear-gradient|radial-gradient|conic-gradient/);
});

test('calendar panel is a dedicated top-bar anchored webview so it is not clipped by the compact bar', () => {
  assert.match(appSource, /loadSurfaceComponent\(surface\)/);
  assert.match(surfaceLoaderSource, /'calendar-panel': \(\) => import\('\.\.\/components\/CalendarPanelSurface\.svelte'\)/);
  assert.match(shellSurfaceSource, /\| 'calendar-panel'/);
  assert.match(calendarPanelWrapper, /showCalendarPanel/);
  assert.match(calendarPanelWrapper, /hideCalendarPanel/);
  assert.match(shellWindowsSource, /pub const CALENDAR_PANEL_LABEL: &str = "calendar-panel"/);
  assert.match(shellWindowsSource, /CALENDAR_PANEL_HEIGHT_LOGICAL: f64 = 430\.0/);
  assert.match(shellWindowsSource, /build_calendar_panel_window\(app\)/);
  assert.match(mainRustSource, /mod calendar_panel;/);
  assert.match(mainRustSource, /calendar_panel::show_calendar_panel/);
  assert.match(mainRustSource, /calendar_panel::hide_calendar_panel/);
  assert.match(mainRustSource, /shell_windows::CALENDAR_PANEL_LABEL[\s\S]*WindowEvent::Focused\(false\)/);
  assert.match(calendarPanelRustSource, /pub fn show_calendar_panel/);
  assert.match(calendarPanelRustSource, /TOP_BAR_LABEL/);
  assert.match(calendarPanelRustSource, /emit_to\(TOP_BAR_LABEL, CALENDAR_PANEL_CLOSED_EVENT/);
});
