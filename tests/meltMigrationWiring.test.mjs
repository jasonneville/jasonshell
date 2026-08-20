import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const packageJson = JSON.parse(readFileSync(new URL('../package.json', import.meta.url), 'utf8'));
function uncommentedSource(path) {
  return readFileSync(new URL(path, import.meta.url), 'utf8')
    .replace(/<!--[\s\S]*?-->/g, '')
    .replace(/\/\*[\s\S]*?\*\//g, '')
    .replace(/(^|[^:])\/\/.*$/gm, '$1');
}

const meltSelectSource = uncommentedSource('../src/components/melt/MeltSelect.svelte');
const meltToggleSource = uncommentedSource('../src/components/melt/MeltToggle.svelte');
const meltRadioGroupSource = uncommentedSource('../src/components/melt/MeltRadioGroup.svelte');
const meltProgressSource = uncommentedSource('../src/components/melt/MeltProgress.svelte');
const meltActionButtonSource = uncommentedSource('../src/components/melt/MeltActionButton.svelte');
const settingsPanelSource = uncommentedSource('../src/components/SettingsPanelSurface.svelte');
const controlPlaneSource = uncommentedSource('../src/components/ControlPlaneSurface.svelte');
const processManagerSource = uncommentedSource('../src/components/ProcessManagerSurface.svelte');
const searchPanelSource = uncommentedSource('../src/components/SearchPanelSurface.svelte');
const taskPreviewSource = uncommentedSource('../src/components/TaskPreviewSurface.svelte');
const stackPopupSource = uncommentedSource('../src/components/StackPopupSurface.svelte');
const topBarSource = uncommentedSource('../src/components/TopBar.svelte');
const bottomBarSource = uncommentedSource('../src/components/BottomBar.svelte');
const svelteConfig = uncommentedSource('../svelte.config.js');

test('Melt UI migration uses the Svelte 5 Melt package only', () => {
  assert.match(packageJson.dependencies.melt, /^\^0\./);
  assert.equal(packageJson.dependencies['@melt-ui/svelte'], undefined);
  assert.equal(packageJson.devDependencies?.['@melt-ui/svelte'], undefined);
  assert.doesNotMatch(
    settingsPanelSource + controlPlaneSource + processManagerSource + searchPanelSource + taskPreviewSource + stackPopupSource + topBarSource + bottomBarSource + meltSelectSource + meltToggleSource + meltRadioGroupSource + meltProgressSource + meltActionButtonSource,
    /@melt-ui\/svelte/
  );
  assert.match(svelteConfig, /vitePreprocess/);
  assert.doesNotMatch(svelteConfig, /preprocessMeltUI/);
});

test('shared JasonShell primitives wrap Melt builders', () => {
  assert.match(meltSelectSource, /import \{ Select \} from 'melt\/builders'/);
  assert.match(meltSelectSource, /new Select<string>/);
  assert.match(meltSelectSource, /select\.getOptionId\(option\.value\)/);
  assert.match(meltSelectSource, /data-open/);
  assert.match(meltToggleSource, /import \{ Toggle \} from 'melt\/builders'/);
  assert.match(meltToggleSource, /new Toggle/);
  assert.match(meltToggleSource, /data-checked/);
  assert.match(meltRadioGroupSource, /import \{ RadioGroup \} from 'melt\/builders'/);
  assert.match(meltRadioGroupSource, /new RadioGroup/);
  assert.match(meltRadioGroupSource, /radioGroup\.getItem\(option\.value\)/);
  assert.match(meltRadioGroupSource, /data-state='checked'/);
  assert.match(meltProgressSource, /import \{ Progress \} from 'melt\/builders'/);
  assert.match(meltProgressSource, /new Progress/);
  assert.match(meltProgressSource, /progress\.root/);
  assert.match(meltProgressSource, /progress\.progress/);
  assert.match(meltProgressSource, /transform:\s*translateX\(var\(--neg-progress\)\)/);
  assert.doesNotMatch(meltProgressSource, /calc\(var\(--neg-progress\)\s*\*\s*100%\)/);
  assert.match(meltActionButtonSource, /import \{ Tooltip \} from 'melt\/builders'/);
  assert.match(meltActionButtonSource, /new Tooltip/);
  assert.match(meltActionButtonSource, /tooltipTrigger = tooltipText \? actionTooltip\.trigger : \{\}/);
  assert.match(meltActionButtonSource, /\{\.\.\.tooltipTrigger\}/);
  assert.doesNotMatch(meltActionButtonSource, /\{\.\.\.actionTooltip\.trigger\}/);
  assert.match(meltActionButtonSource, /\{\.\.\.actionTooltip\.content\}/);
  assert.match(meltActionButtonSource, /export let ariaLabel: string \| undefined = undefined/);
  assert.doesNotMatch(meltActionButtonSource, /export let ariaLabel\s*=\s*''/);
  assert.match(meltActionButtonSource, /data-path=\{dataPath\}/);
  assert.match(meltActionButtonSource, /disabled=\{disabled\}/);
  assert.match(meltActionButtonSource, /draggable=\{draggable\}/);
  assert.match(meltActionButtonSource, /role=\{role\}/);
  assert.match(meltActionButtonSource, /aria-sort=\{ariaSort\}/);
  assert.match(meltActionButtonSource, /aria-colindex=\{ariaColindex\}/);
  assert.match(meltActionButtonSource, /aria-current=\{ariaCurrent\}/);
  assert.match(meltActionButtonSource, /aria-selected=\{ariaSelected\}/);
  assert.match(meltActionButtonSource, /aria-disabled=\{ariaDisabled\}/);
  assert.match(meltActionButtonSource, /aria-controls=\{ariaControls\}/);
  assert.match(meltActionButtonSource, /aria-expanded=\{ariaExpanded\}/);
  assert.match(meltActionButtonSource, /name=\{name\}/);
  assert.match(meltActionButtonSource, /style=\{style\}/);
  assert.match(meltActionButtonSource, /value=\{value\}/);
  assert.match(meltActionButtonSource, /on:click=\{onClick\}/);
  assert.match(meltActionButtonSource, /on:contextmenu=\{onContextMenu\}/);
  assert.match(meltActionButtonSource, /on:dblclick=\{onDblClick\}/);
  assert.match(meltActionButtonSource, /on:dragstart=\{onDragStart\}/);
  assert.match(meltActionButtonSource, /on:drop=\{onDrop\}/);
  assert.match(meltActionButtonSource, /on:keydown=\{onKeyDown\}/);
  assert.match(meltActionButtonSource, /on:mousedown=\{onMouseDown\}/);
  assert.match(meltActionButtonSource, /on:mouseenter=\{onMouseEnter\}/);
  assert.match(meltActionButtonSource, /on:mouseleave=\{onMouseLeave\}/);
  assert.match(meltActionButtonSource, /on:pointercancel=\{onPointerCancel\}/);
  assert.match(meltActionButtonSource, /on:pointerdown=\{onPointerDown\}/);
  assert.match(meltActionButtonSource, /on:pointermove=\{onPointerMove\}/);
  assert.match(meltActionButtonSource, /on:pointerup=\{onPointerUp\}/);
  assert.match(meltActionButtonSource, /on:lostpointercapture=\{onLostPointerCapture\}/);
});

test('settings and control-plane surfaces consume Melt-backed controls without collapsing Tauri surfaces', () => {
  assert.match(settingsPanelSource, /MeltSelect/);
  assert.match(settingsPanelSource, /MeltToggle/);
  assert.match(settingsPanelSource, /MeltRadioGroup/);
  assert.match(settingsPanelSource, /import MeltActionButton from '\.\/melt\/MeltActionButton\.svelte'/);
  assert.match(settingsPanelSource, /<MeltActionButton ariaLabel="Close settings" onClick=\{closePanel\}>x<\/MeltActionButton>/);
  assert.match(settingsPanelSource, /<MeltActionButton onClick=\{resetPresentation\}>Reset<\/MeltActionButton>/);
  assert.match(settingsPanelSource, /<MeltActionButton onClick=\{closePanel\}>Done<\/MeltActionButton>/);
  assert.match(settingsPanelSource, /dateFormatOptions/);
  assert.match(settingsPanelSource, /showSettingsPanel|hideSettingsPanel/);
  assert.match(controlPlaneSource, /import \{ Tabs \} from 'melt\/builders'/);
  assert.match(controlPlaneSource, /import MeltActionButton from '\.\/melt\/MeltActionButton\.svelte'/);
  assert.match(controlPlaneSource, /new Tabs<ControlPlaneSectionId>/);
  assert.match(controlPlaneSource, /sectionTabs\.getContent\(section\.id\)/);
  assert.match(controlPlaneSource, /MeltSelect/);
  assert.match(controlPlaneSource, /<button[\s\S]*\{\.\.\.sectionTabs\.getTrigger\(section\.id\)\}[\s\S]*controlPlaneSectionTabLabel/);
  assert.match(controlPlaneSource, /<MeltActionButton[\s\S]*disabled=\{action\.disabled\}[\s\S]*ariaLabel=\{action\.ariaLabel\}[\s\S]*title=\{controlPlaneActionLabel\(section, action\)\}/);
  assert.doesNotMatch(controlPlaneSource, /invoke\(/);
  assert.match(processManagerSource, /MeltProgress/);
  assert.match(processManagerSource, /import MeltActionButton from '\.\/melt\/MeltActionButton\.svelte'/);
  assert.match(processManagerSource, /processMetricPercent\(process\.cpuPercent, 100\)/);
  assert.match(processManagerSource, /processMetricPercent\(process\.memoryPercent, 100\)/);
  assert.match(processManagerSource, /processMetricPercent\(process\.gpuPercent, 100\)/);
  assert.match(processManagerSource, /formatProcessMemoryPercent\(process\.memoryPercent\)/);
  assert.match(processManagerSource, /formatProcessGpu\(process\.gpuPercent\)/);
  assert.match(processManagerSource, /<MeltActionButton onClick=\{\(\) => void refreshProcesses\(\{ preserveVolatileOrder: false \}\)\}>/);
  assert.match(processManagerSource, /<MeltActionButton[\s\S]*class="process-manager-close-button"[\s\S]*ariaLabel="Close process manager"[\s\S]*onClick=\{\(\) => void requestClose\(\)\}[\s\S]*>×<\/MeltActionButton>/);
  assert.match(processManagerSource, /<MeltActionButton role="columnheader" ariaSort=\{ariaSort\('name'\)\} onClick=\{\(\) => sortBy\('name'\)\}/);
  assert.match(processManagerSource, /<MeltActionButton role="columnheader" ariaSort=\{ariaSort\('startTimeMs'\)\} onClick=\{\(\) => sortBy\('startTimeMs'\)\}/);
  assert.match(processManagerSource, /<MeltActionButton[\s\S]*class="kill-button"[\s\S]*ariaLabel=\{killState\.ariaLabel\}[\s\S]*disabled=\{killState\.disabled\}[\s\S]*onClick=\{\(\) => void killRow\(process\)\}/);
  assert.doesNotMatch(processManagerSource, /<button[\s\S]*class="kill-button"/);
});

test('search panel keeps pinning input-owned while safe buttons use Melt-backed controls', () => {
  assert.match(searchPanelSource, /import MeltActionButton from '\.\/melt\/MeltActionButton\.svelte'/);
  assert.match(searchPanelSource, /function isCtrlEnterHotkey\(event: KeyboardEvent\)[\s\S]*event\.key === 'Enter' && event\.ctrlKey/);
  assert.match(searchPanelSource, /if \(isCtrlEnterHotkey\(event\)\)[\s\S]*pinSelectedFolder\(\)[\s\S]*return/);
  assert.match(searchPanelSource, /function pinSelectedFolder\(\)[\s\S]*SEARCH_PANEL_PIN_FOLDER_EVENT/);
  assert.match(searchPanelSource, /<span class="pin-folder" aria-hidden="true">Pin<\/span>[\s\S]*<span class="pin-folder-shortcut">Ctrl\+Enter<\/span>/);
  assert.match(searchPanelSource, /role="option"/);
  assert.match(searchPanelSource, /on:dblclick=\{\(\) => activateRow\(row\)\}/);
  assert.match(searchPanelSource, /on:keydown=\{\(event\) => handleOptionKeydown\(event, row\)\}/);
  assert.match(searchPanelSource, /on:dragstart=\{\(event\) => startFolderDrag\(event, result\)\}/);
  assert.doesNotMatch(searchPanelSource, /<(?:button|MeltActionButton)[^>]*class="pin-folder"/);

  assert.match(taskPreviewSource, /import MeltActionButton from '\.\/melt\/MeltActionButton\.svelte'/);
  assert.match(taskPreviewSource, /previewSurfaceClass = `surface preview-surface/);
  assert.match(taskPreviewSource, /<MeltActionButton[\s\S]*class=\{previewSurfaceClass\}[\s\S]*ariaDisabled=\{!preview\}[\s\S]*ariaLabel=\{preview \? `Activate \$\{preview\.title \|\| preview\.processName\}` : 'Task preview unavailable'\}/);
  assert.match(taskPreviewSource, /onClick=\{\(\) => void handlePreviewActivate\(\)\}/);
  assert.match(taskPreviewSource, /onKeyDown=\{\(event\) => void handlePreviewKeydown\(event\)\}/);
  assert.match(taskPreviewSource, /function handlePreviewPointerEnter\(\)[\s\S]*emit\(TASK_PREVIEW_HOVER_ENTER_EVENT\)/);
  assert.match(taskPreviewSource, /async function handlePreviewPointerLeave\(event: PointerEvent\)[\s\S]*event\.currentTarget[\s\S]*event\.relatedTarget[\s\S]*root\.contains\(relatedTarget\)[\s\S]*requestPreviewHide\('schedule'\)/);
  assert.match(taskPreviewSource, /on:pointerenter=\{handlePreviewPointerEnter\}/);
  assert.match(taskPreviewSource, /on:pointerleave=\{\(event\) => void handlePreviewPointerLeave\(event\)\}/);
  assert.doesNotMatch(taskPreviewSource, /<button[\s\S]*class="surface preview-surface"/);
});

test('stack-popup safe controls use MeltActionButton while risky grid/ref controls stay raw by design', () => {
  assert.match(stackPopupSource, /import MeltActionButton from '\.\/melt\/MeltActionButton\.svelte'/);
  assert.match(stackPopupSource, /<MeltActionButton class="path-segment" ariaCurrent=\{crumb\.path === currentPath \? 'page' : undefined\} title=\{crumb\.path\} onClick=\{\(\) => void openFolder\(crumb\.path\)\}/);
  assert.match(stackPopupSource, /<MeltActionButton disabled=\{!canGoBack\} onClick=\{\(\) => void navigateHistory\(-1\)\}>Back<\/MeltActionButton>/);
  assert.match(stackPopupSource, /<MeltActionButton disabled=\{!hasSelection\} onClick=\{\(\) => void deleteSelected\(\)\}>Delete<\/MeltActionButton>/);
  assert.match(stackPopupSource, /<MeltActionButton type="submit">OK<\/MeltActionButton>/);
  assert.match(stackPopupSource, /<MeltActionButton onClick=\{cancelInlineEditor\}>Cancel<\/MeltActionButton>/);
  assert.match(stackPopupSource, /<MeltActionButton class=\{sortHeader\('name'\)\.className\} role="columnheader" ariaColindex=\{1\} ariaSort=\{sortHeader\('name'\)\.ariaSort\} onClick=\{\(\) => sortBy\('name'\)\}/);
  assert.match(stackPopupSource, /<MeltActionButton role="menuitem" disabled=\{!selectedEntry\} onClick=\{\(\) => selectedEntry && void activateEntry\(selectedEntry\)\}>Open<\/MeltActionButton>/);
  assert.match(stackPopupSource, /<MeltActionButton class="submenu-trigger" role="menuitem" ariaHaspopup="menu" disabled=\{selectedEntry\?\.entryType !== 'File'\}>/);
  assert.match(stackPopupSource, /<MeltActionButton role="menuitem" disabled=\{!currentPath\} onClick=\{beginCreateFolder\}>New Folder<\/MeltActionButton>/);
  assert.match(stackPopupSource, /<MeltActionButton class="danger" onClick=\{\(\) => void confirmDeleteSelection\(\)\}>Delete<\/MeltActionButton>/);

  assert.match(stackPopupSource, /<button[\s\S]*type="button"[\s\S]*role="row"[\s\S]*aria-selected=\{stackState\.selectedPaths\.includes\(entry\.path\)\}[\s\S]*on:dblclick=\{\(\) => void activateEntry\(entry\)\}[\s\S]*on:dragstart=\{\(event\) => handleRowDragStart\(event, entry\)\}/);
  assert.match(stackPopupSource, /bind:this=\{deleteCancelButton\}/);
  assert.match(stackPopupSource, /class="stack-resize-grip"[\s\S]*bind:this=\{resizeGrip\}[\s\S]*on:pointerdown=\{beginResize\}/);
  assert.match(stackPopupSource, /STACK_BROWSER_BACKGROUND_CONTEXT_MENU_IGNORE_SELECTORS/);
});

test('top-bar action and pinned-folder controls use Melt-backed buttons without breaking shell flows', () => {
  assert.match(topBarSource, /import MeltActionButton from '\.\/melt\/MeltActionButton\.svelte'/);
  assert.match(topBarSource, /<MeltActionButton\s+class="shell-home-button"[\s\S]*ariaHaspopup="dialog"[\s\S]*openSettingsPanel\(event\.currentTarget\)/);
  assert.match(topBarSource, /<MeltActionButton\s+class="rail-scroll left"[\s\S]*scrollRailLeft\(\)/);
  assert.match(topBarSource, /<MeltActionButton\s+class="rail-scroll right"[\s\S]*scrollRailRight\(\)/);
  assert.match(topBarSource, /<MeltActionButton[\s\S]*dataPath=\{pin\.path\}[\s\S]*ariaLabel=\{`Open pinned folder \$\{pin\.name\}`\}/);
  assert.match(topBarSource, /onPointerDown=\{\(event\) => startPinPointerDrag\(pin, event\)\}/);
  assert.match(topBarSource, /onPointerMove=\{movePinPointerDrag\}/);
  assert.match(topBarSource, /onPointerUp=\{finishPinPointerDrag\}/);
  assert.match(topBarSource, /onClick=\{\(event\) => handlePinClick\(event, pin, index\)\}/);
  assert.match(topBarSource, /onContextMenu=\{\(event\) => handlePinContextMenu\(event, pin\)\}/);
  assert.match(topBarSource, /querySelectorAll<HTMLElement>\('button\[data-path\]'\)/);
  assert.match(topBarSource, /showTopBarPinContextMenu\(\{/);
  assert.match(topBarSource, /showStackPopup\(\{/);
  assert.doesNotMatch(topBarSource, /<button[\s\S]*data-path=\{pin\.path\}/);
});

test('bottom-bar command buttons use Melt-backed action buttons without changing taskbar semantics', () => {
  assert.match(bottomBarSource, /import MeltActionButton from '\.\/melt\/MeltActionButton\.svelte'/);
  assert.equal((bottomBarSource.match(/<MeltActionButton/g) ?? []).length, 4);
  assert.doesNotMatch(
    bottomBarSource,
    /import\s+\{[^}]*\b(?:Toggle|Tabs|Popover|SpatialMenu)\b[^}]*\}\s+from 'melt\/builders'/
  );

  assert.doesNotMatch(bottomBarSource, /quick-icon-button|launchQuickIconFromBottomBar|openQuickIconMenu/);

  assert.doesNotMatch(bottomBarSource, /<div class="launcher-strip-launchers"|<MeltActionButton\s+class=\{`launcher-button/);

  assert.match(bottomBarSource, /class:task-group-active=\{group\.isActive\}/);
  assert.match(bottomBarSource, /class:task-group-busy=\{group\.isBusy\}/);
  assert.match(bottomBarSource, /class:task-group-minimized=\{group\.isMinimized\}/);
  assert.match(bottomBarSource, /class:task-group-dragging=\{draggingGroupKey === group\.key\}/);
  assert.match(bottomBarSource, /class:task-group-drop-target=\{dropTargetGroupKey === group\.key && draggingGroupKey !== group\.key\}/);
  assert.match(bottomBarSource, /data-task-group-key=\{group\.key\}/);
  assert.match(bottomBarSource, /data-window-count=\{group\.windows\.length\}/);
  assert.match(bottomBarSource, /role="group"/);
  assert.match(bottomBarSource, /style=\{taskGroupStyle\(group\)\}/);
  assert.match(bottomBarSource, /on:pointerdown=\{\(event\) => startTaskGroupPointerDrag\(group, event\)\}/);
  assert.match(bottomBarSource, /on:pointermove=\{moveTaskGroupPointerDrag\}/);
  assert.match(bottomBarSource, /on:pointerup=\{finishTaskGroupPointerDrag\}/);
  assert.match(bottomBarSource, /on:lostpointercapture=\{handleTaskGroupLostPointerCapture\}/);

  assert.match(bottomBarSource, /<MeltActionButton\s+class=\{`task-button\$\{taskWindow\.isActive \? ' task-button-active' : ''\}\$\{taskWindow\.isMinimized \? ' task-button-minimized' : ''\}`\}[\s\S]*type="button"[\s\S]*title=\{taskWindowLabel\(taskWindow\)\}[\s\S]*ariaLabel=\{taskWindowActionLabel\(taskWindow\)\}[\s\S]*disabled=\{activatingHwnd === taskWindow\.hwnd\}/);
  assert.match(bottomBarSource, /taskWindowActionLabel\(taskWindow\)/);
  assert.match(bottomBarSource, /onPointerDown=\{\(event\) => handleTaskWindowPointerDown\(taskWindow, event\)\}/);
  assert.match(bottomBarSource, /onClick=\{\(event\) => handleTaskWindowClick\(taskWindow, event\)\}/);
  assert.match(bottomBarSource, /onMouseEnter=\{\(event\) => queuePreview\(taskWindow, event\)\}/);
  assert.match(bottomBarSource, /onMouseLeave=\{schedulePreviewHide\}/);
  assert.match(bottomBarSource, /TASK_PREVIEW_HIDE_REQUEST_EVENT/);
  assert.match(bottomBarSource, /listen<TaskPreviewHideRequest>\(TASK_PREVIEW_HIDE_REQUEST_EVENT/);
  assert.match(bottomBarSource, /function handlePreviewHideRequest\(/);
  assert.match(bottomBarSource, /onContextMenu=\{\(event\) => void openTaskMenu\(taskWindow, event\)\}/);
  assert.match(bottomBarSource, /<img class="task-icon" src=\{taskWindow\.iconDataUrl\} alt="" draggable="false" \/>/);
  assert.match(bottomBarSource, /<span class="task-label">\{taskWindowLabel\(taskWindow\)\}<\/span>/);
  assert.match(bottomBarSource, /querySelectorAll<HTMLButtonElement>\('\.task-button'\)/);

  assert.match(bottomBarSource, /<MeltActionButton\s+class="process-manager-button"[\s\S]*type="button"[\s\S]*title="Processes"[\s\S]*ariaLabel="Open process manager"[\s\S]*onClick=\{\(event\) => void openProcessManager\(event\)\}/);
  assert.match(bottomBarSource, /event\.currentTarget as HTMLButtonElement \| null/);
  assert.match(bottomBarSource, /showProcessManager\(\{ anchorLeft: rect\.left, anchorWidth: rect\.width \}\)/);

  assert.doesNotMatch(bottomBarSource, /<button[\s\S]*class="launcher-button"/);
  assert.doesNotMatch(bottomBarSource, /<button[\s\S]*class="task-button"/);
  assert.doesNotMatch(bottomBarSource, /<button[\s\S]*class="process-manager-button"/);
});
