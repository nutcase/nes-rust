use crate::apu::ApuState;
use crate::cartridge::{CartridgeState, Mirroring, Mmc1State};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct SaveState {
    // CPU state
    pub cpu_a: u8,
    pub cpu_x: u8,
    pub cpu_y: u8,
    pub cpu_pc: u16,
    pub cpu_sp: u8,
    pub cpu_status: u8,
    pub cpu_cycles: u64,

    // PPU state
    pub ppu_control: u8,
    pub ppu_mask: u8,
    pub ppu_status: u8,
    pub ppu_oam_addr: u8,
    pub ppu_scroll_x: u8,
    pub ppu_scroll_y: u8,
    pub ppu_addr: u16,
    pub ppu_data_buffer: u8,
    pub ppu_w: bool,
    pub ppu_t: u16,
    pub ppu_v: u16,
    pub ppu_x: u8,
    pub ppu_scanline: i16,
    pub ppu_cycle: u16,
    pub ppu_frame: u64,

    // PPU memory
    pub ppu_palette: [u8; 32],
    pub ppu_nametable: Vec<u8>, // Flattened nametable data
    pub ppu_oam: Vec<u8>,

    // Main RAM
    pub ram: Vec<u8>,

    // Cartridge state
    pub cartridge_prg_bank: u8,
    pub cartridge_chr_bank: u8,
    #[serde(default)]
    pub cartridge_state: Option<CartridgeState>,

    // APU state (basic)
    pub apu_frame_counter: u8,
    pub apu_frame_interrupt: bool,
    #[serde(default)]
    pub apu_state: Option<ApuState>,

    // Additional metadata
    pub rom_filename: String,
    pub timestamp: u64,
    #[serde(default)]
    pub cpu_halted: bool,
    #[serde(default)]
    pub bus_dma_cycles: u32,
    #[serde(default)]
    pub bus_dma_in_progress: bool,
    #[serde(default)]
    pub bus_dmc_stall_cycles: u32,
    #[serde(default)]
    pub ppu_frame_complete: bool,
}

#[derive(Serialize, Deserialize)]
struct LegacySaveState {
    cpu_a: u8,
    cpu_x: u8,
    cpu_y: u8,
    cpu_pc: u16,
    cpu_sp: u8,
    cpu_status: u8,
    cpu_cycles: u64,
    ppu_control: u8,
    ppu_mask: u8,
    ppu_status: u8,
    ppu_oam_addr: u8,
    ppu_scroll_x: u8,
    ppu_scroll_y: u8,
    ppu_addr: u16,
    ppu_data_buffer: u8,
    ppu_w: bool,
    ppu_t: u16,
    ppu_v: u16,
    ppu_x: u8,
    ppu_scanline: i16,
    ppu_cycle: u16,
    ppu_frame: u64,
    ppu_palette: [u8; 32],
    ppu_nametable: Vec<u8>,
    ppu_oam: Vec<u8>,
    ram: Vec<u8>,
    cartridge_prg_bank: u8,
    cartridge_chr_bank: u8,
    apu_frame_counter: u8,
    apu_frame_interrupt: bool,
    rom_filename: String,
    timestamp: u64,
}

impl From<LegacySaveState> for SaveState {
    fn from(legacy: LegacySaveState) -> Self {
        SaveState {
            cpu_a: legacy.cpu_a,
            cpu_x: legacy.cpu_x,
            cpu_y: legacy.cpu_y,
            cpu_pc: legacy.cpu_pc,
            cpu_sp: legacy.cpu_sp,
            cpu_status: legacy.cpu_status,
            cpu_cycles: legacy.cpu_cycles,
            cpu_halted: false,
            ppu_control: legacy.ppu_control,
            ppu_mask: legacy.ppu_mask,
            ppu_status: legacy.ppu_status,
            ppu_oam_addr: legacy.ppu_oam_addr,
            ppu_scroll_x: legacy.ppu_scroll_x,
            ppu_scroll_y: legacy.ppu_scroll_y,
            ppu_addr: legacy.ppu_addr,
            ppu_data_buffer: legacy.ppu_data_buffer,
            ppu_w: legacy.ppu_w,
            ppu_t: legacy.ppu_t,
            ppu_v: legacy.ppu_v,
            ppu_x: legacy.ppu_x,
            ppu_scanline: legacy.ppu_scanline,
            ppu_cycle: legacy.ppu_cycle,
            ppu_frame: legacy.ppu_frame,
            ppu_palette: legacy.ppu_palette,
            ppu_nametable: legacy.ppu_nametable,
            ppu_oam: legacy.ppu_oam,
            ram: legacy.ram,
            cartridge_prg_bank: legacy.cartridge_prg_bank,
            cartridge_chr_bank: legacy.cartridge_chr_bank,
            cartridge_state: None,
            apu_frame_counter: legacy.apu_frame_counter,
            apu_frame_interrupt: legacy.apu_frame_interrupt,
            apu_state: None,
            rom_filename: legacy.rom_filename,
            timestamp: legacy.timestamp,
            bus_dma_cycles: 0,
            bus_dma_in_progress: false,
            bus_dmc_stall_cycles: 0,
            ppu_frame_complete: false,
        }
    }
}

/// CartridgeState before mmc2 was added (no mmc2 field).
#[derive(Serialize, Deserialize)]
struct CartridgeStateV1 {
    mapper: u8,
    mirroring: Mirroring,
    prg_bank: u8,
    chr_bank: u8,
    prg_ram: Vec<u8>,
    chr_ram: Vec<u8>,
    has_valid_save_data: bool,
    mmc1: Option<Mmc1State>,
}

/// SaveState before mmc2 was added to CartridgeState.
#[derive(Serialize, Deserialize)]
struct SaveStateV1 {
    cpu_a: u8,
    cpu_x: u8,
    cpu_y: u8,
    cpu_pc: u16,
    cpu_sp: u8,
    cpu_status: u8,
    cpu_cycles: u64,
    ppu_control: u8,
    ppu_mask: u8,
    ppu_status: u8,
    ppu_oam_addr: u8,
    ppu_scroll_x: u8,
    ppu_scroll_y: u8,
    ppu_addr: u16,
    ppu_data_buffer: u8,
    ppu_w: bool,
    ppu_t: u16,
    ppu_v: u16,
    ppu_x: u8,
    ppu_scanline: i16,
    ppu_cycle: u16,
    ppu_frame: u64,
    ppu_palette: [u8; 32],
    ppu_nametable: Vec<u8>,
    ppu_oam: Vec<u8>,
    ram: Vec<u8>,
    cartridge_prg_bank: u8,
    cartridge_chr_bank: u8,
    cartridge_state: Option<CartridgeStateV1>,
    apu_frame_counter: u8,
    apu_frame_interrupt: bool,
    rom_filename: String,
    timestamp: u64,
}

impl From<SaveStateV1> for SaveState {
    fn from(v1: SaveStateV1) -> Self {
        SaveState {
            cpu_a: v1.cpu_a,
            cpu_x: v1.cpu_x,
            cpu_y: v1.cpu_y,
            cpu_pc: v1.cpu_pc,
            cpu_sp: v1.cpu_sp,
            cpu_status: v1.cpu_status,
            cpu_cycles: v1.cpu_cycles,
            cpu_halted: false,
            ppu_control: v1.ppu_control,
            ppu_mask: v1.ppu_mask,
            ppu_status: v1.ppu_status,
            ppu_oam_addr: v1.ppu_oam_addr,
            ppu_scroll_x: v1.ppu_scroll_x,
            ppu_scroll_y: v1.ppu_scroll_y,
            ppu_addr: v1.ppu_addr,
            ppu_data_buffer: v1.ppu_data_buffer,
            ppu_w: v1.ppu_w,
            ppu_t: v1.ppu_t,
            ppu_v: v1.ppu_v,
            ppu_x: v1.ppu_x,
            ppu_scanline: v1.ppu_scanline,
            ppu_cycle: v1.ppu_cycle,
            ppu_frame: v1.ppu_frame,
            ppu_palette: v1.ppu_palette,
            ppu_nametable: v1.ppu_nametable,
            ppu_oam: v1.ppu_oam,
            ram: v1.ram,
            cartridge_prg_bank: v1.cartridge_prg_bank,
            cartridge_chr_bank: v1.cartridge_chr_bank,
            cartridge_state: v1.cartridge_state.map(|cs| CartridgeState {
                mapper: cs.mapper,
                mirroring: cs.mirroring,
                prg_bank: cs.prg_bank,
                chr_bank: cs.chr_bank,
                prg_ram: cs.prg_ram,
                chr_ram: cs.chr_ram,
                has_valid_save_data: cs.has_valid_save_data,
                mmc1: cs.mmc1,
                mmc2: None,
                mmc3: None,
                mmc5: None,
                namco163: None,
                fme7: None,
                bandai_fcg: None,
                mapper34: None,
                mapper93: None,
                mapper184: None,
                vrc1: None,
                vrc2_vrc4: None,
                mapper15: None,
                mapper72: None,
                mapper58: None,
                mapper59: None,
                mapper60: None,
                mapper225: None,
                mapper232: None,
                mapper234: None,
                mapper235: None,
                mapper202: None,
                mapper212: None,
                mapper226: None,
                mapper230: None,
                mapper228: None,
                mapper242: None,
                mapper243: None,
                mapper221: None,
                mapper191: None,
                mapper195: None,
                mapper208: None,
                mapper189: None,
                mapper236: None,
                mapper227: None,
                mapper246: None,
                sunsoft4: None,
                taito_tc0190: None,
                taito_x1005: None,
                taito_x1017: None,
                mapper233: None,
                mapper41: None,
                mapper40: None,
                mapper42: None,
                mapper50: None,
                irem_g101: None,
                vrc3: None,
                mapper43: None,
                irem_h3001: None,
                mapper103: None,
                mapper37: None,
                mapper44: None,
                mapper47: None,
                mapper12: None,
                mapper114: None,
                mapper123: None,
                mapper115: None,
                mapper205: None,
                mapper61: None,
                mapper185: None,
                sunsoft3: None,
                mapper63: None,
                mapper137: None,
                mapper142: None,
                mapper150: None,
                mapper18: None,
                mapper210: None,
                vrc6: None,
            }),
            apu_frame_counter: v1.apu_frame_counter,
            apu_frame_interrupt: v1.apu_frame_interrupt,
            apu_state: None,
            rom_filename: v1.rom_filename,
            timestamp: v1.timestamp,
            bus_dma_cycles: 0,
            bus_dma_in_progress: false,
            bus_dmc_stall_cycles: 0,
            ppu_frame_complete: false,
        }
    }
}

#[derive(Serialize, Deserialize)]
struct SaveStateV2 {
    cpu_a: u8,
    cpu_x: u8,
    cpu_y: u8,
    cpu_pc: u16,
    cpu_sp: u8,
    cpu_status: u8,
    cpu_cycles: u64,
    ppu_control: u8,
    ppu_mask: u8,
    ppu_status: u8,
    ppu_oam_addr: u8,
    ppu_scroll_x: u8,
    ppu_scroll_y: u8,
    ppu_addr: u16,
    ppu_data_buffer: u8,
    ppu_w: bool,
    ppu_t: u16,
    ppu_v: u16,
    ppu_x: u8,
    ppu_scanline: i16,
    ppu_cycle: u16,
    ppu_frame: u64,
    ppu_palette: [u8; 32],
    ppu_nametable: Vec<u8>,
    ppu_oam: Vec<u8>,
    ram: Vec<u8>,
    cartridge_prg_bank: u8,
    cartridge_chr_bank: u8,
    cartridge_state: Option<CartridgeState>,
    apu_frame_counter: u8,
    apu_frame_interrupt: bool,
    rom_filename: String,
    timestamp: u64,
}

impl From<SaveStateV2> for SaveState {
    fn from(v2: SaveStateV2) -> Self {
        SaveState {
            cpu_a: v2.cpu_a,
            cpu_x: v2.cpu_x,
            cpu_y: v2.cpu_y,
            cpu_pc: v2.cpu_pc,
            cpu_sp: v2.cpu_sp,
            cpu_status: v2.cpu_status,
            cpu_cycles: v2.cpu_cycles,
            cpu_halted: false,
            ppu_control: v2.ppu_control,
            ppu_mask: v2.ppu_mask,
            ppu_status: v2.ppu_status,
            ppu_oam_addr: v2.ppu_oam_addr,
            ppu_scroll_x: v2.ppu_scroll_x,
            ppu_scroll_y: v2.ppu_scroll_y,
            ppu_addr: v2.ppu_addr,
            ppu_data_buffer: v2.ppu_data_buffer,
            ppu_w: v2.ppu_w,
            ppu_t: v2.ppu_t,
            ppu_v: v2.ppu_v,
            ppu_x: v2.ppu_x,
            ppu_scanline: v2.ppu_scanline,
            ppu_cycle: v2.ppu_cycle,
            ppu_frame: v2.ppu_frame,
            ppu_palette: v2.ppu_palette,
            ppu_nametable: v2.ppu_nametable,
            ppu_oam: v2.ppu_oam,
            ram: v2.ram,
            cartridge_prg_bank: v2.cartridge_prg_bank,
            cartridge_chr_bank: v2.cartridge_chr_bank,
            cartridge_state: v2.cartridge_state,
            apu_frame_counter: v2.apu_frame_counter,
            apu_frame_interrupt: v2.apu_frame_interrupt,
            apu_state: None,
            rom_filename: v2.rom_filename,
            timestamp: v2.timestamp,
            bus_dma_cycles: 0,
            bus_dma_in_progress: false,
            bus_dmc_stall_cycles: 0,
            ppu_frame_complete: false,
        }
    }
}

impl SaveState {
    pub fn save_to_file(&self, filename: &str) -> Result<(), Box<dyn std::error::Error>> {
        let data = bincode::serialize(self)?;
        std::fs::write(filename, data)?;
        println!("Save state written to: {}", filename);
        Ok(())
    }

    pub fn load_from_file(filename: &str) -> Result<SaveState, Box<dyn std::error::Error>> {
        let data = std::fs::read(filename)?;
        if let Ok(save_state) = bincode::deserialize::<SaveState>(&data) {
            println!("Save state loaded from: {}", filename);
            return Ok(save_state);
        }

        if let Ok(v2) = bincode::deserialize::<SaveStateV2>(&data) {
            println!("Save state loaded from: {} (v2 format)", filename);
            return Ok(v2.into());
        }

        if let Ok(v1) = bincode::deserialize::<SaveStateV1>(&data) {
            println!("Save state loaded from: {} (v1 format)", filename);
            return Ok(v1.into());
        }

        let legacy = bincode::deserialize::<LegacySaveState>(&data)?;
        println!("Save state loaded from: {} (legacy format)", filename);
        Ok(legacy.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_legacy_save_state_defaults_new_fields() {
        let legacy = LegacySaveState {
            cpu_a: 1,
            cpu_x: 2,
            cpu_y: 3,
            cpu_pc: 0x8000,
            cpu_sp: 0xFD,
            cpu_status: 0x24,
            cpu_cycles: 123,
            ppu_control: 0,
            ppu_mask: 0,
            ppu_status: 0,
            ppu_oam_addr: 0,
            ppu_scroll_x: 0,
            ppu_scroll_y: 0,
            ppu_addr: 0,
            ppu_data_buffer: 0,
            ppu_w: false,
            ppu_t: 0,
            ppu_v: 0,
            ppu_x: 0,
            ppu_scanline: 0,
            ppu_cycle: 0,
            ppu_frame: 0,
            ppu_palette: [0; 32],
            ppu_nametable: vec![0; 2048],
            ppu_oam: vec![0; 256],
            ram: vec![0; 0x800],
            cartridge_prg_bank: 0,
            cartridge_chr_bank: 0,
            apu_frame_counter: 0,
            apu_frame_interrupt: false,
            rom_filename: "legacy".to_string(),
            timestamp: 0,
        };

        let encoded = bincode::serialize(&legacy).expect("serialize legacy save");
        let mut path = std::env::temp_dir();
        path.push(format!(
            "nes_legacy_state_{}.sav",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system time")
                .as_nanos()
        ));
        std::fs::write(&path, encoded).expect("write legacy save");
        let decoded = SaveState::load_from_file(path.to_str().expect("utf-8 path"))
            .expect("load legacy save");
        let _ = std::fs::remove_file(path);

        assert!(decoded.cartridge_state.is_none());
        assert!(decoded.apu_state.is_none());
        assert!(!decoded.cpu_halted);
        assert_eq!(decoded.bus_dma_cycles, 0);
        assert!(!decoded.bus_dma_in_progress);
        assert_eq!(decoded.bus_dmc_stall_cycles, 0);
        assert!(!decoded.ppu_frame_complete);
    }

    #[test]
    fn deserialize_v2_save_state_defaults_apu_state() {
        let v2 = SaveStateV2 {
            cpu_a: 0x01,
            cpu_x: 0x02,
            cpu_y: 0x03,
            cpu_pc: 0x8000,
            cpu_sp: 0xFD,
            cpu_status: 0x24,
            cpu_cycles: 1234,
            ppu_control: 0,
            ppu_mask: 0,
            ppu_status: 0,
            ppu_oam_addr: 0,
            ppu_scroll_x: 0,
            ppu_scroll_y: 0,
            ppu_addr: 0,
            ppu_data_buffer: 0,
            ppu_w: false,
            ppu_t: 0,
            ppu_v: 0,
            ppu_x: 0,
            ppu_scanline: 0,
            ppu_cycle: 0,
            ppu_frame: 0,
            ppu_palette: [0; 32],
            ppu_nametable: vec![0; 2048],
            ppu_oam: vec![0; 256],
            ram: vec![0; 0x800],
            cartridge_prg_bank: 0,
            cartridge_chr_bank: 0,
            cartridge_state: None,
            apu_frame_counter: 7,
            apu_frame_interrupt: true,
            rom_filename: "v2".to_string(),
            timestamp: 77,
        };

        let encoded = bincode::serialize(&v2).expect("serialize v2 save");
        let mut path = std::env::temp_dir();
        path.push(format!(
            "nes_v2_state_{}.sav",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system time")
                .as_nanos()
        ));
        std::fs::write(&path, &encoded).expect("write v2 save");
        let decoded =
            SaveState::load_from_file(path.to_str().expect("utf-8 path")).expect("load v2 save");
        let _ = std::fs::remove_file(path);

        assert_eq!(decoded.apu_frame_counter, 7);
        assert!(decoded.apu_frame_interrupt);
        assert!(decoded.apu_state.is_none());
        assert!(!decoded.cpu_halted);
        assert_eq!(decoded.bus_dma_cycles, 0);
        assert!(!decoded.bus_dma_in_progress);
        assert_eq!(decoded.bus_dmc_stall_cycles, 0);
        assert!(!decoded.ppu_frame_complete);
    }

    #[test]
    fn save_state_round_trips_cpu_and_bus_timing_fields() {
        let state = SaveState {
            cpu_a: 0,
            cpu_x: 0,
            cpu_y: 0,
            cpu_pc: 0x8000,
            cpu_sp: 0xFD,
            cpu_status: 0x24,
            cpu_cycles: 42_123,
            ppu_control: 0,
            ppu_mask: 0,
            ppu_status: 0,
            ppu_oam_addr: 0,
            ppu_scroll_x: 0,
            ppu_scroll_y: 0,
            ppu_addr: 0,
            ppu_data_buffer: 0,
            ppu_w: false,
            ppu_t: 0,
            ppu_v: 0,
            ppu_x: 0,
            ppu_scanline: 10,
            ppu_cycle: 123,
            ppu_frame: 456,
            ppu_palette: [0; 32],
            ppu_nametable: vec![0; 2048],
            ppu_oam: vec![0; 256],
            ram: vec![0; 0x800],
            cartridge_prg_bank: 0,
            cartridge_chr_bank: 0,
            cartridge_state: None,
            apu_frame_counter: 0,
            apu_frame_interrupt: false,
            apu_state: None,
            rom_filename: "roundtrip".to_string(),
            timestamp: 999,
            cpu_halted: true,
            bus_dma_cycles: 7,
            bus_dma_in_progress: true,
            bus_dmc_stall_cycles: 3,
            ppu_frame_complete: true,
        };

        let encoded = bincode::serialize(&state).expect("serialize save state");
        let decoded: SaveState = bincode::deserialize(&encoded).expect("deserialize save state");

        assert_eq!(decoded.cpu_cycles, 42_123);
        assert!(decoded.cpu_halted);
        assert_eq!(decoded.bus_dma_cycles, 7);
        assert!(decoded.bus_dma_in_progress);
        assert_eq!(decoded.bus_dmc_stall_cycles, 3);
        assert!(decoded.ppu_frame_complete);
    }

    #[test]
    fn deserialize_v1_save_state_without_mmc2() {
        use crate::cartridge::{Mirroring, Mmc1State};

        // Build a V1-era SaveState (CartridgeState without mmc2)
        let v1 = SaveStateV1 {
            cpu_a: 0x10,
            cpu_x: 0x20,
            cpu_y: 0x30,
            cpu_pc: 0xC000,
            cpu_sp: 0xFB,
            cpu_status: 0x24,
            cpu_cycles: 5000,
            ppu_control: 0x80,
            ppu_mask: 0x1E,
            ppu_status: 0,
            ppu_oam_addr: 0,
            ppu_scroll_x: 0,
            ppu_scroll_y: 0,
            ppu_addr: 0,
            ppu_data_buffer: 0,
            ppu_w: false,
            ppu_t: 0,
            ppu_v: 0,
            ppu_x: 0,
            ppu_scanline: 0,
            ppu_cycle: 0,
            ppu_frame: 100,
            ppu_palette: [0; 32],
            ppu_nametable: vec![0; 2048],
            ppu_oam: vec![0; 256],
            ram: vec![0; 0x800],
            cartridge_prg_bank: 3,
            cartridge_chr_bank: 5,
            cartridge_state: Some(CartridgeStateV1 {
                mapper: 1,
                mirroring: Mirroring::Vertical,
                prg_bank: 3,
                chr_bank: 5,
                prg_ram: vec![0xAA; 0x2000],
                chr_ram: vec![0; 0x2000],
                has_valid_save_data: true,
                mmc1: Some(Mmc1State {
                    shift_register: 0x10,
                    shift_count: 0,
                    control: 0x0C,
                    chr_bank_0: 2,
                    chr_bank_1: 4,
                    prg_bank: 3,
                    prg_ram_disable: false,
                }),
            }),
            apu_frame_counter: 2,
            apu_frame_interrupt: false,
            rom_filename: "v1_test".to_string(),
            timestamp: 12345,
        };

        let encoded = bincode::serialize(&v1).expect("serialize v1 save");
        let mut path = std::env::temp_dir();
        path.push(format!(
            "nes_v1_state_{}.sav",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system time")
                .as_nanos()
        ));
        std::fs::write(&path, &encoded).expect("write v1 save");
        let decoded =
            SaveState::load_from_file(path.to_str().expect("utf-8 path")).expect("load v1 save");
        let _ = std::fs::remove_file(path);

        assert_eq!(decoded.cpu_a, 0x10);
        assert_eq!(decoded.apu_frame_counter, 2);
        assert_eq!(decoded.rom_filename, "v1_test");
        assert!(decoded.apu_state.is_none());
        assert!(!decoded.cpu_halted);
        assert_eq!(decoded.bus_dma_cycles, 0);
        assert!(!decoded.bus_dma_in_progress);
        assert_eq!(decoded.bus_dmc_stall_cycles, 0);
        assert!(!decoded.ppu_frame_complete);

        let cs = decoded
            .cartridge_state
            .expect("cartridge_state should exist");
        assert_eq!(cs.mapper, 1);
        assert_eq!(cs.prg_bank, 3);
        assert!(cs.mmc1.is_some());
        assert!(cs.mmc2.is_none());
    }
}
