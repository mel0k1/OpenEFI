export interface CpuInfo {
  manufacturer: string;
  name: string;
  codename: string | null;
  cores: number;
  threads: number;
  avx2: boolean;
}

export interface GpuInfo {
  name: string;
  vendor_id: string;
  device_id: string;
  device_type: string;
  codename: string | null;
  max_darwin: number | null;
  note: string | null;
}

export interface DeviceInfo {
  name: string;
  vendor_id: string;
  device_id: string;
  kind: string;
  note: string | null;
}

export interface HardwareReport {
  os: string;
  platform: string;
  board_name: string;
  cpu: CpuInfo;
  memory_gb: number;
  gpus: GpuInfo[];
  network: DeviceInfo[];
  audio: DeviceInfo[];
  bluetooth: DeviceInfo[];
  warnings: string[];
}

export interface KextEntry {
  name: string;
  description: string;
  category: string;
  required: boolean;
  selected: boolean;
  status: string;
  requires: string[];
  repo: string | null;
}

export interface MacosVersion {
  number: string;
  name: string;
  darwin: number;
  supported: boolean;
  reason: string;
}

export interface MacDevice {
  name: string;
  cpu: string;
  cpu_gen: string;
  dgpu: string | null;
  initial: string;
  last: string | null;
}

export interface SmbiosData {
  model: string;
  serial: string;
  mlb: string;
  rom: string;
  uuid: string;
}

export interface SmbiosRecommendation {
  recommended: string;
  reason: string;
}

export async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<T>(cmd, args);
}

export interface SsdtEntry {
  name: string;
  file: string;
  description: string;
  kind: string;
  url: string;
  selected: boolean;
}
