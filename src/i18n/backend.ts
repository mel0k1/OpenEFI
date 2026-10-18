// Перевод строк, приходящих из бэкенда (openefi-core): описания кекстов,
// категории, SSDT, причины SMBIOS-рекомендаций, заметки железа, ошибки.
// Ключ — исходная русская строка. Динамические строки (с подстановками)
// обрабатываются паттернами BACKEND_PATTERNS.

export const BACKEND_EN: Record<string, string> = {
  // --- Категории кекстов ---
  "Обязательные": "Required",
  "Плагины VirtualSMC": "VirtualSMC plug-ins",
  "Графика": "Graphics",
  "Аудио": "Audio",
  "Wi-Fi": "Wi-Fi",
  "Bluetooth": "Bluetooth",
  "Ethernet": "Ethernet",
  "Дополнительно": "Miscellaneous",
  "USB": "USB",
  "Хранилище": "Storage",
  "Карт-ридеры": "Card readers",
  "Ввод": "Input",
  "Брендовые": "Vendor-specific",

  // --- Виды SSDT ---
  "Обязательна": "Required",
  "Рекомендуется": "Recommended",
  "Опциональна": "Optional",

  // --- Описания кекстов ---
  "Патч-движок ядра: базовый слой для остальных плагинов":
    "Kernel patching engine: base layer for other plug-ins",
  "Эмулятор SMC — без него macOS не запустится": "SMC emulator — macOS will not boot without it",
  "Датчики температуры CPU": "CPU temperature sensors",
  "Мониторинг датчиков и управление оборотами вентиляторов":
    "Sensor monitoring and fan speed control",
  "Статус и уровень заряда батареи": "Battery status and charge level",
  "Датчик внешнего освещения": "Ambient light sensor",
  "Датчики вентиляторов на ноутбуках Dell": "Fan sensors on Dell laptops",
  "Датчики температуры GPU AMD": "AMD GPU temperature sensors",
  "Фиксы видеокарт Intel/NVIDIA/AMD": "Intel/NVIDIA/AMD graphics fixes",
  "Поддержка iGPU AMD (Vega APUs)": "AMD iGPU support (Vega APUs)",
  "Поддержка dGPU AMD RDNA 2 (RX 6000)": "AMD RDNA 2 dGPU support (RX 6000)",
  "Нативный звук на unsupported кодеках": "Native audio on unsupported codecs",
  "Фиксы для не-нативных Broadcom Wi-Fi карт": "Fixes for non-native Broadcom Wi-Fi cards",
  "Наследуемый стек Wi-Fi для Sonoma/Tahoe": "Legacy Wi-Fi stack for Sonoma/Tahoe",
  "Broadcom Wi-Fi на macOS Tahoe": "Broadcom Wi-Fi on macOS Tahoe",
  "Intel Wi-Fi с нативным интерфейсом macOS": "Intel Wi-Fi with the native macOS interface",
  "Intel Wi-Fi через приложение HeliPort": "Intel Wi-Fi via the HeliPort app",
  "Прошивка для Intel Bluetooth": "Firmware for Intel Bluetooth",
  "Фиксы багов Intel Bluetooth": "Intel Bluetooth bug fixes",
  "Поддержка сторонних Bluetooth на Monterey+": "Third-party Bluetooth support on Monterey+",
  "PatchRAM прошивки для Broadcom Bluetooth": "PatchRAM firmware for Broadcom Bluetooth",
  "PatchRAM обновления для Broadcom USB Bluetooth": "PatchRAM updates for Broadcom USB Bluetooth",
  "PatchRAM обновления для Broadcom USB Bluetooth (10.15+)":
    "PatchRAM updates for Broadcom USB Bluetooth (10.15+)",
  "Переключатель Broadcom Bluetooth на старых версиях":
    "Broadcom Bluetooth switch on older versions",
  "Прошивка для Realtek Bluetooth": "Firmware for Realtek Bluetooth",
  "Драйвер Intel Ethernet (Mausi)": "Intel Ethernet driver (Mausi)",
  "Драйвер Intel 2.5G Ethernet (i225/i226)": "Intel 2.5G Ethernet driver (i225/i226)",
  "Драйвер Intel IGB серверных контроллеров": "Intel IGB server controller driver",
  "Драйвер Atheros E2200": "Atheros E2200 driver",
  "Драйвер Realtek RTL8111/8168": "Realtek RTL8111/8168 driver",
  "Драйвер Realtek RTL8100": "Realtek RTL8100 driver",
  "Драйвер Realtek 2.5G RTL8125": "Realtek 2.5G RTL8125 driver",
  "Драйвер Broadcom BCM57XX Ethernet": "Broadcom BCM57XX Ethernet driver",
  "Блокировка лишних процессов, разблокировка функций (cpuname, memtab)":
    "Blocks unneeded processes, unlocks features (cpuname, memtab)",
  "Отключение AppleMCEReporter — против паники на AMD и многосокетных системах":
    "Disables AppleMCEReporter — prevents panics on AMD and multi-socket systems",
  "Перестройка топологии ядер Intel (E/P-core) для 6+ ядер":
    "Rebuilds Intel core topology (E/P-cores) for 6+ cores",
  "Синхронизация TSC на Intel HEDT": "TSC synchronization on Intel HEDT",
  "Синхронизация TSC на AMD": "TSC synchronization on AMD",
  "Plug & play синхронизация TSC на AMD и Intel": "Plug & play TSC sync on AMD and Intel",
  "Патчи для установки Rosetta на системах без AVX2":
    "Patches to install Rosetta on non-AVX2 systems",
  "Фиксы совместимости и энергопотребления NVMe SSD": "NVMe SSD compatibility and power fixes",
  "Обработка клавиш яркости без DSDT-патчей": "Brightness key handling without DSDT patches",
  "Динамическое управление питанием CPU (нужен CPUFriendDataProvider)":
    "Dynamic CPU power management (needs CPUFriendDataProvider)",
  "Разблокировка Sidecar, Continuity и прочего на unsupported железе":
    "Unlocks Sidecar, Continuity and more on unsupported hardware",
  "Убирает лаги аутентификации на системах с Touch ID board-ids":
    "Removes authentication lag on systems with Touch ID board-ids",
  "Чтение полей Embedded Controller длиннее 1 байта (батарея)":
    "Reads Embedded Controller fields longer than 1 byte (battery)",
  "Фиксы гибернации и спящего режима": "Hibernation and sleep fixes",
  "Инструмент маппинга USB-портов": "USB port mapping tool",
  "Временный кекст: все USB-порты без лимита (заменить своим UTBMap)":
    "Temporary kext: all USB ports without a limit (replace with your UTBMap)",
  "Ваш маппинг USB из USBToolBox — добавьте через помощник USB на этом шаге":
    "Your USB map from USBToolBox — add it via the USB helper on this step",
  "Поддержка нестандартных xHCI-контроллеров": "Support for non-standard xHCI controllers",
  "Фиксы USB 3.0 на некоторых AMD APU": "USB 3.0 fixes on some AMD APUs",
  "Поддержка редких SATA-контроллеров": "Support for rare SATA controllers",
  "Улучшенная поддержка отдельных SATA-контроллеров":
    "Improved support for certain SATA controllers",
  "Драйвер ридеров карт Realtek (PCIe/USB)": "Realtek card reader driver (PCIe/USB)",
  "Отображение ридера Realtek в Системной информации":
    "Shows the Realtek reader in System Information",
  "Клавиатуры, тачпады и мыши PS/2": "PS/2 keyboards, trackpads and mice",
  "Поддержка I2C-контроллеров и тачпадов": "I2C controller and trackpad support",
  "Сателлит для HID I2C / ELAN1200+ устройств": "Satellite for HID I2C / ELAN1200+ devices",
  "Тачпады Synaptics по SMBus/I2C": "Synaptics trackpads over SMBus/I2C",
  "ALS, подсветка клавиатуры и Fn-клавиши на ноутбуках ASUS":
    "ALS, keyboard backlight and Fn keys on ASUS laptops",
  "Сенсоры и фирменные функции Lenovo IdeaPad/Yoga/ThinkPad":
    "Sensors and vendor features for Lenovo IdeaPad/Yoga/ThinkPad",
  "Комплексная поддержка устройств Microsoft Surface":
    "Full support for Microsoft Surface devices",
  "Фикс OTA-обновлений системы (плагин Lilu). Конфликтует с WhateverGreen на AMD GPU — там нужен форк Carnations":
    "Fixes OTA system updates (Lilu plug-in). Conflicts with WhateverGreen on AMD GPUs — use the Carnations fork there",
  "Обход AMFI для legacy-кекстов OCLP — возвращает Atheros/Realtek Wi-Fi на новых macOS":
    "AMFI bypass for OCLP legacy kexts — brings back Atheros/Realtek Wi-Fi on newer macOS",
  "Платформа мультитач-жестов, основа для VoodooRMI и VoodooPS2":
    "Multitouch gestures platform, base for VoodooRMI and VoodooPS2",
  "Ридеры SD-карт на контроллерах Ricoh/Realtek RTSX":
    "SD card readers on Ricoh/Realtek RTSX controllers",
  "Metal-драйвер NVIDIA (GTX 16xx – RTX 50xx) для macOS Sequoia+ — ставится в систему, не в EFI":
    "NVIDIA Metal driver (GTX 16xx – RTX 50xx) for macOS Sequoia+ — installed into the system, not EFI",

  // --- Описания SSDT ---
  "Управление питанием процессора (XCPM) — обязательна на всех Intel":
    "CPU power management (XCPM) — required on all Intel systems",
  "Фейковый EC + питание USB для ноутбуков — обязательна":
    "Fake EC + USB power for laptops — required",
  "Фейковый EC + питание USB для десктопов — обязательна":
    "Fake EC + USB power for desktops — required",
  "Возврат классического RTC на чипсетах 300-й серии и новее — обязательна":
    "Classic RTC restore on 300-series chipsets and newer — required",
  "RTC-фикс: нужна для чипсетов AMD B550/A520/X570": "RTC fix: needed for AMD B550/A520/X570 chipsets",
  "Восстановление native NVRAM (чипсеты Z390/B360 и др.); не нужна для B365/H310/C246":
    "Restores native NVRAM (Z390/B360 etc.); not needed for B365/H310/C246",
  "Спуф O SMBus/OSI для I2C-тачпадов на ноутбуках (с VoodooI2C)":
    "OSI spoof for I2C trackpads on laptops (with VoodooI2C)",
  "Фиксы моста UNC на X99 (Haswell-E/Broadwell-E)": "UNC bridge fixes on X99 (Haswell-E/Broadwell-E)",
  "Отключение DMAR/VT-d без захода в BIOS — только если VT-d включён":
    "Disables DMAR/VT-d without entering the BIOS — only if VT-d is enabled",

  // --- Причины SMBIOS-рекомендаций ---
  "Универсальный выбор по умолчанию": "Universal default choice",
  "Lynnfield + дискретная графика": "Lynnfield + discrete graphics",
  "Серверный Xeon Westmere/Lynnfield": "Server Xeon Westmere/Lynnfield",
  "Sandy/Ivy Bridge: MacPro6,1 с dGPU": "Sandy/Ivy Bridge: MacPro6,1 with dGPU",
  "Arrandale/Clarksfield ноутбук": "Arrandale/Clarksfield laptop",
  "Sandy Bridge настольный": "Sandy Bridge desktop",
  "Sandy Bridge ноутбук": "Sandy Bridge laptop",
  "Ivy Bridge настольный": "Ivy Bridge desktop",
  "Ivy Bridge ноутбук": "Ivy Bridge laptop",
  "Haswell настольный": "Haswell desktop",
  "Haswell ноутбук": "Haswell laptop",
  "Broadwell настольный": "Broadwell desktop",
  "Broadwell ноутбук": "Broadwell laptop",
  "Skylake ноутбук": "Skylake laptop",
  "Kaby Lake ноутбук": "Kaby Lake laptop",
  "Kaby Lake настольный": "Kaby Lake desktop",
  "Coffee Lake 8xxx ноутбук": "Coffee Lake 8xxx laptop",
  "Whiskey/Coffee Lake ноутбук": "Whiskey/Coffee Lake laptop",
  "Comet Lake настольный": "Comet Lake desktop",
  "Coffee Lake настольный + iGPU": "Coffee Lake desktop + iGPU",
  "Ice Lake ноутбук": "Ice Lake laptop",
  "Tiger Lake ноутбук": "Tiger Lake laptop",
  "Rocket Lake настольный + iGPU": "Rocket Lake desktop + iGPU",

  // --- Заметки железа ---
  "Kepler: нативно до Catalina 10.15; новее — только OCLP":
    "Kepler: native up to Catalina 10.15; newer — OCLP only",
  "Maxwell/Pascal: нативно до Monterey 12; новее — только OCLP":
    "Maxwell/Pascal: native up to Monterey 12; newer — OCLP only",
  "NVIDIA Turing/Ampere/Ada не поддерживаются macOS вовсе — понадобится AMD GPU":
    "NVIDIA Turing/Ampere/Ada are not supported by macOS at all — an AMD GPU is required",
  "RDNA 3: нативно с Ventura 13": "RDNA 3: native from Ventura 13",
  "RDNA 2: нативно до Sequoia 15, нужен NootRX/WhateverGreen":
    "RDNA 2: native up to Sequoia 15, NootRX/WhateverGreen required",
  "Polaris: нативно до Ventura 13; в Sonoma 14+ — только через OCLP":
    "Polaris: native up to Ventura 13; Sonoma 14+ — OCLP only",
  "Atheros Wi-Fi работает только через OCLP (macOS до El Capitan нативно)":
    "Atheros Wi-Fi works only via OCLP (native up to El Capitan)",
  "HDA-контроллер": "HDA controller",
  "Сетевой адаптер": "Network adapter",
  "USB Wi-Fi адаптер": "USB Wi-Fi adapter",
  "USB Wi-Fi адаптеры не поддерживаются macOS — нужен встроенный модуль":
    "USB Wi-Fi adapters are not supported by macOS — an internal module is required",
  "Определён через /proc/asound": "Detected via /proc/asound",
  "Выше лимита железа": "Above the hardware limit",

  // --- Сообщения загрузчика / ошибок (статичные) ---
  "лимит запросов GitHub API исчерпан — повторите позже":
    "GitHub API rate limit exhausted — try again later",
  "репозиторий не указан — скачайте вручную с сайта разработчика":
    "no repository specified — download manually from the developer's site",
  "маппинг добавляется через помощник USB на этом же шаге":
    "the map is added via the USB helper on this same step",
  "системный драйвер — устанавливается внутри macOS, не в EFI":
    "system driver — installed inside macOS, not into EFI",
  "в архиве нет .kext-бандлов": "no .kext bundles in the archive",
  "нет связи с сервером при скачивании": "no connection to the server while downloading",
  "архив слишком большой": "archive too large",
  "файл слишком большой": "file too large",
  "прекомпилированного файла нет в репозитории Dortania — соберите вручную по гайду":
    "no precompiled file in the Dortania repository — build it manually per the guide",
  "в config.plist нет секции ACPI": "no ACPI section in config.plist",
  "ACPI → Add в config.plist — не массив": "ACPI → Add in config.plist is not an array",
  "не создать ACPI → Add": "cannot create ACPI → Add",
  "неизвестная ОС": "unknown OS",
  "Сохранение отменено": "Save cancelled",
  "Выбор папки отменён": "Folder selection cancelled",
  "недопустимый путь": "invalid path",
  "недопустимый путь к папке": "invalid folder path",
  "недопустимый путь к кексту": "invalid kext path",

  // --- BIOS-чеклист: заметки ---
  "Должен быть режим AHCI — IDE/RST/RAID сломает обнаружение диска установщиком":
    "Must be AHCI — IDE/RST/RAID breaks disk detection for the installer",
  "На разных платах встречается как «Above 4G Memory» или «Mining Mode» — нужен для загрузки с AMD GPU":
    "Listed as “Above 4G Memory” or “Mining Mode” on some boards — required for AMD GPU boot",
  "macOS грузится только в чистом UEFI; если после выключения CSM диск пропал из списка загрузки — включите «Boot mode: UEFI only»":
    "macOS boots in pure UEFI only; if the disk disappears from the boot menu after disabling CSM — set “Boot mode: UEFI only”",
  "Navi 10 и старее (Polaris/Vega/Navi 10) не стартуют с включённым ReBAR":
    "Navi 10 and older (Polaris/Vega/Navi 10) will not boot with ReBAR enabled",
  "Для RX 6000 с кекстом NootRX ReBAR должен быть ВКЛЮЧЁН; с WhateverGreen — выключен":
    "For RX 6000 with the NootRX kext ReBAR must be ON; with WhateverGreen — OFF",
  "Не критично, но macOS не использует ReBAR — выключите, если возникнут артефакты":
    "Not critical, but macOS does not use ReBAR — disable it if you see artifacts",
  "На поколениях без SGX пункта не будет — просто пропустите":
    "Generations without SGX simply lack this option — skip it",
  "Если выключать не хотите — OpenCore умеет DisableIoMapper; на AMD этого пункта нет":
    "If you prefer to keep it on — OpenCore compensates with DisableIoMapper; AMD has no such option",
  "Часто спрятан в расширенном меню BIOS; если не найти — OpenCore компенсирует кварками AppleCpuPmCfgLock/AppleXcpmCfgLock":
    "Often hidden in the advanced BIOS menu; if missing — OpenCore compensates with the AppleCpuPmCfgLock/AppleXcpmCfgLock quirks",
  "macOS сам VT-x не использует, но пригодится для виртуалок; выключенным оставлять нет смысла":
    "macOS does not need VT-x itself, but VMs do — no reason to keep it off",
  "OpenCore TPM игнорирует; если Windows 11 стоит на том же ПК — не трогайте, иначе она перестанет загружаться":
    "OpenCore ignores TPM; if Windows 11 is installed on the same PC — do not touch it, or it will stop booting",
  "Или оставьте включённым — тогда AMD-конфиг должен содержать DisableIoMapper=true":
    "Or keep it enabled — then the AMD config must contain DisableIoMapper=true",
  "На AMD обязателен включённый NX — иначе ядро macOS падает при старте":
    "NX must be enabled on AMD — otherwise the macOS kernel crashes at boot",
  "Нужно 64 МБ под iGPU; часто спрятан — на некоторых ноутбуках правится только через мод BIOS/UEFI":
    "Needs 64 MB for the iGPU; often hidden — on some laptops it can only be changed with a BIOS mod",
  "Профиль памяти для производительности; на совместимость macOS не влияет":
    "Memory performance profile; has no effect on macOS compatibility",

  // --- BIOS: живая проверка ---
  "живая проверка доступна только на Windows и Linux":
    "live check is only available on Windows and Linux",
  "Secure Boot не удалось прочитать — нужны права администратора или система грузится в Legacy-режиме":
    "could not read Secure Boot — administrator rights required, or the system boots in Legacy mode",
  "efivars недоступны — Secure Boot прочитать не удалось":
    "efivars unavailable — could not read Secure Boot",

  // --- Защита EFI: аудит и бэкап ---
  "в выбранной папке нет каталога EFI — возможно, выбран не тот диск":
    "no EFI directory in the selected folder — the wrong disk may have been picked",
  "выбранная папка не похожа на EFI-раздел":
    "the selected folder does not look like an EFI system partition",
  "OpenCore не найден в EFI/OC — кексты положить можно, но загрузчик придётся установить отдельно (OpenCorePkg)":
    "OpenCore not found in EFI/OC — kexts can be placed, but the bootloader must be installed separately (OpenCorePkg)",
  "в EFI/Boot лежит BOOTX64.EFI чужого загрузчика — OpenEFI никогда не перезаписывает этот файл":
    "EFI/Boot contains a foreign BOOTX64.EFI — OpenEFI never overwrites this file",
  "OpenCore не найден в EFI/OC — бэкапить нечего":
    "OpenCore not found in EFI/OC — nothing to back up",

  // --- Сборка полной EFI: этапы и предупреждения ---
  "Проверка EFI-папки и резервное копирование…":
    "Checking the EFI folder and backing up…",
  "Определяю последнюю версию OpenCore…": "Resolving the latest OpenCore version…",
  "Скачиваю OpenCorePkg…": "Downloading OpenCorePkg…",
  "Распаковываю загрузчик OpenCore…": "Extracting the OpenCore bootloader…",
  "Скачиваю OcBinaryData (HfsPlus, Resources)…":
    "Downloading OcBinaryData (HfsPlus, Resources)…",
  "Распаковываю HfsPlus и Resources…": "Extracting HfsPlus and Resources…",
  "Настраиваю config.plist (драйверы, пикер)…":
    "Configuring config.plist (drivers, picker)…",
  "Готово": "Done",
  "EFI/BOOT/BOOTX64.EFI принадлежит другому загрузчику — OpenEFI его не тронул. Запускайте OpenCore через EFI/OC/OpenCore.efi (выберите его в BIOS/NVRAM или назначьте BootNext)":
    "EFI/BOOT/BOOTX64.EFI belongs to another bootloader — OpenEFI left it untouched. Launch OpenCore via EFI/OC/OpenCore.efi (pick it in the BIOS/NVRAM or set BootNext)",
  "в EFI/OC не было config.plist — положен Sample.plist из OpenCorePkg: он ничего не загружает, обязательно сгенерируйте свой конфиг (шаг «Конфиг»)":
    "there was no config.plist in EFI/OC — Sample.plist from OpenCorePkg was placed: it loads nothing, make sure to generate your own config (the “Config” step)",
  "config.plist не найден и в архиве OpenCorePkg нет Sample.plist — добавьте конфиг вручную":
    "config.plist not found and the OpenCorePkg archive has no Sample.plist — add a config manually",
  "в архиве OpenCorePkg нет X64/EFI/BOOT/BOOTx64.efi":
    "no X64/EFI/BOOT/BOOTx64.efi in the OpenCorePkg archive",
  "в архиве OpenCorePkg нет X64/EFI/OC/OpenCore.efi":
    "no X64/EFI/OC/OpenCore.efi in the OpenCorePkg archive",
  "в архиве OpenCorePkg нет OpenRuntime.efi": "no OpenRuntime.efi in the OpenCorePkg archive",
  "в архиве OcBinaryData нет Drivers/HfsPlus.efi — структура репозитория изменилась?":
    "no Drivers/HfsPlus.efi in the OcBinaryData archive — did the repository structure change?",
  "в config.plist нет секции UEFI": "no UEFI section in config.plist",
  "UEFI → Drivers в config.plist — не массив": "UEFI → Drivers in config.plist is not an array",
  "не создать UEFI → Drivers": "cannot create UEFI → Drivers",
  "Resources подозрительно большие (zip-bomb?)": "Resources look suspiciously large (zip-bomb?)",
  "GitHub не вернул Location для releases/latest":
    "GitHub did not return a Location header for releases/latest",
  "нет связи с GitHub (OpenCorePkg)": "no connection to GitHub (OpenCorePkg)",
};

// Динамические строки бэкенда: подстановки вида "AMD {codename}: серверная SMBIOS"
export const BACKEND_PATTERNS: [RegExp, string][] = [
  [/^AMD (.+): серверная SMBIOS$/, "AMD $1: server SMBIOS"],
  [/^AMD (.+): ноутбучная SMBIOS$/, "AMD $1: laptop SMBIOS"],
  [/^(.+) ноутбук: Apple Silicon-подобная SMBIOS$/, "$1 laptop: Apple Silicon-like SMBIOS"],
  [/^(.+): SMBIOS с поддержкой гибридной графики$/, "$1: SMBIOS with hybrid graphics support"],
  [/^(.+): новейшие iGPU требуют свежих SMBIOS$/, "$1: the newest iGPUs need newer SMBIOS"],
  [/^HDA кодек (.+)$/, "HDA codec $1"],
  [/^Кодек (.+) поддерживается AppleALC$/, "Codec $1 is supported by AppleALC"],
  [/^Кодек (.+): поддержки в AppleALC нет, звук может не завестись$/,
    "Codec $1: not supported by AppleALC, audio may not work"],
  [/^скачивание прервано: HTTP (\d+)$/, "download interrupted: HTTP $1"],
  [/^не создать (.+) — папка может быть защищена от записи: (.+)$/,
    "cannot create $1 — the folder may be write-protected: $2"],
  [/^не открыть (.+): (.+)$/, "cannot open $1: $2"],
  [/^не разобрать (.+): (.+)$/, "cannot parse $1: $2"],
  [/^не сделать копию (.+): (.+)$/, "cannot back up $1: $2"],
  [/^не записать (.+): (.+)$/, "cannot write $1: $2"],
  [/^ошибка записи (.+): (.+)$/, "write error on $1: $2"],
  [/^ошибка чтения(?: архива)?: (.+)$/, "read error: $1"],
  [/^не создать папку кекстов: (.+)$/, "cannot create kexts folder: $1"],
  [/^не создать папку (.+): (.+)$/, "cannot create folder $1: $2"],
  [/^не создать папку: (.+)$/, "cannot create folder: $1"],
  [/^не создать (.+): (.+)$/, "cannot create $1: $2"],
  [/^не сериализовать (.+): (.+)$/, "cannot serialize $1: $2"],
  [/^(.+) — не словарь plist$/, "$1 is not a plist dictionary"],
  [/^бэкап EFI\/OC не сделан: (.+)$/, "EFI/OC backup failed: $1"],
  [/^битый ZIP-архив: (.+)$/, "corrupt ZIP archive: $1"],
  [/^битый ZIP: (.+)$/, "corrupt ZIP: $1"],
  [/^ошибка чтения из архива: (.+)$/, "read error from the archive: $1"],
  [/^не определить версию OpenCore из redirect: (.+)$/, "cannot determine the OpenCore version from the redirect: $1"],
  [/^в архиве OpenCorePkg нет (.+)$/, "no $1 in the OpenCorePkg archive"],
  [/^config\.plist есть, но драйверы не зарегистрированы — включите их сами в UEFI → Drivers: (.+)$/,
    "config.plist found, but drivers were not registered — enable them yourself in UEFI → Drivers: $1"],
  [/^не создать скачанный архив: (.+)$/, "cannot open the downloaded archive: $1"],
];
