<div align="center">

<img src="https://img.shields.io/badge/OpenEFI-хакинтош%20без%20боли-5865F2?style=for-the-badge&labelColor=1a1b26" alt="OpenEFI" width="100%">

# ⚡ OpenEFI

**Удобный подбор конфигурации EFI для хакинтоша**

*Сканирование железа · Подбор кекстов · SMBIOS · Генерация серийников*

[![Release](https://img.shields.io/github/v/release/mel0k1/OpenEFI?style=flat-square&label=%F0%9F%93%A6%20Release&color=181717)](https://github.com/mel0k1/OpenEFI/releases/latest)
[![CI](https://img.shields.io/github/actions/workflow/status/mel0k1/OpenEFI/build.yml?style=flat-square&label=%E2%99%BB%EF%B8%8F%20CI&logo=githubactions&logoColor=white)](https://github.com/mel0k1/OpenEFI/actions)
[![Tests](https://img.shields.io/badge/%F0%9F%A7%AA%20Tests-11%20passing-brightgreen?style=flat-square)](https://github.com/mel0k1/OpenEFI/actions)
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
| 🧩 | **Подбор кекстов** | Lilu, VirtualSMC, WhateverGreen, AppleALC, сетевые и Bluetooth-драйверы — по вашим PCI ID, с учётом целевой версии macOS и зависимостей |
| 🍎 | **Совместимость macOS** | Оценка максимальной версии по GPU (NVIDIA / AMD / Intel iGPU) и поколению процессора |
| 💻 | **SMBIOS** | Рекомендация модели Mac по железу + каталог из 77 моделей с полными данными |
| 🔑 | **Генерация серийников** | Serial, MLB (с контрольной суммой), ROM и UUID — полный порт алгоритма macserial из OpenCorePkg на Rust |
| 📤 | **Экспорт отчёта** | JSON со всеми данными — для config.plist и помощи на форумах |
| 🌗 | **Тёмная и светлая тема** | Переключение одним кликом, интерфейс на русском |

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

Ядро `openefi-core` покрыто юнит-тестами: алгоритм серийников (с контрольными суммами), кодовые имена CPU, база PCI ID, рекомендации SMBIOS. CI гоняет их на всех трёх ОС при каждом пуше:

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
│       ├── cpu_codename.rs
│       └── db.rs         # встроенные базы (pci.ids, device IDs, модели Mac)
└── .github/workflows/    # CI: тесты + сборка под 3 ОС + авто-релизы
```

## 🗺️ Дорожная карта

- [ ] Генерация `config.plist` (разделы ACPI / Kernel / PlatformInfo)
- [ ] Подбор SSDT по железу из репозитория Dortania
- [ ] Скачивание кекстов прямо в EFI-папку
- [ ] USB-маппинг: интеграция с USBToolBox
- [ ] Английская локализация

## 🙏 Благодарности

- [OpCore-Simplify](https://github.com/lzhoang2801/OpCore-Simplify) — логика подбора кекстов и SMBIOS
- [RapidEFI-Tool](https://github.com/JeoJay127/RapidEFI-Tool) — идеи интерфейса
- [OpenCorePkg](https://github.com/acidanthera/OpenCorePkg) — алгоритм macserial
- [Dortania](https://dortania.github.io/) — лучшая документация по хакинтошу
- [Acidanthera](https://github.com/acidanthera) — OpenCore и кексты
- [shadcn/ui](https://ui.shadcn.com) — компоненты интерфейса

## 📄 Лицензия

[GPL-3.0](LICENSE) © mel0k1
