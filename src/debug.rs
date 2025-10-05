#[cfg(debug_assertions)]
use std::fs::File;
#[cfg(debug_assertions)]
use std::io::Write;
#[cfg(debug_assertions)]
use std::path::Path;

#[cfg(debug_assertions)]
pub fn dump_vram<P: AsRef<Path>>(bus: &crate::bus::Bus, path: P) -> std::io::Result<()> {
    let snapshot = bus.ppu_vram_snapshot();
    let mut file = File::create(path)?;
    file.write_all(&snapshot)?;
    Ok(())
}

#[cfg(not(debug_assertions))]
pub fn dump_vram<P: AsRef<std::path::Path>>(
    _bus: &crate::bus::Bus,
    _path: P,
) -> std::io::Result<()> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Other,
        "VRAM dump helper is only available in debug builds",
    ))
}
