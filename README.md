# OpenEFI

<div align="center">

**Удобный подбор конфигурации EFI для хакинтоша**

Определение железа · Подбор кекстов · SMBIOS · Генерация серийников

[![Release](https://img.shields.io/github/v/release/mel0k1/OpenEFI?label=Release&color=blue)](https://github.com/mel0k1/OpenEFI/releases/latest)
[![CI](https://img.shields.io/github/actions/workflow/status/mel0k1/OpenEFI/build.yml?label=CI&logo=githubactions&logoColor=white)](https://github.com/mel0k1/OpenEFI/actions)
[![License](https://img.shields.io/github/license/mel0k1/OpenEFI?label=License&color=green)](LICENSE)
[![Platform](https://img.shields.io/badge/Platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey)](releases)

</div>

---

**OpenEFI** — кроссплатформенное desktop-приложение, которое упрощает первый и самый муторный этап сборки хакинтоша: определение железа и подбор его окружения. Программа сканирует PCI-устройства, процессор и память, подбирает совместимые кексты по device ID (как это делает OpCore-Simplify), рекомендует SMBIOS-модель по поколению CPU и генерирует серийные номера по алгоритму macserial из OpenCorePkg.

Проект написан на **Rust + Tauri 2** с интерфейсом на **React + shadcn/ui**. Логика подбора перенесена из OpCore-Simplify (Python) и RapidEFI-Tool (Flutter), но реализована нативно — без сторонних бинарников и интерпретаторов.

## Возможности

- **Сканирование железа** — CPU (производитель, кодовое имя, ядра, AVX2), GPU, сеть, аудио, Bluetooth на Windows, macOS и Linux; база PCI ID встроена
- **Подбор кекстов** — Lilu, VirtualSMC, WhateverGreen, AppleALC, сетевые и Bluetooth-драйверы: по вашим PCI ID, с учётом целевой версии macOS и зависимостей
- **Совместимость macOS** — оценка максимальной версии по GPU (NVIDIA/AMD/Intel iGPU) и поколению процессора
- **SMBIOS** — рекомендация модели Mac по железу (порт алгоритма OpCore-Simplify) + каталог из 77 моделей
- **Генерация серийников** — Serial, MLB (с контрольной суммой), ROM и UUID: порт алгоритма macserial на Rust
- **Экспорт отчёта** — JSON со всеми данными для config.plist и форумов

## Скриншот

> Интерфейс: тёмная/светлая тема, мастер из 5 шагов (скан → железо → кексты → SMBIOS → экспорт)

## Сборка из исходников

Нужны [Node.js](https://nodejs.org) 18+, [Rust](https://rustup.rs) и системные зависимости Tauri ([инструкция](https://tauri.app/start/prerequisites/)):

```bash
git clone https://github.com/mel0k1/OpenEFI.git
cd OpenEFI
npm install
npm run tauri dev    # режим разработки
npm run tauri build  # релизная сборка
```

Готовые сборки под Windows, macOS (universal) и Linux публикуются в [Releases](releases) автоматически через GitHub Actions.

## Тесты

Ядро (openefi-core) покрыто юнит-тестами: алгоритм серийников, кодовые имена CPU, база PCI ID, рекомендации SMBIOS:

```bash
cd src-tauri/core
cargo test
```

## Архитектура

```
openefi/
├── src/                  # React-фронтенд (Vite + Tailwind + shadcn/ui)
│   ├── components/steps/ # шаги мастера
│   └── hooks/            # тема, буфер обмена
├── src-tauri/
│   ├── src/              # Tauri-команды (тонкий слой над ядром)
│   └── core/             # openefi-core: вся логика на Rust
│       ├── hardware.rs   # сканирование PCI/USB/CPU (3 ОС)
│       ├── kexts.rs      # база кекстов и алгоритм подбора
│       ├── smbios.rs     # SMBIOS + macserial
│       ├── cpu_codename.rs
│       └── db.rs         # встроенные базы (pci.ids, device IDs, модели Mac)
└── .github/workflows/    # CI: сборка под 3 ОС + релизы
```

## Дорожная карта

- [ ] Генерация config.plist (ACPI/Kernel/PlatformInfo-разделы)
- [ ] Подбор SSDT по железу из репозитория Dortania
- [ ] Скачивание кекстов прямо в EFI-папку
- [ ] USB-маппинг: интеграция с USBToolBox

## Благодарности

- [OpCore-Simplify](https://github.com/lzhoang2801/OpCore-Simplify) — логика подбора кекстов и SMBIOS
- [RapidEFI-Tool](https://github.com/JeoJay127/RapidEFI-Tool) — идеи интерфейса
- [OpenCorePkg](https://github.com/acidanthera/OpenCorePkg) — алгоритм macserial
- [Dortania](https://dortania.github.io/) — документация по хакинтошу
- [Acidanthera](https://github.com/acidanthera) — OpenCore и кексты
- [shadcn/ui](https://ui.shadcn.com) — компоненты интерфейса

## Лицензия

[GPL-3.0](LICENSE) © mel0k1
