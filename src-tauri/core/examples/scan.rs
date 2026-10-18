fn main() {
    let report = openefi_core::hardware::scan();
    println!("CPU: {} | codename: {:?} | cores: {} | AVX2: {}",
        report.cpu.name, report.cpu.codename, report.cpu.cores, report.cpu.avx2);
    println!("Platform: {} | Board: {} | RAM: {} GB", report.platform, report.board_name, report.memory_gb);
    for gpu in &report.gpus { println!("GPU: [{}] {} ({}:{})", gpu.device_type, gpu.name, gpu.vendor_id, gpu.device_id); }
    for n in &report.network { println!("NET: [{}] {} ({}:{})", n.kind, n.name, n.vendor_id, n.device_id); }
    for a in &report.audio { println!("AUDIO: {} {:?} ", a.name, a.note); }
    for b in &report.bluetooth { println!("BT: {}", b.name); }
    println!("Warnings: {:?}", report.warnings);
    let darwin = 24;
    println!("\n--- kexts for darwin {darwin} ---");
    for k in openefi_core::kexts::match_kexts(&report, darwin).into_iter().filter(|k| k.selected) {
        println!("  [{}] {}", k.status, k.name);
    }
    let rec = openefi_core::smbios::recommend(&report, darwin);
    println!("\nSMBIOS: {} ({})", rec.recommended, rec.reason);
    let sm = openefi_core::smbios::generate(&rec.recommended);
    println!("Serial: {} | MLB: {} | ROM: {} | UUID: {}", sm.serial, sm.mlb, sm.rom, sm.uuid);
}
