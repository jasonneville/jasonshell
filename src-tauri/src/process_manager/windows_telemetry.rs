//! Allowlisted Windows collectors; no process-row aggregation or sensor launching.
use super::{native_runtime::now, system_info::*};
use std::{collections::HashMap, time::Instant};
use windows::{
    core::{w, PCWSTR},
    Win32::{
        Foundation::FILETIME,
        Graphics::Dxgi::{CreateDXGIFactory1, IDXGIFactory1, DXGI_ADAPTER_FLAG_SOFTWARE},
        NetworkManagement::IpHelper::*,
        Networking::WinSock::{SOCKADDR_IN, SOCKADDR_IN6, SOCKET_ADDRESS},
        System::{
            Performance::*,
            Registry::{RegGetValueW, HKEY_LOCAL_MACHINE, RRF_RT_REG_SZ},
            SystemInformation::*,
            Threading::{
                GetActiveProcessorCount, GetActiveProcessorGroupCount, GetSystemTimes,
                ALL_PROCESSOR_GROUPS,
            },
        },
    },
};

pub enum Collected {
    Live(Live),
    Storage(Storage),
    Inventory(Inventory),
}
#[derive(Default)]
pub struct LiveCollector {
    epoch: Option<(u64, u64)>,
    cpu: Option<(u64, u64)>,
    network: HashMap<u64, (u64, u64, Instant)>,
    gpu: Option<(isize, isize)>,
}
impl Drop for LiveCollector {
    fn drop(&mut self) {
        self.close_gpu();
    }
}
fn integer(value: u64, source: &str, unit: Option<&str>, time: u64) -> Sample<u64> {
    safe_number(value)
        .map(|value| Sample::ready(value, source, unit, time))
        .unwrap_or_else(|| Sample::unavailable(source, unit, SampleReason::LimitReached))
}
fn unavailable<T>(source: &str, unit: Option<&str>) -> Sample<T> {
    Sample::unavailable(source, unit, SampleReason::QueryFailed)
}
fn wide(value: &[u16]) -> String {
    String::from_utf16_lossy(&value[..value.iter().position(|c| *c == 0).unwrap_or(value.len())])
        .chars()
        .filter(|c| !c.is_control())
        .take(256)
        .collect()
}
impl LiveCollector {
    fn close_gpu(&mut self) {
        if let Some((query, _)) = self.gpu.take() {
            unsafe {
                PdhCloseQuery(PDH_HQUERY(query as *mut _));
            }
        }
    }
    pub fn collect(&mut self, generation: u64, revision: u64) -> Live {
        if self.epoch != Some((generation, revision)) {
            self.cpu = None;
            self.network.clear();
            self.close_gpu();
            self.epoch = Some((generation, revision));
        }
        let time = now();
        let cpu = self.cpu_sample(time);
        let mut memory = Memory {
            total_bytes: unavailable("GlobalMemoryStatusEx", Some("bytes")),
            available_bytes: unavailable("GlobalMemoryStatusEx", Some("bytes")),
            used_percent: unavailable("GlobalMemoryStatusEx", Some("percent")),
        };
        let mut status = MEMORYSTATUSEX {
            dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32,
            ..Default::default()
        };
        if unsafe { GlobalMemoryStatusEx(&mut status) }.is_ok() {
            memory.total_bytes = integer(
                status.ullTotalPhys,
                "GlobalMemoryStatusEx",
                Some("bytes"),
                time,
            );
            memory.available_bytes = integer(
                status.ullAvailPhys,
                "GlobalMemoryStatusEx",
                Some("bytes"),
                time,
            );
            if let Some(percent) = memory_used_percent(status.ullTotalPhys, status.ullAvailPhys) {
                memory.used_percent =
                    Sample::ready(percent, "GlobalMemoryStatusEx", Some("percent"), time);
            }
        }
        let (network, network_state) = self.network_sample(time);
        Live {
            state: if matches!(network_state, SampleState::Truncated) {
                SampleState::Truncated
            } else {
                SampleState::Partial
            },
            sampled_at_ms: Some(time),
            cpu,
            memory,
            gpus: self.gpu_sample(time),
            network,
            uptime_ms: integer(
                unsafe { GetTickCount64() },
                "GetTickCount64",
                Some("milliseconds"),
                time,
            ),
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
        }
    }
    fn cpu_sample(&mut self, time: u64) -> Sample<f64> {
        // GetSystemTimes covers the primary processor group only on >64 CPUs.
        // Never promote that subset to whole-machine telemetry.
        let processors = unsafe { GetActiveProcessorCount(ALL_PROCESSOR_GROUPS) };
        if processors == 0 || processors > 64 || unsafe { GetActiveProcessorGroupCount() } != 1 {
            self.cpu = None;
            return Sample::unavailable(
                "GetSystemTimes",
                Some("percent"),
                SampleReason::NotSupported,
            );
        }
        let mut idle = FILETIME::default();
        let mut kernel = FILETIME::default();
        let mut user = FILETIME::default();
        if unsafe { GetSystemTimes(Some(&mut idle), Some(&mut kernel), Some(&mut user)) }.is_err() {
            self.cpu = None;
            return unavailable("GetSystemTimes", Some("percent"));
        }
        let ticks = |t: FILETIME| ((t.dwHighDateTime as u64) << 32) | t.dwLowDateTime as u64;
        let Some(total) = ticks(kernel).checked_add(ticks(user)) else {
            self.cpu = None;
            return unavailable("GetSystemTimes", Some("percent"));
        };
        let current = (ticks(idle), total);
        let previous = self.cpu.replace(current);
        let value = previous.and_then(|(idle, total)| {
            let elapsed = current.1.checked_sub(total)?;
            let idle = current.0.checked_sub(idle)?;
            if elapsed == 0 || idle > elapsed {
                None
            } else {
                Some((elapsed - idle) as f64 / elapsed as f64 * 100.0)
            }
        });
        value
            .map(|value| Sample::ready(value, "GetSystemTimes", Some("percent"), time))
            .unwrap_or_else(|| Sample::warming("GetSystemTimes", Some("percent")))
    }
    fn gpu_sample(&mut self, time: u64) -> Vec<GpuMetric> {
        let mut warming = false;
        if self.gpu.is_none() {
            let mut query = PDH_HQUERY::default();
            let mut counter = PDH_HCOUNTER::default();
            if unsafe { PdhOpenQueryW(PCWSTR::null(), 0, &mut query) } != 0 {
                return vec![];
            }
            if unsafe {
                PdhAddEnglishCounterW(
                    query,
                    w!("\\GPU Engine(*)\\Utilization Percentage"),
                    0,
                    &mut counter,
                )
            } != 0
            {
                unsafe {
                    PdhCloseQuery(query);
                }
                return vec![];
            }
            self.gpu = Some((query.0 as isize, counter.0 as isize));
            warming = true;
        }
        let Some((query, counter)) = self.gpu else {
            return vec![];
        };
        if unsafe { PdhCollectQueryData(PDH_HQUERY(query as *mut _)) } != 0 {
            return vec![];
        }
        if warming {
            return vec![];
        }
        let counter = PDH_HCOUNTER(counter as *mut _);
        let mut bytes = 0;
        let mut count = 0;
        if unsafe {
            PdhGetFormattedCounterArrayW(counter, PDH_FMT_DOUBLE, &mut bytes, &mut count, None)
        } != PDH_MORE_DATA
            || bytes == 0
            || bytes > 4 * 1024 * 1024
        {
            return vec![];
        }
        // u64 allocation provides alignment for PDH item structures.
        let mut buffer = vec![0u64; (bytes as usize).div_ceil(8)];
        if unsafe {
            PdhGetFormattedCounterArrayW(
                counter,
                PDH_FMT_DOUBLE,
                &mut bytes,
                &mut count,
                Some(buffer.as_mut_ptr().cast()),
            )
        } != 0
            || count as usize
                > buffer.len() * 8 / std::mem::size_of::<PDH_FMT_COUNTERVALUE_ITEM_W>()
        {
            return vec![];
        }
        let items = unsafe {
            std::slice::from_raw_parts(
                buffer.as_ptr().cast::<PDH_FMT_COUNTERVALUE_ITEM_W>(),
                count as usize,
            )
        };
        let mut adapters: HashMap<String, HashMap<(u32, u32), f64>> = HashMap::new();
        let mut seen = std::collections::HashSet::new();
        let mut truncated = false;
        for item in items {
            if item.FmtValue.CStatus != PDH_CSTATUS_VALID_DATA
                && item.FmtValue.CStatus != PDH_CSTATUS_NEW_DATA
            {
                continue;
            }
            let Some(name) =
                bounded_wide_in_buffer(item.szName.0, buffer.as_ptr().cast(), buffer.len() * 8)
            else {
                continue;
            };
            let Some(tail) = name.split("luid_").nth(1) else {
                continue;
            };
            let parts: Vec<_> = tail.split('_').take(2).collect();
            if parts.len() != 2
                || !parts.iter().all(|p| {
                    p.len() == 10
                        && p.starts_with("0x")
                        && p[2..].bytes().all(|b| b.is_ascii_hexdigit())
                })
            {
                continue;
            }
            let id = parts.join("_");
            let number = |marker: &str| -> Option<u32> {
                let value = name.split(marker).nth(1)?.split('_').next()?;
                if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
                    return None;
                }
                value.parse().ok()
            };
            let (Some(physical), Some(engine)) = (number("_phys_"), number("_eng_")) else {
                continue;
            };
            let value = unsafe { item.FmtValue.Anonymous.doubleValue };
            if !value.is_finite() || value < 0.0 {
                continue;
            }
            // A counter instance is process-scoped. Aggregate only contributions
            // to the SAME physical adapter/engine, then select the busiest engine.
            // Never sum different engines, adapters, or process-row GPU metrics.
            let Some(instance) = name.split("_engtype_").next() else {
                continue;
            };
            if !seen.insert(instance.to_owned()) {
                continue;
            }
            if !adapters.contains_key(&id) && adapters.len() >= 16 {
                truncated = true;
                continue;
            }
            let engines = adapters.entry(id).or_default();
            if !engines.contains_key(&(physical, engine)) && engines.len() >= 1024 {
                truncated = true;
                continue;
            }
            let total = engines.entry((physical, engine)).or_default();
            *total = (*total + value).min(100.0);
        }
        adapters
            .into_iter()
            .map(|(id, values)| GpuMetric {
                id,
                name: None,
                utilization_percent: gpu_busiest_engine(&values.into_values().collect::<Vec<_>>())
                    .map(|value| {
                        let mut sample = Sample::ready(
                            value,
                            "PDH.GPU Engine.busiest_engine",
                            Some("percent"),
                            time,
                        );
                        if truncated {
                            sample.state = SampleState::Truncated;
                            sample.reason = Some(SampleReason::LimitReached);
                        }
                        sample
                    })
                    .unwrap_or_else(|| {
                        unavailable("PDH.GPU Engine.busiest_engine", Some("percent"))
                    }),
            })
            .collect()
    }
    fn network_sample(&mut self, time: u64) -> (Vec<NetworkAdapter>, SampleState) {
        let mut bytes = 15_000u32;
        let mut buffer = vec![0u64; (bytes as usize).div_ceil(8)];
        let flags = GAA_FLAG_INCLUDE_GATEWAYS | GAA_FLAG_SKIP_ANYCAST | GAA_FLAG_SKIP_MULTICAST;
        let mut status = unsafe {
            GetAdaptersAddresses(0, flags, None, Some(buffer.as_mut_ptr().cast()), &mut bytes)
        };
        if status == 111 && bytes <= 1024 * 1024 {
            buffer.resize((bytes as usize).div_ceil(8), 0);
            status = unsafe {
                GetAdaptersAddresses(0, flags, None, Some(buffer.as_mut_ptr().cast()), &mut bytes)
            };
        }
        if status != 0 {
            self.network.clear();
            return (vec![], SampleState::Unavailable);
        }
        let base = buffer.as_ptr() as usize;
        let end = base + buffer.len() * 8;
        let inside =
            |p: usize, size: usize| p >= base && p.checked_add(size).is_some_and(|n| n <= end);
        let mut next = buffer.as_ptr().cast::<IP_ADAPTER_ADDRESSES_LH>();
        let mut output = Vec::new();
        let mut baselines = HashMap::new();
        let mut truncated = false;
        let tick = Instant::now();
        for _ in 0..32 {
            if next.is_null() {
                break;
            }
            if !inside(
                next as usize,
                std::mem::size_of::<IP_ADAPTER_ADDRESSES_LH>(),
            ) {
                truncated = true;
                break;
            }
            let adapter = unsafe { &*next };
            next = adapter.Next;
            let luid = unsafe { adapter.Luid.Value };
            let mut row = MIB_IF_ROW2 {
                InterfaceLuid: adapter.Luid,
                ..Default::default()
            };
            let valid_row = unsafe { GetIfEntry2(&mut row) }.0 == 0;
            let connected = adapter.OperStatus.0 == 1;
            let name = bounded_wide_in_buffer(
                adapter.FriendlyName.0,
                buffer.as_ptr().cast(),
                buffer.len() * 8,
            )
            .unwrap_or_else(|| "Network adapter".into());
            let mut ipv4 = Vec::new();
            let mut ipv6 = Vec::new();
            let mut address = adapter.FirstUnicastAddress;
            for _ in 0..32 {
                if address.is_null() {
                    break;
                }
                if !inside(
                    address as usize,
                    std::mem::size_of::<IP_ADAPTER_UNICAST_ADDRESS_LH>(),
                ) {
                    truncated = true;
                    break;
                }
                let value = unsafe { &*address };
                address = value.Next;
                if let Some(ip) = socket_ip(value.Address, base, end) {
                    let list = if ip.contains(':') {
                        &mut ipv6
                    } else {
                        &mut ipv4
                    };
                    if list.len() < 16 {
                        if !list.contains(&ip) {
                            list.push(ip);
                        }
                    } else {
                        truncated = true;
                    }
                }
            }
            if !address.is_null() {
                truncated = true;
            }
            let mut gateways = Vec::new();
            let mut gateway = adapter.FirstGatewayAddress;
            for _ in 0..16 {
                if gateway.is_null() {
                    break;
                }
                if !inside(
                    gateway as usize,
                    std::mem::size_of::<IP_ADAPTER_GATEWAY_ADDRESS_LH>(),
                ) {
                    truncated = true;
                    break;
                }
                let value = unsafe { &*gateway };
                gateway = value.Next;
                if let Some(ip) = socket_ip(value.Address, base, end) {
                    if !gateways.contains(&ip) {
                        gateways.push(ip);
                    }
                }
            }
            if !gateway.is_null() {
                truncated = true;
            }
            let mut dns_servers = Vec::new();
            let mut dns = adapter.FirstDnsServerAddress;
            for _ in 0..16 {
                if dns.is_null() {
                    break;
                }
                if !inside(
                    dns as usize,
                    std::mem::size_of::<IP_ADAPTER_DNS_SERVER_ADDRESS_XP>(),
                ) {
                    truncated = true;
                    break;
                }
                let value = unsafe { &*dns };
                dns = value.Next;
                if let Some(ip) = socket_ip(value.Address, base, end) {
                    if !dns_servers.contains(&ip) {
                        dns_servers.push(ip);
                    }
                }
            }
            if !dns.is_null() {
                truncated = true;
            }
            let rates = self.network.get(&luid).and_then(|(rx, tx, previous)| {
                let ms = tick.duration_since(*previous).as_millis() as u64;
                Some((
                    network_rate(*rx, row.InOctets, ms)?,
                    network_rate(*tx, row.OutOctets, ms)?,
                ))
            });
            let rate = |value: Option<f64>| {
                if !connected {
                    Sample::unavailable(
                        "GetIfEntry2",
                        Some("bytes_per_second"),
                        SampleReason::NotConnected,
                    )
                } else if !valid_row {
                    unavailable("GetIfEntry2", Some("bytes_per_second"))
                } else {
                    value
                        .map(|value| {
                            Sample::ready(value, "GetIfEntry2", Some("bytes_per_second"), time)
                        })
                        .unwrap_or_else(|| Sample::warming("GetIfEntry2", Some("bytes_per_second")))
                }
            };
            if valid_row && connected {
                baselines.insert(luid, (row.InOctets, row.OutOctets, tick));
            }
            // IF_TYPE_PPP and tunnel are VPN-capable; do not guess physical from
            // an Ethernet type alone (Hyper-V and VPNs also use Ethernet).
            let kind = match adapter.IfType {
                23 | 131 => "vpn",
                24 => "virtual",
                _ if valid_row && row.InterfaceAndOperStatusFlags._bitfield & 1 != 0 => "physical",
                _ => "unknown",
            };
            output.push(NetworkAdapter {
                id: format!("interface-{}", unsafe {
                    adapter.Anonymous1.Anonymous.IfIndex
                }),
                name,
                kind: kind.into(),
                ipv4,
                ipv6,
                gateways,
                dns_servers,
                link_receive_bits_per_second: integer(
                    adapter.ReceiveLinkSpeed,
                    "GetAdaptersAddresses",
                    Some("bits_per_second"),
                    time,
                ),
                link_transmit_bits_per_second: integer(
                    adapter.TransmitLinkSpeed,
                    "GetAdaptersAddresses",
                    Some("bits_per_second"),
                    time,
                ),
                receive_bytes_per_second: rate(rates.map(|v| v.0)),
                transmit_bytes_per_second: rate(rates.map(|v| v.1)),
            });
        }
        if !next.is_null() {
            truncated = true;
        }
        self.network = baselines;
        (
            output,
            if truncated {
                SampleState::Truncated
            } else {
                SampleState::Ready
            },
        )
    }
}
fn bounded_wide_in_buffer(pointer: *const u16, base: *const u8, bytes: usize) -> Option<String> {
    let p = pointer as usize;
    let start = base as usize;
    let end = start.checked_add(bytes)?;
    if p < start || p >= end || p % 2 != 0 {
        return None;
    }
    let slice = unsafe { std::slice::from_raw_parts(pointer, ((end - p) / 2).min(257)) };
    let length = slice.iter().position(|c| *c == 0)?;
    Some(wide(&slice[..length]))
}
fn socket_ip(address: SOCKET_ADDRESS, base: usize, end: usize) -> Option<String> {
    let p = address.lpSockaddr as usize;
    let size = usize::try_from(address.iSockaddrLength).ok()?;
    if p < base || p.checked_add(size)? > end || size < 2 {
        return None;
    }
    let family = unsafe { std::ptr::read_unaligned(p as *const u16) };
    match family {
        2 if size >= std::mem::size_of::<SOCKADDR_IN>() => {
            let value = unsafe { std::ptr::read_unaligned(p as *const SOCKADDR_IN) };
            Some(
                std::net::Ipv4Addr::from(unsafe { value.sin_addr.S_un.S_addr }.to_ne_bytes())
                    .to_string(),
            )
        }
        23 if size >= std::mem::size_of::<SOCKADDR_IN6>() => {
            let value = unsafe { std::ptr::read_unaligned(p as *const SOCKADDR_IN6) };
            Some(std::net::Ipv6Addr::from(unsafe { value.sin6_addr.u.Byte }).to_string())
        }
        _ => None,
    }
}
fn registry(subkey: &str, name: &str) -> Option<String> {
    let subkey: Vec<_> = subkey.encode_utf16().chain(Some(0)).collect();
    let name: Vec<_> = name.encode_utf16().chain(Some(0)).collect();
    let mut buffer = [0u16; 257];
    let mut bytes = (buffer.len() * 2) as u32;
    if unsafe {
        RegGetValueW(
            HKEY_LOCAL_MACHINE,
            PCWSTR(subkey.as_ptr()),
            PCWSTR(name.as_ptr()),
            RRF_RT_REG_SZ,
            None,
            Some(buffer.as_mut_ptr().cast()),
            Some(&mut bytes),
        )
    }
    .0 != 0
    {
        None
    } else {
        Some(wide(&buffer))
    }
}
pub fn inventory() -> Inventory {
    let time = now();
    let text = |value: Option<String>, source| {
        value
            .filter(|s| !s.is_empty())
            .map(|value| Sample::ready(value, source, None, time))
            .unwrap_or_else(|| unavailable(source, None))
    };
    let mut ram_kb = 0;
    let installed_ram_bytes = if unsafe { GetPhysicallyInstalledSystemMemory(&mut ram_kb) }.is_ok()
    {
        ram_kb
            .checked_mul(1024)
            .map(|bytes| {
                integer(
                    bytes,
                    "GetPhysicallyInstalledSystemMemory",
                    Some("bytes"),
                    time,
                )
            })
            .unwrap_or_else(|| unavailable("GetPhysicallyInstalledSystemMemory", Some("bytes")))
    } else {
        unavailable("GetPhysicallyInstalledSystemMemory", Some("bytes"))
    };
    let mut core_bytes = 0;
    let _ =
        unsafe { GetLogicalProcessorInformationEx(RelationProcessorCore, None, &mut core_bytes) };
    let mut cores = None;
    if core_bytes > 0 && core_bytes <= 1024 * 1024 {
        let mut buffer = vec![0u64; (core_bytes as usize).div_ceil(8)];
        if unsafe {
            GetLogicalProcessorInformationEx(
                RelationProcessorCore,
                Some(buffer.as_mut_ptr().cast()),
                &mut core_bytes,
            )
        }
        .is_ok()
        {
            let bytes = unsafe {
                std::slice::from_raw_parts(buffer.as_ptr().cast::<u8>(), core_bytes as usize)
            };
            let mut offset = 0;
            let mut count = 0;
            while offset + 8 <= bytes.len() {
                let size =
                    u32::from_le_bytes(bytes[offset + 4..offset + 8].try_into().unwrap_or([0; 4]))
                        as usize;
                if size < 8 || offset + size > bytes.len() {
                    break;
                }
                count += 1;
                offset += size;
            }
            if offset == bytes.len() {
                cores = Some(count);
            }
        }
    }
    let mut gpu_names = Vec::new();
    let mut vram = 0u64;
    let mut gpu_ok = false;
    let mut gpu_truncated = false;
    if let Ok(factory) = unsafe { CreateDXGIFactory1::<IDXGIFactory1>() } {
        for index in 0..16 {
            let Ok(adapter) = (unsafe { factory.EnumAdapters1(index) }) else {
                break;
            };
            let Ok(desc) = (unsafe { adapter.GetDesc1() }) else {
                continue;
            };
            if desc.Flags & DXGI_ADAPTER_FLAG_SOFTWARE.0 as u32 != 0 {
                continue;
            }
            gpu_ok = true;
            gpu_names.push(wide(&desc.Description));
            vram = vram.saturating_add(desc.DedicatedVideoMemory as u64);
        }
        gpu_truncated = unsafe { factory.EnumAdapters1(16) }.is_ok();
    }
    let (board, bios) = firmware();
    let os = "SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion";
    let version = registry(os, "CurrentBuildNumber").map(|build| {
        format!(
            "Windows-reported build {build}{}",
            registry(os, "DisplayVersion")
                .map(|v| format!(" ({v})"))
                .unwrap_or_default()
        )
    });
    Inventory {
        state: if gpu_truncated {
            SampleState::Truncated
        } else {
            SampleState::Partial
        },
        sampled_at_ms: Some(time),
        cpu_name: text(
            registry(
                "HARDWARE\\DESCRIPTION\\System\\CentralProcessor\\0",
                "ProcessorNameString",
            ),
            "registry.ProcessorNameString",
        ),
        physical_cores: cores
            .map(|value| integer(value, "GetLogicalProcessorInformationEx", None, time))
            .unwrap_or_else(|| unavailable("GetLogicalProcessorInformationEx", None)),
        logical_processors: integer(
            unsafe { GetActiveProcessorCount(ALL_PROCESSOR_GROUPS) } as u64,
            "GetActiveProcessorCount.ALL_PROCESSOR_GROUPS",
            None,
            time,
        ),
        installed_ram_bytes,
        gpu_names: if gpu_ok {
            let mut sample = Sample::ready(gpu_names, "DXGI", None, time);
            if gpu_truncated {
                sample.state = SampleState::Truncated;
                sample.reason = Some(SampleReason::LimitReached);
            }
            sample
        } else {
            unavailable("DXGI", None)
        },
        vram_bytes: if gpu_ok {
            let mut sample = integer(vram, "DXGI.DedicatedVideoMemory", Some("bytes"), time);
            if gpu_truncated && sample.value.is_some() {
                sample.state = SampleState::Truncated;
                sample.reason = Some(SampleReason::LimitReached);
            }
            sample
        } else {
            unavailable("DXGI.DedicatedVideoMemory", Some("bytes"))
        },
        board: text(board, "SMBIOS.type2.vendor_model"),
        bios: text(bios, "SMBIOS.type0.vendor_version"),
        windows_version: text(version, "registry.Windows_reported"),
    }
}
fn firmware() -> (Option<String>, Option<String>) {
    let provider = FIRMWARE_TABLE_PROVIDER(u32::from_be_bytes(*b"RSMB"));
    let size = unsafe { GetSystemFirmwareTable(provider, 0, None) };
    if size < 8 || size > 1024 * 1024 {
        return (None, None);
    }
    let mut bytes = vec![0; size as usize];
    if unsafe { GetSystemFirmwareTable(provider, 0, Some(&mut bytes)) } != size {
        return (None, None);
    }
    super::system_info::smbios_allowlist(&bytes)
}
