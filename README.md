<div align="center">

<img src="https://img.shields.io/badge/OpenEFI-хакинтош%20без%20боли-5865F2?style=for-the-badge&labelColor=1a1b26" alt="OpenEFI" width="100%">

# ⚡ OpenEFI

**Удобный подбор конфигурации EFI для хакинтоша**

*Сканирование железа · Подбор кекстов · SMBIOS · Генерация серийников*

[![Release](https://img.shields.io/github/v/release/mel0k1/OpenEFI?style=flat-square&label=%F0%9F%93%A6%20Release&color=181717)](https://github.com/mel0k1/OpenEFI/releases/latest)
[![CI](https://img.shields.io/github/actions/workflow/status/mel0k1/OpenEFI/build.yml?style=flat-square&label=%E2%99%BB%EF%B8%8F%20CI&logo=githubactions&logoColor=white)](https://github.com/mel0k1/OpenEFI/actions)
[![Tests](https://img.shields.io/badge/%F0%9F%A7%AA%20Tests-36%20passing-brightgreen?style=flat-square)](https://github.com/mel0k1/OpenEFI/actions)
[![License](https://img.shields.io/github/license/mel0k1/OpenEFI?style=flat-square&label=%F0%9F%93%84%20License&color=green)](LICENSE)
[![Platform](https://img.shields.io/badge/%F0%9F%96%A5%EF%B8%8F%20Windows%20%7C%20macOS%20%7C%20Linux-blue?style=flat-square)](https://github.com/mel0k1/OpenEFI/releases)

**Rust** · **Tauri 2** · **React** · **shadcn/ui** · **GPL-3.0**

</div>

---

## 🤔 Зачем это нужно

Первый и самый муторный этап сборки хакинтоша — разобраться, что за железо у вас стоит, какие кексты к нему подойдут и какой SMBIOS выбрать. Обычно для этого нужно прошерстить гайды Dortania, вручную сверять device ID по таблицам и молиться, что ничего не забыл.

**OpenEFI делает это за 30 секунд:** сканирует ваше железо, подбирает совместимые кексты по device ID, рекомендует SMBIOS-модель по поколению процессора и генерирует серийные номера — в один нативный бинарник без Python, интерпретаторов и сторонних зависимостей.

## ✨ Возможности

| | Функция | Что делает |
|---|---|---|
| 🔍 | **Сканирование железа** | CPU (производитель, кодовое имя, ядра, AVX2), GPU, сеть, аудио, Bluetooth — на Windows, macOS и Linux. База PCI ID встроена |
| 🧩 | **Подбор кекстов** | 72 кекста: Lilu, VirtualSMC, WhateverGreen, AppleALC, сетевые, Bluetooth и системные драйверы — по вашим PCI ID, с учётом целевой версии macOS и зависимостей |
| 📥 | **Установка в EFI** | Скачивание выбранных кекстов последними релизами с GitHub и распаковка .kext прямо в EFI/OC/Kexts — без ручного копирования |
| 🍎 | **Совместимость macOS** | Оценка максимальной версии по GPU (NVIDIA / AMD / Intel iGPU) и поколению процессора |
| 💻 | **SMBIOS** | Рекомендация модели Mac по железу + каталог из 77 моделей с полными данными |
| 🔑 | **Генерация серийников** | Serial, MLB (с контрольной суммой), ROM и UUID — полный порт алгоритма macserial из OpenCorePkg на Rust |
| 📋 | **config.plist** | Полный конфиг OpenCore на базе Sample.plist: ACPI/Add, Kernel/Add (правильный порядок кекстов), DeviceProperties для iGPU, PlatformInfo с вашими серийниками |
| 🧾 | **Подбор SSDT** | Матрица Dortania по железу: PLUG, EC-USBX, AWAC, PMC, XOSI и другие + скачивание прекомпилированных .aml прямо в EFI/OC/ACPI одной кнопкой, с регистрацией в config.plist |
| 🔌 | **USB-маппинг** | Интеграция с USBToolBox: скачивание утилиты для Windows/macOS, установка готового UTBMap.kext в EFI и правка config.plist (UTBMap включён, UTBDefault отключён) |
| 📤 | **Экспорт отчёта** | JSON со всеми данными — для помощи на форумах |
| 🌗 | **Тёмная и светлая тема** | Переключение одним кликом |
| 🌍 | **Русский и английский** | Переключение языка в настройках |

## 🖼️ Интерфейс

> Мастер из 5 шагов: **Скан → Железо → macOS → Кексты → SMBIOS → Экспорт**

```text
┌─────────────────────────────────────────────┐
│  OpenEFI — сборка EFI для хакинтоша    🌙   │
├─────────────────────────────────────────────┤
│  ① Скан железа      ▸ все устройства        │
│  ② Выбор macOS      ▸ целевая версия        │
│  ③ Кексты           ▸ подобраны по ID       │
│  ④ SMBIOS           ▸ iMac19,1 + серийники  │
│  ⑤ Экспорт          ▸ JSON-отчёт            │
└─────────────────────────────────────────────┘
```

*Скриншоты появятся в первом релизе.*

## 🚀 Быстрый старт

Скачайте готовую сборку из [Releases](https://github.com/mel0k1/OpenEFI/releases/latest) — она соберётся автоматически под вашу ОС:

| ОС | Файл |
|---|---|
| Windows | `OpenEFI_x.y.z_x64-setup.exe` / `.msi` |
| macOS (Intel + Apple Silicon) | `OpenEFI_x.y.z_universal.app.tar.gz` / `.dmg` |
| Linux | `OpenEFI_x.y.z_amd64.AppImage` / `.deb` / `.rpm` |

## 🛠️ Сборка из исходников

Нужны [Node.js](https://nodejs.org) 18+, [Rust](https://rustup.rs) и системные зависимости Tauri ([инструкция](https://tauri.app/start/prerequisites/)):

```bash
git clone https://github.com/mel0k1/OpenEFI.git
cd OpenEFI
npm install
npm run tauri dev    # режим разработки
npm run tauri build  # релизная сборка
```

## 🧪 Тесты

Ядро `openefi-core` покрыто юнит-тестами: алгоритм серийников (с контрольными суммами), кодовые имена CPU, база PCI ID, рекомендации SMBIOS, матрица BIOS-чеклиста, аудит EFI. CI гоняет их на всех трёх ОС при каждом пуше:

```bash
cd src-tauri/core
cargo test
```

## 🏗️ Архитектура

```text
openefi/
├── src/                  # React-фронтенд (Vite + Tailwind + shadcn/ui)
│   ├── components/steps/ # шаги мастера
│   └── hooks/            # тема, буфер обмена
├── src-tauri/
│   ├── src/              # Tauri-команды (тонкий слой над ядром)
│   └── core/             # openefi-core: вся логика на Rust
│       ├── hardware.rs   # сканирование PCI/USB/CPU (3 ОС)
│       ├── kexts.rs      # база кекстов и алгоритм подбора
│       ├── smbios.rs     # SMBIOS + порт macserial
│       ├── ssdts.rs      # матрица SSDT из гайда Dortania
│       ├── config.rs     # генерация config.plist (Sample.plist + ваши данные)
│       ├── bios.rs       # BIOS-чеклист под железо + живые проверки Secure Boot/UEFI
│       ├── efiprotect.rs # аудит EFI, обнаружение чужих загрузчиков, бэкап EFI/OC
│       ├── opencore.rs   # сборка полной EFI: OpenCorePkg + OcBinaryData одной кнопкой
│       ├── cpu_codename.rs
│       └── db.rs         # встроенные базы (pci.ids, device IDs, модели Mac)
└── .github/workflows/    # CI: тесты + сборка под 3 ОС + авто-релизы
```

## 🗺️ Дорожная карта

- [x] Генерация `config.plist` (ACPI / Kernel / DeviceProperties / PlatformInfo) — v0.2.0
- [x] Подбор SSDT по железу из матрицы Dortania — v0.2.0
- [x] Скачивание кекстов прямо в EFI-папку (GitHub Releases → EFI/OC/Kexts) — v0.3.0
- [x] USB-маппинг: интеграция с USBToolBox (утилита + UTBMap в EFI + правка конфига) — v0.4.0
- [x] Английская локализация (переключатель языка в настройках) — v0.4.0
- [x] Скачивание SSDT .aml из репозитория Dortania в EFI/OC/ACPI + правка ACPI/Add — v0.5.0
- [x] BIOS-чеклист: что включить/выключить в BIOS под ваше железо + живая проверка Secure Boot и режима загрузки — v0.6.0
- [x] Защита EFI: аудит папки (чужие загрузчики), автоматический бэкап EFI/OC перед изменениями, гарантия невмешательства в Windows/Linux-загрузчики — v0.6.0
- [x] Сборка полной EFI одной кнопкой: OpenCorePkg + OcBinaryData скачиваются и раскладываются автоматически, драйверы регистрируются в config.plist — v0.7.0

## 🙏 Благодарности

- [OpCore-Simplify](https://github.com/lzhoang2801/OpCore-Simplify) — логика подбора кекстов и SMBIOS
- [RapidEFI-Tool](https://github.com/JeoJay127/RapidEFI-Tool) — идеи интерфейса
- [OpenCorePkg](https://github.com/acidanthera/OpenCorePkg) — алгоритм macserial
- [Dortania](https://dortania.github.io/) — лучшая документация по хакинтошу
- [Acidanthera](https://github.com/acidanthera) — OpenCore и кексты
- [shadcn/ui](https://ui.shadcn.com) — компоненты интерфейса

## 📄 Лицензия

[GPL-3.0](LICENSE) © mel0k1
