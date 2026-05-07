<script lang="ts">
  import './CalendarPanelSurface.css';
  import { onMount } from 'svelte';
  import {
    addCalendarMonths,
    calendarMonthModel,
    formatCalendarLongDate,
    formatCalendarTimezone
  } from '../features/top-bar/topBarUxState';
  import { formatShellTime, getInitialShellPreferences, type ShellPreferences } from '../lib/shellPreferences';
  import { hideCalendarPanel } from '../lib/calendarPanel';
  import MeltActionButton from './melt/MeltActionButton.svelte';

  let now = new Date();
  let viewDate = new Date(now.getFullYear(), now.getMonth(), 1);
  let selectedDate = now;
  let shellPreferences: ShellPreferences = getInitialShellPreferences();

  $: calendarModel = calendarMonthModel(viewDate, { selectedDate, today: now });
  $: selectedLongDate = formatCalendarLongDate(selectedDate);
  $: timezoneLabel = formatCalendarTimezone(now);
  $: panelTime = formatShellTime(now, shellPreferences);

  function jumpCalendarMonths(monthDelta: number) {
    viewDate = addCalendarMonths(viewDate, monthDelta);
  }

  function selectCalendarDate(date: Date) {
    selectedDate = date;
    viewDate = new Date(date.getFullYear(), date.getMonth(), 1);
  }

  function handleCalendarWheel(event: WheelEvent) {
    event.preventDefault();
    jumpCalendarMonths(event.deltaY > 0 ? 1 : -1);
  }

  function handleCalendarKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      event.preventDefault();
      void hideCalendarPanel();
    }
  }

  onMount(() => {
    const timer = window.setInterval(() => {
      now = new Date();
    }, 1_000);
    return () => window.clearInterval(timer);
  });
</script>

<div
  id="calendar-panel"
  class="calendar-panel"
  role="dialog"
  aria-label="Calendar and time"
  tabindex="0"
  on:keydown={handleCalendarKeydown}
  on:wheel={handleCalendarWheel}
>
  <header class="calendar-panel-header">
    <div>
      <strong>{panelTime}</strong>
      <span>{selectedLongDate}</span>
    </div>
    <MeltActionButton class="calendar-close" ariaLabel="Close calendar" tooltip="Close calendar" onClick={() => void hideCalendarPanel()}>×</MeltActionButton>
  </header>

  <div class="calendar-nav" aria-label="Calendar navigation">
    <MeltActionButton class="calendar-nav-button" ariaLabel="Previous year" tooltip="Previous year" onClick={() => jumpCalendarMonths(-12)}>«</MeltActionButton>
    <MeltActionButton class="calendar-nav-button" ariaLabel="Previous month" tooltip="Previous month" onClick={() => jumpCalendarMonths(-1)}>‹</MeltActionButton>
    <strong>{calendarModel.monthLabel}</strong>
    <MeltActionButton class="calendar-nav-button" ariaLabel="Next month" tooltip="Next month" onClick={() => jumpCalendarMonths(1)}>›</MeltActionButton>
    <MeltActionButton class="calendar-nav-button" ariaLabel="Next year" tooltip="Next year" onClick={() => jumpCalendarMonths(12)}>»</MeltActionButton>
  </div>

  <div class="calendar-grid" role="grid" aria-label={calendarModel.monthLabel}>
    {#each calendarModel.weekdayLabels as weekday}
      <div class="calendar-weekday" role="columnheader">{weekday}</div>
    {/each}
    {#each calendarModel.weeks as week}
      {#each week as day}
        <button
          type="button"
          class:outside={!day.inCurrentMonth}
          class:today={day.isToday}
          class:selected={day.isSelected}
          aria-current={day.isToday ? 'date' : undefined}
          aria-pressed={day.isSelected}
          on:click={() => selectCalendarDate(day.date)}
        >
          {day.day}
        </button>
      {/each}
    {/each}
  </div>

  <footer class="calendar-info">
    <span>{timezoneLabel}</span>
    <span>Weekdays shown for local calendar</span>
  </footer>
</div>
