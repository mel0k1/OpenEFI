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
  /** Системный драйвер — ставится в macOS, не в EFI */
  non_efi: boolean;
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
  /** Имя файла в репозитории Dortania; пусто — автоскачивания нет */
  repo_file: string;
  description: string;
  kind: string;
  url: string;
  selected: boolean;
}

export interface SsdtDlItem {
  name: string;
  file: string;
  repo_file: string;
}

export interface KextDownloadItem {
  name: string;
  repo: string | null;
}

export interface KextDlResult {
  name: string;
  ok: boolean;
  version: string | null;
  installed: string[];
  message: string | null;
}

export interface KextsToEfiReport {
  kexts_dir: string;
  results: KextDlResult[];
  backup_path?: string | null;
  audit_warnings?: string[];
}

export interface KextDlProgress {
  done: number;
  total: number;
  kext: string;
}

export interface SsdtsToEfiReport {
  acpi_dir: string;
  results: KextDlResult[];
  config_path: string | null;
  backup_made: boolean;
  registered: number;
  backup_path?: string | null;
  audit_warnings?: string[];
}

export interface UtbMapReport {
  kexts_dir: string;
  config_path: string | null;
  backup_made: boolean;
  utbmap_registered: boolean;
  utbdefault_disabled: boolean;
}

// --- BIOS-чеклист ---

export interface BiosItem {
  id: string;
  /** Название пункта в реальном BIOS (не переводится) */
  option: string;
  /** enable | disable | check */
  action: "enable" | "disable" | "check";
  /** required | recommended | optional */
  priority: "required" | "recommended" | "optional";
  applies: boolean;
  note: string | null;
}

export interface BiosChecklist {
  items: BiosItem[];
}

export interface BiosStatus {
  secure_boot: boolean | null;
  /** "uefi" | "legacy" */
  boot_mode: string | null;
  notes: string[];
}

// --- Защита EFI ---

export interface BootloaderEntry {
  name: string;
  path: string;
  /** opencore | clover | windows | linux | refind | other */
  kind: string;
}

export interface EfiAudit {
  efi_dir: string;
  found: boolean;
  looks_like_esp: boolean;
  opencore: boolean;
  kexts_count: number;
  acpi_count: number;
  has_config: boolean;
  bootloaders: BootloaderEntry[];
  shared_bootx64: boolean;
  warnings: string[];
}

export interface EfiBackupReport {
  backup_path: string;
  files: number;
}

// --- Сборка полной EFI одной кнопкой ---

export interface FullEfiProgress {
  stage: string;
  message: string;
  percent: number;
}

/** installed | updated | skipped_same | skipped_foreign */
export type Bootx64Outcome = "installed" | "updated" | "skipped_same" | "skipped_foreign";

export interface FullEfiReport {
  efi_dir: string;
  oc_dir: string;
  opencore_version: string | null;
  files_installed: string[];
  bootx64: Bootx64Outcome;
  bootx64_note: string | null;
  config_source: string;
  config_path: string | null;
  config_patched: boolean;
  config_backup: boolean;
  canopy_enabled: boolean;
  resources_installed: string[];
  backup_path?: string | null;
  audit_warnings?: string[];
  warnings: string[];
}
