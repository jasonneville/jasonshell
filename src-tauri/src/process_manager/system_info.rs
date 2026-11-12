//! Pure lifecycle, bounded work accounting, and telemetry wire contracts.
use serde::{Deserialize, Serialize};

const MAX_SAFE: u64 = 9_007_199_254_740_991;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProcessManagerTab {
    Processes,
    SystemInfo,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessManagerContext {
    pub visible: bool,
    pub active_tab: ProcessManagerTab,
    pub open_generation: u64,
    pub tab_revision: u64,
    pub focus_pid: Option<u32>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lane {
    Live,
    Storage,
    Inventory,
}
impl Lane {
    fn index(self) -> usize {
        match self {
            Self::Live => 0,
            Self::Storage => 1,
            Self::Inventory => 2,
        }
    }
    fn interval(self) -> u64 {
        match self {
            Self::Live => 2_000,
            Self::Storage => 15_000,
            Self::Inventory => 60_000,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorkToken {
    pub generation: u64,
    pub revision: u64,
    pub lane: Lane,
    id: u64,
}
pub struct Coordinator {
    context: ProcessManagerContext,
    outstanding: [Option<WorkToken>; 3],
    due: [u64; 3],
    next: u64,
}
impl Default for Coordinator {
    fn default() -> Self {
        Self {
            context: ProcessManagerContext {
                visible: false,
                active_tab: ProcessManagerTab::Processes,
                open_generation: 0,
                tab_revision: 0,
                focus_pid: None,
            },
            outstanding: [None; 3],
            due: [0; 3],
            next: 0,
        }
    }
}
impl Coordinator {
    pub fn context(&self) -> ProcessManagerContext {
        self.context.clone()
    }
    pub fn open(&mut self, pid: Option<u32>) -> Result<ProcessManagerContext, &'static str> {
        let generation = advance(self.context.open_generation).inspect_err(|_| {
            self.context.visible = false;
        })?;
        let revision = advance(self.context.tab_revision).inspect_err(|_| {
            self.context.visible = false;
        })?;
        self.context = ProcessManagerContext {
            visible: true,
            active_tab: ProcessManagerTab::Processes,
            open_generation: generation,
            tab_revision: revision,
            focus_pid: pid,
        };
        self.due = [0; 3];
        Ok(self.context())
    }
    pub fn hide(&mut self) -> Result<(), &'static str> {
        self.context.visible = false;
        let revision = advance(self.context.tab_revision)?;
        self.context.tab_revision = revision;
        Ok(())
    }
    pub fn activate(
        &mut self,
        generation: u64,
        revision: u64,
        tab: ProcessManagerTab,
        visible: bool,
        focused: bool,
    ) -> Result<ProcessManagerContext, &'static str> {
        if !self.context.visible
            || !visible
            || !focused
            || generation != self.context.open_generation
            || revision <= self.context.tab_revision
            || safe_number(revision).is_none()
        {
            return Err("inactive or stale context");
        }
        self.context.tab_revision = revision;
        self.context.active_tab = tab;
        self.due = [0; 3];
        Ok(self.context())
    }
    pub fn admit(
        &mut self,
        lane: Lane,
        generation: u64,
        revision: u64,
        now_ms: u64,
        visible: bool,
        focused: bool,
    ) -> Option<WorkToken> {
        if !visible
            || !focused
            || !self.context.visible
            || self.context.active_tab != ProcessManagerTab::SystemInfo
            || generation != self.context.open_generation
            || revision != self.context.tab_revision
        {
            return None;
        }
        let index = lane.index();
        if self.outstanding[index].is_some() || now_ms < self.due[index] {
            return None;
        }
        self.next = self.next.checked_add(1)?;
        let token = WorkToken {
            generation,
            revision,
            lane,
            id: self.next,
        };
        self.outstanding[index] = Some(token);
        Some(token)
    }
    pub fn complete(&mut self, token: WorkToken, now_ms: u64) -> bool {
        let index = token.lane.index();
        if self.outstanding[index] != Some(token) {
            return false;
        }
        self.outstanding[index] = None;
        let current = self.context.visible
            && self.context.active_tab == ProcessManagerTab::SystemInfo
            && self.context.open_generation == token.generation
            && self.context.tab_revision == token.revision;
        if current {
            self.due[index] = now_ms.saturating_add(token.lane.interval());
        }
        current
    }
}
fn advance(value: u64) -> Result<u64, &'static str> {
    value
        .checked_add(1)
        .and_then(safe_number)
        .ok_or("revision exhausted")
}
pub fn authorize_manager(
    label: &str,
    same_registered_instance: impl FnOnce() -> bool,
) -> Result<(), &'static str> {
    if label != "process-manager" || !same_registered_instance() {
        Err("unauthorized")
    } else {
        Ok(())
    }
}
pub fn safe_number(value: u64) -> Option<u64> {
    (value <= MAX_SAFE).then_some(value)
}
pub fn network_rate(previous: u64, current: u64, elapsed_ms: u64) -> Option<f64> {
    if elapsed_ms == 0 {
        return None;
    }
    let rate = current.checked_sub(previous)? as f64 * 1000.0 / elapsed_ms as f64;
    rate.is_finite().then_some(rate)
}
pub fn memory_used_percent(total: u64, available: u64) -> Option<f64> {
    if total == 0 || available > total {
        None
    } else {
        Some((total - available) as f64 / total as f64 * 100.0)
    }
}
pub fn gpu_busiest_engine(values: &[f64]) -> Option<f64> {
    values
        .iter()
        .copied()
        .filter(|value| value.is_finite() && *value >= 0.0)
        .reduce(f64::max)
        .map(|value| value.min(100.0))
}
pub fn valid_probe_root(root: &str) -> bool {
    let b = root.as_bytes();
    b.len() == 3 && b[0].is_ascii_uppercase() && b[1] == b':' && b[2] == b'\\'
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProbeToken {
    pub root: String,
    pub generation: u64,
    pub revision: u64,
    id: u64,
}
struct ProbeEntry {
    token: ProbeToken,
    started: u64,
    retired: bool,
    kill_requested: bool,
}
#[derive(Default)]
pub struct ProbeBudget {
    entries: Vec<ProbeEntry>,
    next: u64,
}
impl ProbeBudget {
    pub fn reserve(
        &mut self,
        root: &str,
        generation: u64,
        revision: u64,
        now_ms: u64,
    ) -> Option<ProbeToken> {
        if !valid_probe_root(root)
            || self.entries.len() >= 2
            || self.entries.iter().any(|entry| entry.token.root == root)
        {
            return None;
        }
        self.next = self.next.checked_add(1)?;
        let token = ProbeToken {
            root: root.into(),
            generation,
            revision,
            id: self.next,
        };
        self.entries.push(ProbeEntry {
            token: token.clone(),
            started: now_ms,
            retired: false,
            kill_requested: false,
        });
        Some(token)
    }
    pub fn retire(&mut self, generation: u64, revision: u64) {
        for entry in &mut self.entries {
            if entry.token.generation == generation && entry.token.revision == revision {
                entry.retired = true;
            }
        }
    }
    pub fn kill_requests(&mut self, now_ms: u64) -> Vec<ProbeToken> {
        self.entries
            .iter_mut()
            .filter_map(|entry| {
                if !entry.kill_requested
                    && (entry.retired || now_ms.saturating_sub(entry.started) >= 1500)
                {
                    entry.kill_requested = true;
                    Some(entry.token.clone())
                } else {
                    None
                }
            })
            .collect()
    }
    pub fn reaped(&mut self, token: &ProbeToken) -> bool {
        if let Some(index) = self.entries.iter().position(|entry| &entry.token == token) {
            self.entries.swap_remove(index);
            true
        } else {
            false
        }
    }
    pub fn may_publish(&self, token: &ProbeToken, generation: u64, revision: u64) -> bool {
        self.entries.iter().any(|entry| {
            &entry.token == token
                && !entry.retired
                && !entry.kill_requested
                && token.generation == generation
                && token.revision == revision
        })
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SampleState {
    Ready,
    WarmingUp,
    Partial,
    Truncated,
    Unavailable,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SampleReason {
    NoBaseline,
    ProviderNotVerified,
    NotSupported,
    AccessDenied,
    QueryFailed,
    TimedOut,
    LimitReached,
    Stale,
    NotConnected,
}
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sample<T> {
    pub state: SampleState,
    pub value: Option<T>,
    pub reason: Option<SampleReason>,
    pub source: String,
    pub unit: Option<String>,
    pub sampled_at_ms: Option<u64>,
}
pub trait WireValue {
    fn valid_wire(&self) -> bool;
}
impl WireValue for u64 {
    fn valid_wire(&self) -> bool {
        safe_number(*self).is_some()
    }
}
impl WireValue for f64 {
    fn valid_wire(&self) -> bool {
        self.is_finite() && self.abs() <= MAX_SAFE as f64
    }
}
impl WireValue for String {
    fn valid_wire(&self) -> bool {
        true
    }
}
impl WireValue for Vec<String> {
    fn valid_wire(&self) -> bool {
        true
    }
}
impl<T: Serialize + WireValue> Serialize for Sample<T> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::{Error, SerializeStruct};
        if self.value.as_ref().is_some_and(|value| !value.valid_wire())
            || self
                .sampled_at_ms
                .is_some_and(|value| safe_number(value).is_none())
            || (matches!(self.state, SampleState::Ready) && self.value.is_none())
        {
            return Err(S::Error::custom("invalid telemetry numeric or ready value"));
        }
        let mut object = serializer.serialize_struct("Sample", 6)?;
        object.serialize_field("state", &self.state)?;
        object.serialize_field("value", &self.value)?;
        object.serialize_field("reason", &self.reason)?;
        object.serialize_field("source", &self.source)?;
        object.serialize_field("unit", &self.unit)?;
        object.serialize_field("sampledAtMs", &self.sampled_at_ms)?;
        object.end()
    }
}
fn serialize_safe<S: serde::Serializer>(value: &u64, serializer: S) -> Result<S::Ok, S::Error> {
    use serde::ser::Error;
    if safe_number(*value).is_none() {
        return Err(S::Error::custom("unsafe telemetry integer"));
    }
    serializer.serialize_u64(*value)
}
fn serialize_option_safe<S: serde::Serializer>(
    value: &Option<u64>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    use serde::ser::Error;
    if value.is_some_and(|value| safe_number(value).is_none()) {
        return Err(S::Error::custom("unsafe telemetry timestamp"));
    }
    value.serialize(serializer)
}
/// Parses only SMBIOS type 0 vendor/version and type 2 vendor/model.
/// Serial, UUID, asset-tag and other identifiers are never read or returned.
pub fn smbios_allowlist(raw: &[u8]) -> (Option<String>, Option<String>) {
    if raw.len() < 8 {
        return (None, None);
    }
    let length = u32::from_le_bytes(raw[4..8].try_into().unwrap_or([0; 4])) as usize;
    let Some(end) = 8usize.checked_add(length).filter(|end| *end <= raw.len()) else {
        return (None, None);
    };
    let mut position = 8;
    let mut board = None;
    let mut bios = None;
    for _ in 0..4096 {
        if position == end {
            return (board, bios);
        }
        if position + 4 > end {
            return (None, None);
        }
        let kind = raw[position];
        let size = raw[position + 1] as usize;
        if size < 4 || position + size > end {
            return (None, None);
        }
        let start = position + size;
        let Some(relative) = raw[start..end].windows(2).position(|pair| pair == [0, 0]) else {
            return (None, None);
        };
        let string_end = start + relative;
        let field = |index: u8| -> Option<String> {
            if index == 0 {
                return None;
            }
            let bytes = raw[start..string_end]
                .split(|b| *b == 0)
                .nth(index as usize - 1)?;
            if bytes.is_empty() || bytes.len() > 256 {
                return None;
            }
            Some(
                String::from_utf8_lossy(bytes)
                    .chars()
                    .filter(|c| !c.is_control())
                    .take(256)
                    .collect(),
            )
        };
        if (kind == 0 || kind == 2) && size >= 6 {
            let value = match (field(raw[position + 4]), field(raw[position + 5])) {
                (Some(a), Some(b)) => Some(format!("{a} {b}").chars().take(256).collect()),
                (a, b) => a.or(b),
            };
            if kind == 0 {
                bios = value;
            } else {
                board = value;
            }
        }
        if kind == 127 {
            return (board, bios);
        }
        position = string_end + 2;
    }
    (None, None)
}
impl<T> Sample<T> {
    pub fn ready(value: T, source: &str, unit: Option<&str>, now: u64) -> Self {
        Self {
            state: SampleState::Ready,
            value: Some(value),
            reason: None,
            source: source.into(),
            unit: unit.map(str::to_owned),
            sampled_at_ms: Some(now),
        }
    }
    pub fn unavailable(source: &str, unit: Option<&str>, reason: SampleReason) -> Self {
        Self {
            state: SampleState::Unavailable,
            value: None,
            reason: Some(reason),
            source: source.into(),
            unit: unit.map(str::to_owned),
            sampled_at_ms: None,
        }
    }
    pub fn warming(source: &str, unit: Option<&str>) -> Self {
        Self {
            state: SampleState::WarmingUp,
            value: None,
            reason: Some(SampleReason::NoBaseline),
            source: source.into(),
            unit: unit.map(str::to_owned),
            sampled_at_ms: None,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Memory {
    pub total_bytes: Sample<u64>,
    pub available_bytes: Sample<u64>,
    pub used_percent: Sample<f64>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GpuMetric {
    pub id: String,
    pub name: Option<String>,
    pub utilization_percent: Sample<f64>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkAdapter {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub ipv4: Vec<String>,
    pub ipv6: Vec<String>,
    pub gateways: Vec<String>,
    pub dns_servers: Vec<String>,
    pub link_receive_bits_per_second: Sample<u64>,
    pub link_transmit_bits_per_second: Sample<u64>,
    pub receive_bytes_per_second: Sample<f64>,
    pub transmit_bytes_per_second: Sample<f64>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sensors {
    pub cpu_temperature_c: Sample<f64>,
    pub gpu_temperature_c: Sample<f64>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Live {
    pub state: SampleState,
    #[serde(serialize_with = "serialize_option_safe")]
    pub sampled_at_ms: Option<u64>,
    pub cpu: Sample<f64>,
    pub memory: Memory,
    pub gpus: Vec<GpuMetric>,
    pub network: Vec<NetworkAdapter>,
    pub uptime_ms: Sample<u64>,
    pub sensors: Sensors,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageRoot {
    pub root: String,
    pub kind: String,
    pub filesystem: Sample<String>,
    pub total_bytes: Sample<u64>,
    pub total_free_bytes: Sample<u64>,
    pub caller_available_bytes: Sample<u64>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Storage {
    pub state: SampleState,
    #[serde(serialize_with = "serialize_option_safe")]
    pub sampled_at_ms: Option<u64>,
    pub roots: Vec<StorageRoot>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Inventory {
    pub state: SampleState,
    #[serde(serialize_with = "serialize_option_safe")]
    pub sampled_at_ms: Option<u64>,
    pub cpu_name: Sample<String>,
    pub physical_cores: Sample<u64>,
    pub logical_processors: Sample<u64>,
    pub installed_ram_bytes: Sample<u64>,
    pub gpu_names: Sample<Vec<String>>,
    pub vram_bytes: Sample<u64>,
    pub board: Sample<String>,
    pub bios: Sample<String>,
    pub windows_version: Sample<String>,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemInfoSnapshot {
    pub schema_version: u8,
    #[serde(serialize_with = "serialize_safe")]
    pub open_generation: u64,
    #[serde(serialize_with = "serialize_safe")]
    pub tab_revision: u64,
    #[serde(serialize_with = "serialize_safe")]
    pub snapshot_revision: u64,
    #[serde(serialize_with = "serialize_safe")]
    pub captured_at_ms: u64,
    pub live: Live,
    pub storage: Storage,
    pub inventory: Inventory,
}
/// App-owned state shared by the production scheduler and IPC. Collectors are
/// deliberately absent: cache reads cannot wait on or enqueue hardware work.
pub struct RuntimeInner {
    pub coordinator: Coordinator,
    pub cache: Option<SystemInfoSnapshot>,
    pub revision: u64,
    pub probes: ProbeBudget,
}
impl Default for RuntimeInner {
    fn default() -> Self {
        Self {
            coordinator: Coordinator::default(),
            cache: None,
            revision: 0,
            probes: ProbeBudget::default(),
        }
    }
}
impl RuntimeInner {
    pub fn invalidate(&mut self) {
        if let Some(old) = &self.cache {
            self.probes.retire(old.open_generation, old.tab_revision);
        }
        self.cache = None;
    }
    pub fn warm(&mut self, now_ms: u64) -> Result<(), String> {
        let Some(revision) = self.revision.checked_add(1).and_then(safe_number) else {
            self.invalidate();
            let _ = self.coordinator.hide();
            return Err("revision exhausted".into());
        };
        self.revision = revision;
        self.cache = Some(SystemInfoSnapshot::warming(
            &self.coordinator.context(),
            self.revision,
            now_ms,
        ));
        Ok(())
    }
    pub fn read_snapshot(
        &self,
        generation: u64,
        revision: u64,
        after: Option<u64>,
        visible: bool,
        focused: bool,
    ) -> Result<Option<SystemInfoSnapshot>, String> {
        let context = self.coordinator.context();
        if !visible
            || !focused
            || !context.visible
            || context.active_tab != ProcessManagerTab::SystemInfo
            || context.open_generation != generation
            || context.tab_revision != revision
        {
            return Err("inactive or stale context".into());
        }
        self.cache
            .as_ref()
            .ok_or_else(|| "cache unavailable".to_string())?
            .read_cached(generation, revision, after)
            .map_err(str::to_owned)
    }
}
impl SystemInfoSnapshot {
    /// Pure cache read. There is deliberately no collector/scheduler dependency.
    pub fn read_cached(
        &self,
        generation: u64,
        revision: u64,
        after: Option<u64>,
    ) -> Result<Option<Self>, &'static str> {
        if generation != self.open_generation
            || revision != self.tab_revision
            || after.is_some_and(|value| safe_number(value).is_none())
        {
            return Err("inactive or stale context");
        }
        Ok(
            if after.is_some_and(|value| value >= self.snapshot_revision) {
                None
            } else {
                Some(self.clone())
            },
        )
    }
    pub fn warming(context: &ProcessManagerContext, revision: u64, now: u64) -> Self {
        Self {
            schema_version: 1,
            open_generation: context.open_generation,
            tab_revision: context.tab_revision,
            snapshot_revision: revision,
            captured_at_ms: now,
            live: Live {
                state: SampleState::WarmingUp,
                sampled_at_ms: None,
                cpu: Sample::warming("GetSystemTimes", Some("percent")),
                memory: Memory {
                    total_bytes: Sample::warming("GlobalMemoryStatusEx", Some("bytes")),
                    available_bytes: Sample::warming("GlobalMemoryStatusEx", Some("bytes")),
                    used_percent: Sample::warming("GlobalMemoryStatusEx", Some("percent")),
                },
                gpus: vec![],
                network: vec![],
                uptime_ms: Sample::warming("GetTickCount64", Some("milliseconds")),
                sensors: Sensors {
                    cpu_temperature_c: Sample::unavailable(
                        "verified_running_provider",
                        Some("celsius"),
                        SampleReason::ProviderNotVerified,
                    ),
                    gpu_temperature_c: Sample::unavailable(
                        "verified_running_provider",
                        Some("celsius"),
                        SampleReason::ProviderNotVerified,
                    ),
                },
            },
            storage: Storage {
                state: SampleState::WarmingUp,
                sampled_at_ms: None,
                roots: vec![],
            },
            inventory: Inventory {
                state: SampleState::WarmingUp,
                sampled_at_ms: None,
                cpu_name: Sample::warming("registry", None),
                physical_cores: Sample::warming("GetLogicalProcessorInformationEx", None),
                logical_processors: Sample::warming("GetActiveProcessorCount", None),
                installed_ram_bytes: Sample::warming(
                    "GetPhysicallyInstalledSystemMemory",
                    Some("bytes"),
                ),
                gpu_names: Sample::warming("DXGI", None),
                vram_bytes: Sample::warming("DXGI", Some("bytes")),
                board: Sample::warming("SMBIOS", None),
                bios: Sample::warming("SMBIOS", None),
                windows_version: Sample::warming("registry", None),
            },
        }
    }
}
