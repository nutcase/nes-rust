use super::{Cartridge, Mirroring, Sunsoft4};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mmc1State {
    pub shift_register: u8,
    pub shift_count: u8,
    pub control: u8,
    pub chr_bank_0: u8,
    pub chr_bank_1: u8,
    pub prg_bank: u8,
    pub prg_ram_disable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mmc2State {
    pub prg_bank: u8,
    pub chr_bank_0_fd: u8,
    pub chr_bank_0_fe: u8,
    pub chr_bank_1_fd: u8,
    pub chr_bank_1_fe: u8,
    pub latch_0: bool,
    pub latch_1: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mmc3State {
    pub bank_select: u8,
    pub bank_registers: [u8; 8],
    #[serde(default)]
    pub extra_bank_registers: [u8; 8],
    pub irq_latch: u8,
    pub irq_counter: u8,
    pub irq_reload: bool,
    pub irq_enabled: bool,
    pub irq_pending: bool,
    pub prg_ram_enabled: bool,
    pub prg_ram_write_protect: bool,
    #[serde(default)]
    pub irq_cycle_mode: bool,
    #[serde(default = "default_mmc3_irq_prescaler")]
    pub irq_prescaler: u8,
    #[serde(default)]
    pub irq_delay: u8,
}

fn default_mmc3_irq_prescaler() -> u8 {
    4
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Mmc5PulseState {
    pub duty: u8,
    pub length_counter: u8,
    pub envelope_divider: u8,
    pub envelope_decay: u8,
    pub envelope_disable: bool,
    pub envelope_start: bool,
    pub volume: u8,
    pub timer: u16,
    pub timer_reload: u16,
    pub duty_counter: u8,
    pub length_enabled: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Mmc5AudioState {
    pub pulse1: Mmc5PulseState,
    pub pulse2: Mmc5PulseState,
    pub pulse1_enabled: bool,
    pub pulse2_enabled: bool,
    pub pcm_irq_enabled: bool,
    pub pcm_read_mode: bool,
    pub pcm_irq_pending: bool,
    pub pcm_dac: u8,
    pub audio_frame_accum: u32,
    pub audio_even_cycle: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mmc5State {
    pub prg_mode: u8,
    pub chr_mode: u8,
    pub exram_mode: u8,
    pub prg_ram_protect_1: u8,
    pub prg_ram_protect_2: u8,
    pub nametable_map: [u8; 4],
    pub fill_tile: u8,
    pub fill_attr: u8,
    pub prg_ram_bank: u8,
    pub prg_banks: [u8; 4],
    pub chr_upper: u8,
    pub sprite_chr_banks: [u8; 8],
    pub bg_chr_banks: [u8; 4],
    pub exram: Vec<u8>,
    pub irq_scanline_compare: u8,
    pub irq_enabled: bool,
    pub irq_pending: bool,
    pub in_frame: bool,
    pub scanline_counter: u8,
    pub multiplier_a: u8,
    pub multiplier_b: u8,
    pub split_control: u8,
    pub split_scroll: u8,
    pub split_bank: u8,
    pub ppu_ctrl: u8,
    pub ppu_mask: u8,
    pub cached_tile_x: u8,
    pub cached_tile_y: u8,
    pub cached_ext_palette: u8,
    pub cached_ext_bank: u8,
    #[serde(default)]
    pub ppu_data_uses_bg_banks: bool,
    #[serde(default)]
    pub audio: Mmc5AudioState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Namco163State {
    pub chr_banks: [u8; 12],
    pub prg_banks: [u8; 3],
    pub sound_disable: bool,
    pub chr_nt_disabled_low: bool,
    pub chr_nt_disabled_high: bool,
    pub wram_write_enable: bool,
    pub wram_write_protect: u8,
    pub internal_addr: u8,
    pub internal_auto_increment: bool,
    pub irq_counter: u16,
    pub irq_enabled: bool,
    pub irq_pending: bool,
    pub audio_delay: u8,
    pub audio_channel_index: u8,
    pub audio_outputs: [f32; 8],
    pub audio_current: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mapper18State {
    pub prg_banks: [u8; 3],
    pub chr_banks: [u8; 8],
    pub prg_ram_enabled: bool,
    pub prg_ram_write_enabled: bool,
    pub irq_reload: u16,
    pub irq_counter: u16,
    pub irq_control: u8,
    pub irq_pending: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mapper210State {
    pub chr_banks: [u8; 8],
    pub prg_banks: [u8; 3],
    pub namco340: bool,
    pub prg_ram_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fme7State {
    pub command: u8,
    pub chr_banks: [u8; 8],
    pub prg_banks: [u8; 3],
    pub prg_bank_6000: u8,
    pub prg_ram_enabled: bool,
    pub prg_ram_select: bool,
    pub irq_counter: u16,
    pub irq_counter_enabled: bool,
    pub irq_enabled: bool,
    pub irq_pending: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BandaiFcgState {
    pub chr_banks: [u8; 8],
    pub prg_bank: u8,
    pub irq_counter: u16,
    pub irq_latch: u16,
    pub irq_enabled: bool,
    pub irq_pending: bool,
    #[serde(default)]
    pub outer_prg_bank: u8,
    #[serde(default)]
    pub prg_ram_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mapper34State {
    pub nina001: bool,
    pub chr_bank_1: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mapper93State {
    pub chr_ram_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mapper184State {
    pub chr_bank_1: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vrc1State {
    pub prg_banks: [u8; 3],
    pub chr_bank_0: u8,
    pub chr_bank_1: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vrc2Vrc4State {
    pub prg_banks: [u8; 2],
    pub chr_banks: [u16; 8],
    pub wram_enabled: bool,
    pub prg_swap_mode: bool,
    pub vrc4_mode: bool,
    pub latch: u8,
    pub irq_latch: u8,
    pub irq_counter: u8,
    pub irq_enable_after_ack: bool,
    pub irq_enabled: bool,
    pub irq_cycle_mode: bool,
    pub irq_prescaler: i16,
    pub irq_pending: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mapper15State {
    pub mode: u8,
    pub data: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mapper72State {
    pub last_command: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mapper58State {
    pub nrom128: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mapper59State {
    pub latch: u16,
    pub locked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mapper60State {
    pub game_select: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mapper61State {
    pub latch: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mapper63State {
    pub latch: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mapper137State {
    pub index: u8,
    pub registers: [u8; 8],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mapper142State {
    pub bank_select: u8,
    pub prg_banks: [u8; 4],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mapper150State {
    pub index: u8,
    pub registers: [u8; 8],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mapper225State {
    pub nrom128: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mapper232State {
    pub outer_bank: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mapper41State {
    pub inner_bank: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mapper40State {
    pub irq_counter: u16,
    pub irq_enabled: bool,
    pub irq_pending: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mapper42State {
    pub irq_counter: u16,
    pub irq_enabled: bool,
    pub irq_pending: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mapper43State {
    pub irq_counter: u16,
    pub irq_enabled: bool,
    pub irq_pending: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mapper50State {
    pub irq_counter: u16,
    pub irq_enabled: bool,
    pub irq_pending: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IremG101State {
    pub prg_banks: [u8; 2],
    pub chr_banks: [u8; 8],
    pub prg_mode: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IremH3001State {
    pub prg_banks: [u8; 2],
    pub chr_banks: [u8; 8],
    pub prg_mode: bool,
    pub irq_reload: u16,
    pub irq_counter: u16,
    pub irq_enabled: bool,
    pub irq_pending: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vrc3State {
    pub irq_reload: u16,
    pub irq_counter: u16,
    pub irq_enable_on_ack: bool,
    pub irq_enabled: bool,
    pub irq_mode_8bit: bool,
    pub irq_pending: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vrc6PulseState {
    pub volume: u8,
    pub duty: u8,
    pub ignore_duty: bool,
    pub period: u16,
    pub enabled: bool,
    pub step: u8,
    pub divider: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vrc6SawState {
    pub rate: u8,
    pub period: u16,
    pub enabled: bool,
    pub step: u8,
    pub divider: u16,
    pub accumulator: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vrc6State {
    pub prg_bank_16k: u8,
    pub prg_bank_8k: u8,
    pub chr_banks: [u8; 8],
    pub banking_control: u8,
    pub irq_latch: u8,
    pub irq_counter: u8,
    pub irq_enable_after_ack: bool,
    pub irq_enabled: bool,
    pub irq_cycle_mode: bool,
    pub irq_prescaler: i16,
    pub irq_pending: bool,
    pub audio_halt: bool,
    pub audio_freq_shift: u8,
    pub pulse1: Vrc6PulseState,
    pub pulse2: Vrc6PulseState,
    pub saw: Vrc6SawState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mapper233State {
    pub nrom128: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mapper234State {
    pub reg0: u8,
    pub reg1: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mapper235State {
    pub nrom128: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mapper202State {
    pub mode_32k: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mapper37State {
    pub outer_bank: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mapper44State {
    pub outer_bank: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mapper103State {
    pub prg_ram_disabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mapper12State {
    pub chr_outer: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mapper114State {
    pub nrom_override: u8,
    pub chr_outer_bank: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mapper115State {
    pub nrom_override: u8,
    pub chr_outer_bank: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mapper212State {
    pub mode_32k: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mapper47State {
    pub outer_bank: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mapper123State {
    pub nrom_override: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mapper205State {
    pub block: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mapper226State {
    pub nrom128: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mapper230State {
    pub contra_mode: bool,
    pub nrom128: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mapper228State {
    pub chip_select: u8,
    pub nrom128: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mapper242State {
    pub latch: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mapper243State {
    pub index: u8,
    pub registers: [u8; 8],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mapper221State {
    pub mode: u8,
    pub outer_bank: u8,
    pub chr_write_protect: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mapper191State {
    pub outer_bank: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mapper195State {
    pub mode: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mapper208State {
    pub protection_index: u8,
    pub protection_regs: [u8; 4],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mapper189State {
    pub prg_bank: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mapper185State {
    pub disabled_reads: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mapper236State {
    pub mode: u8,
    pub outer_bank: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mapper227State {
    pub latch: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mapper246State {
    pub prg_banks: [u8; 4],
    pub chr_banks: [u8; 4],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sunsoft3State {
    pub chr_banks: [u8; 4],
    pub prg_bank: u8,
    pub irq_counter: u16,
    pub irq_enabled: bool,
    pub irq_pending: bool,
    pub irq_write_high: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sunsoft4State {
    pub chr_banks: [u8; 4],
    pub nametable_banks: [u8; 2],
    pub control: u8,
    pub prg_bank: u8,
    pub prg_ram_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaitoTc0190State {
    pub prg_banks: [u8; 2],
    pub chr_banks: [u8; 6],
    #[serde(default)]
    pub irq_latch: u8,
    #[serde(default)]
    pub irq_counter: u8,
    #[serde(default)]
    pub irq_reload: bool,
    #[serde(default)]
    pub irq_enabled: bool,
    #[serde(default)]
    pub irq_pending: bool,
    #[serde(default)]
    pub irq_delay: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaitoX1005State {
    pub prg_banks: [u8; 3],
    pub chr_banks: [u8; 6],
    pub ram_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaitoX1017State {
    pub prg_banks: [u8; 3],
    pub chr_banks: [u8; 6],
    pub ram_enabled: [bool; 3],
    pub chr_invert: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CartridgeState {
    pub mapper: u8,
    pub mirroring: Mirroring,
    pub prg_bank: u8,
    pub chr_bank: u8,
    pub prg_ram: Vec<u8>,
    pub chr_ram: Vec<u8>,
    pub has_valid_save_data: bool,
    pub mmc1: Option<Mmc1State>,
    pub mmc2: Option<Mmc2State>,
    #[serde(default)]
    pub mmc3: Option<Mmc3State>,
    #[serde(default)]
    pub mmc5: Option<Mmc5State>,
    #[serde(default)]
    pub namco163: Option<Namco163State>,
    #[serde(default)]
    pub fme7: Option<Fme7State>,
    #[serde(default)]
    pub bandai_fcg: Option<BandaiFcgState>,
    #[serde(default)]
    pub mapper34: Option<Mapper34State>,
    #[serde(default)]
    pub mapper93: Option<Mapper93State>,
    #[serde(default)]
    pub mapper184: Option<Mapper184State>,
    #[serde(default)]
    pub vrc1: Option<Vrc1State>,
    #[serde(default)]
    pub vrc2_vrc4: Option<Vrc2Vrc4State>,
    #[serde(default)]
    pub mapper15: Option<Mapper15State>,
    #[serde(default)]
    pub mapper72: Option<Mapper72State>,
    #[serde(default)]
    pub mapper58: Option<Mapper58State>,
    #[serde(default)]
    pub mapper59: Option<Mapper59State>,
    #[serde(default)]
    pub mapper60: Option<Mapper60State>,
    #[serde(default)]
    pub mapper225: Option<Mapper225State>,
    #[serde(default)]
    pub mapper232: Option<Mapper232State>,
    #[serde(default)]
    pub mapper234: Option<Mapper234State>,
    #[serde(default)]
    pub mapper235: Option<Mapper235State>,
    #[serde(default)]
    pub mapper202: Option<Mapper202State>,
    #[serde(default)]
    pub mapper212: Option<Mapper212State>,
    #[serde(default)]
    pub mapper226: Option<Mapper226State>,
    #[serde(default)]
    pub mapper230: Option<Mapper230State>,
    #[serde(default)]
    pub mapper228: Option<Mapper228State>,
    #[serde(default)]
    pub mapper242: Option<Mapper242State>,
    #[serde(default)]
    pub mapper243: Option<Mapper243State>,
    #[serde(default)]
    pub mapper221: Option<Mapper221State>,
    #[serde(default)]
    pub mapper191: Option<Mapper191State>,
    #[serde(default)]
    pub mapper195: Option<Mapper195State>,
    #[serde(default)]
    pub mapper208: Option<Mapper208State>,
    #[serde(default)]
    pub mapper189: Option<Mapper189State>,
    #[serde(default)]
    pub mapper236: Option<Mapper236State>,
    #[serde(default)]
    pub mapper227: Option<Mapper227State>,
    #[serde(default)]
    pub mapper246: Option<Mapper246State>,
    #[serde(default)]
    pub sunsoft4: Option<Sunsoft4State>,
    #[serde(default)]
    pub taito_tc0190: Option<TaitoTc0190State>,
    #[serde(default)]
    pub taito_x1005: Option<TaitoX1005State>,
    #[serde(default)]
    pub taito_x1017: Option<TaitoX1017State>,
    #[serde(default)]
    pub mapper233: Option<Mapper233State>,
    #[serde(default)]
    pub mapper41: Option<Mapper41State>,
    #[serde(default)]
    pub mapper40: Option<Mapper40State>,
    #[serde(default)]
    pub mapper42: Option<Mapper42State>,
    #[serde(default)]
    pub mapper50: Option<Mapper50State>,
    #[serde(default)]
    pub irem_g101: Option<IremG101State>,
    #[serde(default)]
    pub vrc3: Option<Vrc3State>,
    #[serde(default)]
    pub mapper43: Option<Mapper43State>,
    #[serde(default)]
    pub irem_h3001: Option<IremH3001State>,
    #[serde(default)]
    pub mapper103: Option<Mapper103State>,
    #[serde(default)]
    pub mapper37: Option<Mapper37State>,
    #[serde(default)]
    pub mapper44: Option<Mapper44State>,
    #[serde(default)]
    pub mapper47: Option<Mapper47State>,
    #[serde(default)]
    pub mapper12: Option<Mapper12State>,
    #[serde(default)]
    pub mapper114: Option<Mapper114State>,
    #[serde(default)]
    pub mapper123: Option<Mapper123State>,
    #[serde(default)]
    pub mapper115: Option<Mapper115State>,
    #[serde(default)]
    pub mapper205: Option<Mapper205State>,
    #[serde(default)]
    pub mapper61: Option<Mapper61State>,
    #[serde(default)]
    pub mapper185: Option<Mapper185State>,
    #[serde(default)]
    pub sunsoft3: Option<Sunsoft3State>,
    #[serde(default)]
    pub mapper63: Option<Mapper63State>,
    #[serde(default)]
    pub mapper137: Option<Mapper137State>,
    #[serde(default)]
    pub mapper142: Option<Mapper142State>,
    #[serde(default)]
    pub mapper150: Option<Mapper150State>,
    #[serde(default)]
    pub mapper18: Option<Mapper18State>,
    #[serde(default)]
    pub mapper210: Option<Mapper210State>,
    #[serde(default)]
    pub vrc6: Option<Vrc6State>,
}

impl Cartridge {
    pub fn snapshot_state(&self) -> CartridgeState {
        let mmc1 = self.mmc1.as_ref().map(|m| Mmc1State {
            shift_register: m.shift_register,
            shift_count: m.shift_count,
            control: m.control,
            chr_bank_0: m.chr_bank_0,
            chr_bank_1: m.chr_bank_1,
            prg_bank: m.prg_bank,
            prg_ram_disable: m.prg_ram_disable,
        });

        let mmc2 = self.mmc2.as_ref().map(|m| Mmc2State {
            prg_bank: m.prg_bank,
            chr_bank_0_fd: m.chr_bank_0_fd,
            chr_bank_0_fe: m.chr_bank_0_fe,
            chr_bank_1_fd: m.chr_bank_1_fd,
            chr_bank_1_fe: m.chr_bank_1_fe,
            latch_0: m.latch_0.get(),
            latch_1: m.latch_1.get(),
        });

        let mmc3 = self.mmc3.as_ref().map(|m| Mmc3State {
            bank_select: m.bank_select,
            bank_registers: m.bank_registers,
            extra_bank_registers: m.extra_bank_registers,
            irq_latch: m.irq_latch,
            irq_counter: m.irq_counter,
            irq_reload: m.irq_reload,
            irq_enabled: m.irq_enabled,
            irq_pending: m.irq_pending.get(),
            prg_ram_enabled: m.prg_ram_enabled,
            prg_ram_write_protect: m.prg_ram_write_protect,
            irq_cycle_mode: m.irq_cycle_mode,
            irq_prescaler: m.irq_prescaler,
            irq_delay: m.irq_delay,
        });

        let mmc5 = self.mmc5.as_ref().map(|m| Mmc5State {
            prg_mode: m.prg_mode,
            chr_mode: m.chr_mode,
            exram_mode: m.exram_mode,
            prg_ram_protect_1: m.prg_ram_protect_1,
            prg_ram_protect_2: m.prg_ram_protect_2,
            nametable_map: m.nametable_map,
            fill_tile: m.fill_tile,
            fill_attr: m.fill_attr,
            prg_ram_bank: m.prg_ram_bank,
            prg_banks: m.prg_banks,
            chr_upper: m.chr_upper,
            sprite_chr_banks: m.sprite_chr_banks,
            bg_chr_banks: m.bg_chr_banks,
            exram: m.exram.clone(),
            irq_scanline_compare: m.irq_scanline_compare,
            irq_enabled: m.irq_enabled,
            irq_pending: m.irq_pending.get(),
            in_frame: m.in_frame.get(),
            scanline_counter: m.scanline_counter.get(),
            multiplier_a: m.multiplier_a,
            multiplier_b: m.multiplier_b,
            split_control: m.split_control,
            split_scroll: m.split_scroll,
            split_bank: m.split_bank,
            ppu_ctrl: m.ppu_ctrl.get(),
            ppu_mask: m.ppu_mask.get(),
            cached_tile_x: m.cached_tile_x.get(),
            cached_tile_y: m.cached_tile_y.get(),
            cached_ext_palette: m.cached_ext_palette.get(),
            cached_ext_bank: m.cached_ext_bank.get(),
            ppu_data_uses_bg_banks: m.ppu_data_uses_bg_banks,
            audio: Mmc5AudioState {
                pulse1: Mmc5PulseState {
                    duty: m.pulse1.duty,
                    length_counter: m.pulse1.length_counter,
                    envelope_divider: m.pulse1.envelope_divider,
                    envelope_decay: m.pulse1.envelope_decay,
                    envelope_disable: m.pulse1.envelope_disable,
                    envelope_start: m.pulse1.envelope_start,
                    volume: m.pulse1.volume,
                    timer: m.pulse1.timer,
                    timer_reload: m.pulse1.timer_reload,
                    duty_counter: m.pulse1.duty_counter,
                    length_enabled: m.pulse1.length_enabled,
                },
                pulse2: Mmc5PulseState {
                    duty: m.pulse2.duty,
                    length_counter: m.pulse2.length_counter,
                    envelope_divider: m.pulse2.envelope_divider,
                    envelope_decay: m.pulse2.envelope_decay,
                    envelope_disable: m.pulse2.envelope_disable,
                    envelope_start: m.pulse2.envelope_start,
                    volume: m.pulse2.volume,
                    timer: m.pulse2.timer,
                    timer_reload: m.pulse2.timer_reload,
                    duty_counter: m.pulse2.duty_counter,
                    length_enabled: m.pulse2.length_enabled,
                },
                pulse1_enabled: m.pulse1_enabled,
                pulse2_enabled: m.pulse2_enabled,
                pcm_irq_enabled: m.pcm_irq_enabled,
                pcm_read_mode: m.pcm_read_mode,
                pcm_irq_pending: m.pcm_irq_pending.get(),
                pcm_dac: m.pcm_dac,
                audio_frame_accum: m.audio_frame_accum,
                audio_even_cycle: m.audio_even_cycle,
            },
        });

        let namco163 = self.namco163.as_ref().map(|n| Namco163State {
            chr_banks: n.chr_banks,
            prg_banks: n.prg_banks,
            sound_disable: n.sound_disable,
            chr_nt_disabled_low: n.chr_nt_disabled_low,
            chr_nt_disabled_high: n.chr_nt_disabled_high,
            wram_write_enable: n.wram_write_enable,
            wram_write_protect: n.wram_write_protect,
            internal_addr: n.internal_addr.get(),
            internal_auto_increment: n.internal_auto_increment,
            irq_counter: n.irq_counter,
            irq_enabled: n.irq_enabled,
            irq_pending: n.irq_pending.get(),
            audio_delay: n.audio_delay,
            audio_channel_index: n.audio_channel_index,
            audio_outputs: n.audio_outputs,
            audio_current: n.audio_current,
        });
        let mapper18 = self.jaleco_ss88006.as_ref().map(|m| Mapper18State {
            prg_banks: m.prg_banks,
            chr_banks: m.chr_banks,
            prg_ram_enabled: m.prg_ram_enabled,
            prg_ram_write_enabled: m.prg_ram_write_enabled,
            irq_reload: m.irq_reload,
            irq_counter: m.irq_counter,
            irq_control: m.irq_control,
            irq_pending: m.irq_pending.get(),
        });
        let mapper210 = self.namco210.as_ref().map(|m| Mapper210State {
            chr_banks: m.chr_banks,
            prg_banks: m.prg_banks,
            namco340: m.namco340,
            prg_ram_enabled: m.prg_ram_enabled,
        });

        let fme7 = self.fme7.as_ref().map(|f| Fme7State {
            command: f.command,
            chr_banks: f.chr_banks,
            prg_banks: f.prg_banks,
            prg_bank_6000: f.prg_bank_6000,
            prg_ram_enabled: f.prg_ram_enabled,
            prg_ram_select: f.prg_ram_select,
            irq_counter: f.irq_counter,
            irq_counter_enabled: f.irq_counter_enabled,
            irq_enabled: f.irq_enabled,
            irq_pending: f.irq_pending.get(),
        });

        let bandai_fcg = self.bandai_fcg.as_ref().map(|b| BandaiFcgState {
            chr_banks: b.chr_banks,
            prg_bank: b.prg_bank,
            irq_counter: b.irq_counter,
            irq_latch: b.irq_latch,
            irq_enabled: b.irq_enabled,
            irq_pending: b.irq_pending.get(),
            outer_prg_bank: b.outer_prg_bank,
            prg_ram_enabled: b.prg_ram_enabled,
        });

        let mapper34 = if self.mapper == 34 {
            Some(Mapper34State {
                nina001: self.mapper34_nina001,
                chr_bank_1: self.chr_bank_1,
            })
        } else {
            None
        };

        let mapper93 = if self.mapper == 93 {
            Some(Mapper93State {
                chr_ram_enabled: self.mapper93_chr_ram_enabled,
            })
        } else {
            None
        };

        let mapper184 = if self.mapper == 184 {
            Some(Mapper184State {
                chr_bank_1: self.chr_bank_1,
            })
        } else {
            None
        };

        let vrc1 = self.vrc1.as_ref().map(|v| Vrc1State {
            prg_banks: v.prg_banks,
            chr_bank_0: v.chr_bank_0,
            chr_bank_1: v.chr_bank_1,
        });
        let vrc2_vrc4 = self.vrc2_vrc4.as_ref().map(|v| Vrc2Vrc4State {
            prg_banks: v.prg_banks,
            chr_banks: v.chr_banks,
            wram_enabled: v.wram_enabled,
            prg_swap_mode: v.prg_swap_mode,
            vrc4_mode: v.vrc4_mode,
            latch: v.latch,
            irq_latch: v.irq_latch,
            irq_counter: v.irq_counter,
            irq_enable_after_ack: v.irq_enable_after_ack,
            irq_enabled: v.irq_enabled,
            irq_cycle_mode: v.irq_cycle_mode,
            irq_prescaler: v.irq_prescaler,
            irq_pending: v.irq_pending.get(),
        });

        let mapper15 = self.mapper15.as_ref().map(|m| Mapper15State {
            mode: m.mode,
            data: m.data,
        });
        let mapper72 = if matches!(self.mapper, 72 | 92) {
            Some(Mapper72State {
                last_command: self.chr_bank_1,
            })
        } else {
            None
        };
        let mapper58 = if matches!(self.mapper, 58 | 213) {
            Some(Mapper58State {
                nrom128: self.mapper58_nrom128,
            })
        } else {
            None
        };
        let mapper59 = if self.mapper == 59 {
            Some(Mapper59State {
                latch: self.mapper59_latch,
                locked: self.mapper59_locked,
            })
        } else {
            None
        };
        let mapper60 = if self.mapper == 60 {
            Some(Mapper60State {
                game_select: self.mapper60_game_select,
            })
        } else {
            None
        };
        let mapper61 = if self.mapper == 61 {
            Some(Mapper61State {
                latch: self.mapper61_latch,
            })
        } else {
            None
        };
        let mapper63 = if self.mapper == 63 {
            Some(Mapper63State {
                latch: self.mapper63_latch,
            })
        } else {
            None
        };
        let mapper137 = if self.mapper == 137 {
            Some(Mapper137State {
                index: self.mapper137_index,
                registers: self.mapper137_registers,
            })
        } else {
            None
        };
        let mapper142 = if self.mapper == 142 {
            Some(Mapper142State {
                bank_select: self.mapper142_bank_select,
                prg_banks: self.mapper142_prg_banks,
            })
        } else {
            None
        };
        let mapper150 = if self.mapper == 150 {
            Some(Mapper150State {
                index: self.mapper150_index,
                registers: self.mapper150_registers,
            })
        } else {
            None
        };
        let mapper225 = if matches!(self.mapper, 225 | 255) {
            Some(Mapper225State {
                nrom128: self.mapper225_nrom128,
            })
        } else {
            None
        };
        let mapper232 = if self.mapper == 232 {
            Some(Mapper232State {
                outer_bank: self.mapper232_outer_bank,
            })
        } else {
            None
        };
        let mapper41 = if self.mapper == 41 {
            Some(Mapper41State {
                inner_bank: self.mapper41_inner_bank,
            })
        } else {
            None
        };
        let mapper40 = self.mapper40.as_ref().map(|m| Mapper40State {
            irq_counter: m.irq_counter,
            irq_enabled: m.irq_enabled,
            irq_pending: m.irq_pending.get(),
        });
        let mapper42 = self.mapper42.as_ref().map(|m| Mapper42State {
            irq_counter: m.irq_counter,
            irq_enabled: m.irq_enabled,
            irq_pending: m.irq_pending.get(),
        });
        let mapper43 = self.mapper43.as_ref().map(|m| Mapper43State {
            irq_counter: m.irq_counter,
            irq_enabled: m.irq_enabled,
            irq_pending: m.irq_pending.get(),
        });
        let mapper50 = self.mapper50.as_ref().map(|m| Mapper50State {
            irq_counter: m.irq_counter,
            irq_enabled: m.irq_enabled,
            irq_pending: m.irq_pending.get(),
        });
        let irem_g101 = self.irem_g101.as_ref().map(|g| IremG101State {
            prg_banks: g.prg_banks,
            chr_banks: g.chr_banks,
            prg_mode: g.prg_mode,
        });
        let irem_h3001 = self.irem_h3001.as_ref().map(|h| IremH3001State {
            prg_banks: h.prg_banks,
            chr_banks: h.chr_banks,
            prg_mode: h.prg_mode,
            irq_reload: h.irq_reload,
            irq_counter: h.irq_counter,
            irq_enabled: h.irq_enabled,
            irq_pending: h.irq_pending.get(),
        });
        let vrc3 = self.vrc3.as_ref().map(|v| Vrc3State {
            irq_reload: v.irq_reload,
            irq_counter: v.irq_counter,
            irq_enable_on_ack: v.irq_enable_on_ack,
            irq_enabled: v.irq_enabled,
            irq_mode_8bit: v.irq_mode_8bit,
            irq_pending: v.irq_pending.get(),
        });
        let mapper233 = if self.mapper == 233 {
            Some(Mapper233State {
                nrom128: self.mapper233_nrom128,
            })
        } else {
            None
        };
        let mapper234 = if self.mapper == 234 {
            Some(Mapper234State {
                reg0: self.mapper234_reg0,
                reg1: self.mapper234_reg1,
            })
        } else {
            None
        };
        let mapper235 = if self.mapper == 235 {
            Some(Mapper235State {
                nrom128: self.mapper235_nrom128,
            })
        } else {
            None
        };
        let mapper202 = if self.mapper == 202 {
            Some(Mapper202State {
                mode_32k: self.mapper202_32k_mode,
            })
        } else {
            None
        };
        let mapper37 = if self.mapper == 37 {
            Some(Mapper37State {
                outer_bank: self.mapper37_outer_bank,
            })
        } else {
            None
        };
        let mapper44 = if self.mapper == 44 {
            Some(Mapper44State {
                outer_bank: self.mapper44_outer_bank,
            })
        } else {
            None
        };
        let mapper103 = if self.mapper == 103 {
            Some(Mapper103State {
                prg_ram_disabled: self.mapper103_prg_ram_disabled,
            })
        } else {
            None
        };
        let mapper12 = if self.mapper == 12 {
            Some(Mapper12State {
                chr_outer: self.mapper12_chr_outer,
            })
        } else {
            None
        };
        let mapper114 = if matches!(self.mapper, 114 | 182) {
            Some(Mapper114State {
                nrom_override: self.mapper114_override,
                chr_outer_bank: self.mapper114_chr_outer_bank,
            })
        } else {
            None
        };
        let mapper212 = if self.mapper == 212 {
            Some(Mapper212State {
                mode_32k: self.mapper212_32k_mode,
            })
        } else {
            None
        };
        let mapper47 = if self.mapper == 47 {
            Some(Mapper47State {
                outer_bank: self.mapper47_outer_bank,
            })
        } else {
            None
        };
        let mapper123 = if self.mapper == 123 {
            Some(Mapper123State {
                nrom_override: self.mapper123_override,
            })
        } else {
            None
        };
        let mapper115 = if matches!(self.mapper, 115 | 248) {
            Some(Mapper115State {
                nrom_override: self.mapper115_override,
                chr_outer_bank: self.mapper115_chr_outer_bank,
            })
        } else {
            None
        };
        let mapper205 = if self.mapper == 205 {
            Some(Mapper205State {
                block: self.mapper205_block,
            })
        } else {
            None
        };
        let mapper226 = if self.mapper == 226 {
            Some(Mapper226State {
                nrom128: self.mapper226_nrom128,
            })
        } else {
            None
        };
        let mapper230 = if self.mapper == 230 {
            Some(Mapper230State {
                contra_mode: self.mapper230_contra_mode,
                nrom128: self.mapper230_nrom128,
            })
        } else {
            None
        };
        let mapper228 = if self.mapper == 228 {
            Some(Mapper228State {
                chip_select: self.mapper228_chip_select,
                nrom128: self.mapper228_nrom128,
            })
        } else {
            None
        };
        let mapper242 = if self.mapper == 242 {
            Some(Mapper242State {
                latch: self.mapper242_latch,
            })
        } else {
            None
        };
        let mapper243 = if self.mapper == 243 {
            Some(Mapper243State {
                index: self.mapper243_index,
                registers: self.mapper243_registers,
            })
        } else {
            None
        };
        let mapper221 = if self.mapper == 221 {
            Some(Mapper221State {
                mode: self.mapper221_mode,
                outer_bank: self.mapper221_outer_bank,
                chr_write_protect: self.mapper221_chr_write_protect,
            })
        } else {
            None
        };
        let mapper191 = if self.mapper == 191 {
            Some(Mapper191State {
                outer_bank: self.mapper191_outer_bank,
            })
        } else {
            None
        };
        let mapper195 = if self.mapper == 195 {
            Some(Mapper195State {
                mode: self.mapper195_mode,
            })
        } else {
            None
        };
        let mapper208 = if self.mapper == 208 {
            Some(Mapper208State {
                protection_index: self.mapper208_protection_index,
                protection_regs: self.mapper208_protection_regs,
            })
        } else {
            None
        };
        let mapper189 = if self.mapper == 189 {
            Some(Mapper189State {
                prg_bank: self.mapper189_prg_bank,
            })
        } else {
            None
        };
        let mapper185 = if self.mapper == 185 {
            Some(Mapper185State {
                disabled_reads: self.mapper185_disabled_reads.get(),
            })
        } else {
            None
        };
        let mapper236 = if self.mapper == 236 {
            Some(Mapper236State {
                mode: self.mapper236_mode,
                outer_bank: self.mapper236_outer_bank,
            })
        } else {
            None
        };
        let mapper227 = if self.mapper == 227 {
            Some(Mapper227State {
                latch: self.mapper227_latch,
            })
        } else {
            None
        };
        let mapper246 = self.mapper246.as_ref().map(|m| Mapper246State {
            prg_banks: m.prg_banks,
            chr_banks: m.chr_banks,
        });
        let sunsoft3 = self.sunsoft3.as_ref().map(|m| Sunsoft3State {
            chr_banks: m.chr_banks,
            prg_bank: m.prg_bank,
            irq_counter: m.irq_counter,
            irq_enabled: m.irq_enabled,
            irq_pending: m.irq_pending.get(),
            irq_write_high: m.irq_write_high,
        });
        let sunsoft4 = self.sunsoft4.as_ref().map(|m| Sunsoft4State {
            chr_banks: m.chr_banks,
            nametable_banks: m.nametable_banks,
            control: m.control,
            prg_bank: m.prg_bank,
            prg_ram_enabled: m.prg_ram_enabled,
        });
        let taito_tc0190 = self.taito_tc0190.as_ref().map(|m| TaitoTc0190State {
            prg_banks: m.prg_banks,
            chr_banks: m.chr_banks,
            irq_latch: m.irq_latch,
            irq_counter: m.irq_counter,
            irq_reload: m.irq_reload,
            irq_enabled: m.irq_enabled,
            irq_pending: m.irq_pending.get(),
            irq_delay: m.irq_delay,
        });
        let taito_x1005 = self.taito_x1005.as_ref().map(|m| TaitoX1005State {
            prg_banks: m.prg_banks,
            chr_banks: m.chr_banks,
            ram_enabled: m.ram_enabled,
        });
        let taito_x1017 = self.taito_x1017.as_ref().map(|m| TaitoX1017State {
            prg_banks: m.prg_banks,
            chr_banks: m.chr_banks,
            ram_enabled: m.ram_enabled,
            chr_invert: m.chr_invert,
        });
        let vrc6 = self.vrc6.as_ref().map(|m| Vrc6State {
            prg_bank_16k: m.prg_bank_16k,
            prg_bank_8k: m.prg_bank_8k,
            chr_banks: m.chr_banks,
            banking_control: m.banking_control,
            irq_latch: m.irq_latch,
            irq_counter: m.irq_counter,
            irq_enable_after_ack: m.irq_enable_after_ack,
            irq_enabled: m.irq_enabled,
            irq_cycle_mode: m.irq_cycle_mode,
            irq_prescaler: m.irq_prescaler,
            irq_pending: m.irq_pending.get(),
            audio_halt: m.audio_halt,
            audio_freq_shift: m.audio_freq_shift,
            pulse1: Vrc6PulseState {
                volume: m.pulse1.volume,
                duty: m.pulse1.duty,
                ignore_duty: m.pulse1.ignore_duty,
                period: m.pulse1.period,
                enabled: m.pulse1.enabled,
                step: m.pulse1.step,
                divider: m.pulse1.divider,
            },
            pulse2: Vrc6PulseState {
                volume: m.pulse2.volume,
                duty: m.pulse2.duty,
                ignore_duty: m.pulse2.ignore_duty,
                period: m.pulse2.period,
                enabled: m.pulse2.enabled,
                step: m.pulse2.step,
                divider: m.pulse2.divider,
            },
            saw: Vrc6SawState {
                rate: m.saw.rate,
                period: m.saw.period,
                enabled: m.saw.enabled,
                step: m.saw.step,
                divider: m.saw.divider,
                accumulator: m.saw.accumulator,
            },
        });

        CartridgeState {
            mapper: self.mapper,
            mirroring: self.mirroring,
            prg_bank: self.get_prg_bank(),
            chr_bank: self.get_chr_bank(),
            prg_ram: self.prg_ram.clone(),
            chr_ram: self.chr_ram.clone(),
            has_valid_save_data: self.has_valid_save_data,
            mmc1,
            mmc2,
            mmc3,
            mmc5,
            namco163,
            fme7,
            bandai_fcg,
            mapper34,
            mapper93,
            mapper184,
            vrc1,
            vrc2_vrc4,
            mapper15,
            mapper72,
            mapper58,
            mapper59,
            mapper60,
            mapper225,
            mapper232,
            mapper234,
            mapper235,
            mapper202,
            mapper212,
            mapper226,
            mapper230,
            mapper228,
            mapper242,
            mapper243,
            mapper221,
            mapper191,
            mapper195,
            mapper208,
            mapper189,
            mapper236,
            mapper227,
            mapper246,
            sunsoft4,
            taito_tc0190,
            taito_x1005,
            taito_x1017,
            mapper233,
            mapper41,
            mapper40,
            mapper42,
            mapper50,
            irem_g101,
            vrc3,
            mapper43,
            irem_h3001,
            mapper103,
            mapper37,
            mapper44,
            mapper47,
            mapper12,
            mapper114,
            mapper123,
            mapper115,
            mapper205,
            mapper61,
            mapper185,
            sunsoft3,
            mapper63,
            mapper137,
            mapper142,
            mapper150,
            mapper18,
            mapper210,
            vrc6,
        }
    }

    pub fn restore_state(&mut self, state: &CartridgeState) {
        if state.mapper != self.mapper {
            return;
        }

        self.mirroring = state.mirroring;
        self.set_prg_bank(state.prg_bank);
        self.set_chr_bank(state.chr_bank);
        if let Some(saved) = state.mapper34.as_ref() {
            self.mapper34_nina001 = saved.nina001;
            self.chr_bank_1 = saved.chr_bank_1;
        }
        if let Some(saved) = state.mapper93.as_ref() {
            self.mapper93_chr_ram_enabled = saved.chr_ram_enabled;
        }
        if let Some(saved) = state.mapper184.as_ref() {
            self.chr_bank_1 = saved.chr_bank_1;
        }
        if let Some(saved) = state.vrc1.as_ref() {
            self.chr_bank_1 = saved.chr_bank_1;
        }
        if let Some(saved) = state.vrc2_vrc4.as_ref() {
            self.prg_bank = saved.prg_banks[0];
            self.chr_bank = saved.chr_banks[0] as u8;
        }
        self.has_valid_save_data = state.has_valid_save_data;

        let prg_len = self.prg_ram.len().min(state.prg_ram.len());
        if prg_len > 0 {
            self.prg_ram[..prg_len].copy_from_slice(&state.prg_ram[..prg_len]);
        }

        let chr_len = self.chr_ram.len().min(state.chr_ram.len());
        if chr_len > 0 {
            self.chr_ram[..chr_len].copy_from_slice(&state.chr_ram[..chr_len]);
        }

        if let (Some(ref mut mmc1), Some(saved)) = (self.mmc1.as_mut(), state.mmc1.as_ref()) {
            mmc1.shift_register = saved.shift_register;
            mmc1.shift_count = saved.shift_count;
            mmc1.control = saved.control;
            mmc1.chr_bank_0 = saved.chr_bank_0;
            mmc1.chr_bank_1 = saved.chr_bank_1;
            mmc1.prg_bank = saved.prg_bank;
            mmc1.prg_ram_disable = saved.prg_ram_disable;
        }

        if let (Some(ref mut mmc2), Some(saved)) = (self.mmc2.as_mut(), state.mmc2.as_ref()) {
            mmc2.prg_bank = saved.prg_bank;
            mmc2.chr_bank_0_fd = saved.chr_bank_0_fd;
            mmc2.chr_bank_0_fe = saved.chr_bank_0_fe;
            mmc2.chr_bank_1_fd = saved.chr_bank_1_fd;
            mmc2.chr_bank_1_fe = saved.chr_bank_1_fe;
            mmc2.latch_0.set(saved.latch_0);
            mmc2.latch_1.set(saved.latch_1);
        }

        if let (Some(ref mut mmc3), Some(saved)) = (self.mmc3.as_mut(), state.mmc3.as_ref()) {
            mmc3.bank_select = saved.bank_select;
            mmc3.bank_registers = saved.bank_registers;
            mmc3.extra_bank_registers = saved.extra_bank_registers;
            mmc3.irq_latch = saved.irq_latch;
            mmc3.irq_counter = saved.irq_counter;
            mmc3.irq_reload = saved.irq_reload;
            mmc3.irq_enabled = saved.irq_enabled;
            mmc3.irq_pending.set(saved.irq_pending);
            mmc3.prg_ram_enabled = saved.prg_ram_enabled;
            mmc3.prg_ram_write_protect = saved.prg_ram_write_protect;
            mmc3.irq_cycle_mode = saved.irq_cycle_mode;
            mmc3.irq_prescaler = saved.irq_prescaler;
            mmc3.irq_delay = saved.irq_delay;
        }

        if let (Some(ref mut mmc5), Some(saved)) = (self.mmc5.as_mut(), state.mmc5.as_ref()) {
            mmc5.prg_mode = saved.prg_mode;
            mmc5.chr_mode = saved.chr_mode;
            mmc5.exram_mode = saved.exram_mode;
            mmc5.prg_ram_protect_1 = saved.prg_ram_protect_1;
            mmc5.prg_ram_protect_2 = saved.prg_ram_protect_2;
            mmc5.nametable_map = saved.nametable_map;
            mmc5.fill_tile = saved.fill_tile;
            mmc5.fill_attr = saved.fill_attr;
            mmc5.prg_ram_bank = saved.prg_ram_bank;
            mmc5.prg_banks = saved.prg_banks;
            mmc5.chr_upper = saved.chr_upper;
            mmc5.sprite_chr_banks = saved.sprite_chr_banks;
            mmc5.bg_chr_banks = saved.bg_chr_banks;
            mmc5.exram.clone_from(&saved.exram);
            mmc5.irq_scanline_compare = saved.irq_scanline_compare;
            mmc5.irq_enabled = saved.irq_enabled;
            mmc5.irq_pending.set(saved.irq_pending);
            mmc5.in_frame.set(saved.in_frame);
            mmc5.scanline_counter.set(saved.scanline_counter);
            mmc5.multiplier_a = saved.multiplier_a;
            mmc5.multiplier_b = saved.multiplier_b;
            mmc5.split_control = saved.split_control;
            mmc5.split_scroll = saved.split_scroll;
            mmc5.split_bank = saved.split_bank;
            mmc5.ppu_ctrl.set(saved.ppu_ctrl);
            mmc5.ppu_mask.set(saved.ppu_mask);
            mmc5.cached_tile_x.set(saved.cached_tile_x);
            mmc5.cached_tile_y.set(saved.cached_tile_y);
            mmc5.cached_ext_palette.set(saved.cached_ext_palette);
            mmc5.cached_ext_bank.set(saved.cached_ext_bank);
            mmc5.ppu_data_uses_bg_banks = saved.ppu_data_uses_bg_banks;
            mmc5.pulse1.duty = saved.audio.pulse1.duty;
            mmc5.pulse1.length_counter = saved.audio.pulse1.length_counter;
            mmc5.pulse1.envelope_divider = saved.audio.pulse1.envelope_divider;
            mmc5.pulse1.envelope_decay = saved.audio.pulse1.envelope_decay;
            mmc5.pulse1.envelope_disable = saved.audio.pulse1.envelope_disable;
            mmc5.pulse1.envelope_start = saved.audio.pulse1.envelope_start;
            mmc5.pulse1.volume = saved.audio.pulse1.volume;
            mmc5.pulse1.timer = saved.audio.pulse1.timer;
            mmc5.pulse1.timer_reload = saved.audio.pulse1.timer_reload;
            mmc5.pulse1.duty_counter = saved.audio.pulse1.duty_counter;
            mmc5.pulse1.length_enabled = saved.audio.pulse1.length_enabled;
            mmc5.pulse2.duty = saved.audio.pulse2.duty;
            mmc5.pulse2.length_counter = saved.audio.pulse2.length_counter;
            mmc5.pulse2.envelope_divider = saved.audio.pulse2.envelope_divider;
            mmc5.pulse2.envelope_decay = saved.audio.pulse2.envelope_decay;
            mmc5.pulse2.envelope_disable = saved.audio.pulse2.envelope_disable;
            mmc5.pulse2.envelope_start = saved.audio.pulse2.envelope_start;
            mmc5.pulse2.volume = saved.audio.pulse2.volume;
            mmc5.pulse2.timer = saved.audio.pulse2.timer;
            mmc5.pulse2.timer_reload = saved.audio.pulse2.timer_reload;
            mmc5.pulse2.duty_counter = saved.audio.pulse2.duty_counter;
            mmc5.pulse2.length_enabled = saved.audio.pulse2.length_enabled;
            mmc5.pulse1_enabled = saved.audio.pulse1_enabled;
            mmc5.pulse2_enabled = saved.audio.pulse2_enabled;
            mmc5.pcm_irq_enabled = saved.audio.pcm_irq_enabled;
            mmc5.pcm_read_mode = saved.audio.pcm_read_mode;
            mmc5.pcm_irq_pending.set(saved.audio.pcm_irq_pending);
            mmc5.pcm_dac = saved.audio.pcm_dac;
            mmc5.audio_frame_accum = saved.audio.audio_frame_accum;
            mmc5.audio_even_cycle = saved.audio.audio_even_cycle;
        }

        if let (Some(ref mut namco163), Some(saved)) =
            (self.namco163.as_mut(), state.namco163.as_ref())
        {
            namco163.chr_banks = saved.chr_banks;
            namco163.prg_banks = saved.prg_banks;
            namco163.sound_disable = saved.sound_disable;
            namco163.chr_nt_disabled_low = saved.chr_nt_disabled_low;
            namco163.chr_nt_disabled_high = saved.chr_nt_disabled_high;
            namco163.wram_write_enable = saved.wram_write_enable;
            namco163.wram_write_protect = saved.wram_write_protect;
            namco163.internal_addr.set(saved.internal_addr);
            namco163.internal_auto_increment = saved.internal_auto_increment;
            namco163.irq_counter = saved.irq_counter;
            namco163.irq_enabled = saved.irq_enabled;
            namco163.irq_pending.set(saved.irq_pending);
            namco163.audio_delay = saved.audio_delay;
            namco163.audio_channel_index = saved.audio_channel_index;
            namco163.audio_outputs = saved.audio_outputs;
            namco163.audio_current = saved.audio_current;
        }
        if let (Some(ref mut mapper18), Some(saved)) =
            (self.jaleco_ss88006.as_mut(), state.mapper18.as_ref())
        {
            mapper18.prg_banks = saved.prg_banks;
            mapper18.chr_banks = saved.chr_banks;
            mapper18.prg_ram_enabled = saved.prg_ram_enabled;
            mapper18.prg_ram_write_enabled = saved.prg_ram_write_enabled;
            mapper18.irq_reload = saved.irq_reload;
            mapper18.irq_counter = saved.irq_counter;
            mapper18.irq_control = saved.irq_control;
            mapper18.irq_pending.set(saved.irq_pending);
            self.prg_bank = saved.prg_banks[0];
            self.chr_bank = saved.chr_banks[0];
        }
        if let (Some(ref mut mapper210), Some(saved)) =
            (self.namco210.as_mut(), state.mapper210.as_ref())
        {
            mapper210.chr_banks = saved.chr_banks;
            mapper210.prg_banks = saved.prg_banks;
            mapper210.namco340 = saved.namco340;
            mapper210.prg_ram_enabled = saved.prg_ram_enabled;
            self.prg_bank = saved.prg_banks[0];
            self.chr_bank = saved.chr_banks[0];
        }

        if let (Some(ref mut fme7), Some(saved)) = (self.fme7.as_mut(), state.fme7.as_ref()) {
            fme7.command = saved.command;
            fme7.chr_banks = saved.chr_banks;
            fme7.prg_banks = saved.prg_banks;
            fme7.prg_bank_6000 = saved.prg_bank_6000;
            fme7.prg_ram_enabled = saved.prg_ram_enabled;
            fme7.prg_ram_select = saved.prg_ram_select;
            fme7.irq_counter = saved.irq_counter;
            fme7.irq_counter_enabled = saved.irq_counter_enabled;
            fme7.irq_enabled = saved.irq_enabled;
            fme7.irq_pending.set(saved.irq_pending);
        }

        if let (Some(ref mut bandai), Some(saved)) =
            (self.bandai_fcg.as_mut(), state.bandai_fcg.as_ref())
        {
            bandai.chr_banks = saved.chr_banks;
            bandai.prg_bank = saved.prg_bank;
            bandai.irq_counter = saved.irq_counter;
            bandai.irq_latch = saved.irq_latch;
            bandai.irq_enabled = saved.irq_enabled;
            bandai.irq_pending.set(saved.irq_pending);
            bandai.outer_prg_bank = saved.outer_prg_bank;
            bandai.prg_ram_enabled = saved.prg_ram_enabled;
        }

        if let (Some(ref mut vrc1), Some(saved)) = (self.vrc1.as_mut(), state.vrc1.as_ref()) {
            vrc1.prg_banks = saved.prg_banks;
            vrc1.chr_bank_0 = saved.chr_bank_0;
            vrc1.chr_bank_1 = saved.chr_bank_1;
        }
        if let (Some(ref mut vrc2_vrc4), Some(saved)) =
            (self.vrc2_vrc4.as_mut(), state.vrc2_vrc4.as_ref())
        {
            vrc2_vrc4.prg_banks = saved.prg_banks;
            vrc2_vrc4.chr_banks = saved.chr_banks;
            vrc2_vrc4.wram_enabled = saved.wram_enabled;
            vrc2_vrc4.prg_swap_mode = saved.prg_swap_mode;
            vrc2_vrc4.vrc4_mode = saved.vrc4_mode;
            vrc2_vrc4.latch = saved.latch;
            vrc2_vrc4.irq_latch = saved.irq_latch;
            vrc2_vrc4.irq_counter = saved.irq_counter;
            vrc2_vrc4.irq_enable_after_ack = saved.irq_enable_after_ack;
            vrc2_vrc4.irq_enabled = saved.irq_enabled;
            vrc2_vrc4.irq_cycle_mode = saved.irq_cycle_mode;
            vrc2_vrc4.irq_prescaler = saved.irq_prescaler;
            vrc2_vrc4.irq_pending.set(saved.irq_pending);
            self.prg_bank = saved.prg_banks[0];
            self.chr_bank = saved.chr_banks[0] as u8;
        }
        if let (Some(ref mut vrc6), Some(saved)) = (self.vrc6.as_mut(), state.vrc6.as_ref()) {
            vrc6.prg_bank_16k = saved.prg_bank_16k;
            vrc6.prg_bank_8k = saved.prg_bank_8k;
            vrc6.chr_banks = saved.chr_banks;
            vrc6.banking_control = saved.banking_control;
            vrc6.irq_latch = saved.irq_latch;
            vrc6.irq_counter = saved.irq_counter;
            vrc6.irq_enable_after_ack = saved.irq_enable_after_ack;
            vrc6.irq_enabled = saved.irq_enabled;
            vrc6.irq_cycle_mode = saved.irq_cycle_mode;
            vrc6.irq_prescaler = saved.irq_prescaler;
            vrc6.irq_pending.set(saved.irq_pending);
            vrc6.audio_halt = saved.audio_halt;
            vrc6.audio_freq_shift = saved.audio_freq_shift;
            vrc6.pulse1.volume = saved.pulse1.volume;
            vrc6.pulse1.duty = saved.pulse1.duty;
            vrc6.pulse1.ignore_duty = saved.pulse1.ignore_duty;
            vrc6.pulse1.period = saved.pulse1.period;
            vrc6.pulse1.enabled = saved.pulse1.enabled;
            vrc6.pulse1.step = saved.pulse1.step;
            vrc6.pulse1.divider = saved.pulse1.divider;
            vrc6.pulse2.volume = saved.pulse2.volume;
            vrc6.pulse2.duty = saved.pulse2.duty;
            vrc6.pulse2.ignore_duty = saved.pulse2.ignore_duty;
            vrc6.pulse2.period = saved.pulse2.period;
            vrc6.pulse2.enabled = saved.pulse2.enabled;
            vrc6.pulse2.step = saved.pulse2.step;
            vrc6.pulse2.divider = saved.pulse2.divider;
            vrc6.saw.rate = saved.saw.rate;
            vrc6.saw.period = saved.saw.period;
            vrc6.saw.enabled = saved.saw.enabled;
            vrc6.saw.step = saved.saw.step;
            vrc6.saw.divider = saved.saw.divider;
            vrc6.saw.accumulator = saved.saw.accumulator;
            self.prg_bank = saved.prg_bank_16k;
            self.chr_bank = saved.chr_banks[0];
            self.vrc6_apply_banking_control(saved.banking_control);
        }

        if let (Some(ref mut mapper15), Some(saved)) =
            (self.mapper15.as_mut(), state.mapper15.as_ref())
        {
            mapper15.mode = saved.mode;
            mapper15.data = saved.data;
        }
        if let Some(saved) = state.mapper72.as_ref() {
            self.chr_bank_1 = saved.last_command;
        }
        if let Some(saved) = state.mapper58.as_ref() {
            self.mapper58_nrom128 = saved.nrom128;
        }
        if let Some(saved) = state.mapper59.as_ref() {
            self.mapper59_latch = saved.latch;
            self.mapper59_locked = saved.locked;
            self.sync_mapper59_latch();
        }
        if let Some(saved) = state.mapper60.as_ref() {
            self.mapper60_game_select = saved.game_select;
            self.sync_mapper60_game();
        }
        if let Some(saved) = state.mapper61.as_ref() {
            self.mapper61_latch = saved.latch;
            self.sync_mapper61_latch();
        }
        if let Some(saved) = state.mapper63.as_ref() {
            self.mapper63_latch = saved.latch;
            self.prg_bank =
                (((saved.latch as usize) >> 2) % (self.prg_rom.len() / 0x4000).max(1)) as u8;
            self.mirroring = if saved.latch & 0x0001 != 0 {
                Mirroring::Horizontal
            } else {
                Mirroring::Vertical
            };
        }
        if let Some(saved) = state.mapper137.as_ref() {
            self.mapper137_index = saved.index;
            self.mapper137_registers = saved.registers;
            self.update_mapper137_state();
        }
        if let Some(saved) = state.mapper142.as_ref() {
            self.mapper142_bank_select = saved.bank_select;
            self.mapper142_prg_banks = saved.prg_banks;
            self.prg_bank = saved.prg_banks[0];
        }
        if let Some(saved) = state.mapper150.as_ref() {
            self.mapper150_index = saved.index;
            self.mapper150_registers = saved.registers;
            self.update_mapper150_state();
        }
        if let Some(saved) = state.mapper225.as_ref() {
            self.mapper225_nrom128 = saved.nrom128;
        }
        if let Some(saved) = state.mapper232.as_ref() {
            self.mapper232_outer_bank = saved.outer_bank;
        }
        if let Some(saved) = state.mapper41.as_ref() {
            self.mapper41_inner_bank = saved.inner_bank;
            self.sync_mapper41_chr_bank();
        }
        if let Some(saved) = state.mapper233.as_ref() {
            self.mapper233_nrom128 = saved.nrom128;
        }
        if let Some(saved) = state.mapper234.as_ref() {
            self.mapper234_reg0 = saved.reg0;
            self.mapper234_reg1 = saved.reg1;
            self.sync_mapper234_state();
        }
        if let Some(saved) = state.mapper235.as_ref() {
            self.mapper235_nrom128 = saved.nrom128;
        }
        if let Some(saved) = state.mapper202.as_ref() {
            self.mapper202_32k_mode = saved.mode_32k;
        }
        if let Some(saved) = state.mapper37.as_ref() {
            self.mapper37_outer_bank = saved.outer_bank;
        }
        if let Some(saved) = state.mapper44.as_ref() {
            self.mapper44_outer_bank = saved.outer_bank;
        }
        if let Some(saved) = state.mapper103.as_ref() {
            self.mapper103_prg_ram_disabled = saved.prg_ram_disabled;
        }
        if let Some(saved) = state.mapper12.as_ref() {
            self.mapper12_chr_outer = saved.chr_outer;
        }
        if let Some(saved) = state.mapper114.as_ref() {
            self.mapper114_override = saved.nrom_override;
            self.mapper114_chr_outer_bank = saved.chr_outer_bank;
        }
        if let Some(saved) = state.mapper212.as_ref() {
            self.mapper212_32k_mode = saved.mode_32k;
        }
        if let Some(saved) = state.mapper47.as_ref() {
            self.mapper47_outer_bank = saved.outer_bank;
        }
        if let Some(saved) = state.mapper123.as_ref() {
            self.mapper123_override = saved.nrom_override;
        }
        if let Some(saved) = state.mapper115.as_ref() {
            self.mapper115_override = saved.nrom_override;
            self.mapper115_chr_outer_bank = saved.chr_outer_bank;
        }
        if let Some(saved) = state.mapper205.as_ref() {
            self.mapper205_block = saved.block;
        }
        if let Some(saved) = state.mapper226.as_ref() {
            self.mapper226_nrom128 = saved.nrom128;
        }
        if let Some(saved) = state.mapper230.as_ref() {
            self.mapper230_contra_mode = saved.contra_mode;
            self.mapper230_nrom128 = saved.nrom128;
        }
        if let Some(saved) = state.mapper228.as_ref() {
            self.mapper228_chip_select = saved.chip_select;
            self.mapper228_nrom128 = saved.nrom128;
        }
        if let Some(saved) = state.mapper242.as_ref() {
            self.mapper242_latch = saved.latch;
        }
        if let Some(saved) = state.mapper243.as_ref() {
            self.mapper243_index = saved.index;
            self.mapper243_registers = saved.registers;
        }
        if let Some(saved) = state.mapper221.as_ref() {
            self.mapper221_mode = saved.mode;
            self.mapper221_outer_bank = saved.outer_bank;
            self.mapper221_chr_write_protect = saved.chr_write_protect;
        }
        if let Some(saved) = state.mapper191.as_ref() {
            self.mapper191_outer_bank = saved.outer_bank;
        }
        if let Some(saved) = state.mapper195.as_ref() {
            self.mapper195_mode = saved.mode;
        }
        if let Some(saved) = state.mapper208.as_ref() {
            self.mapper208_protection_index = saved.protection_index;
            self.mapper208_protection_regs = saved.protection_regs;
        }
        if let Some(saved) = state.mapper189.as_ref() {
            self.mapper189_prg_bank = saved.prg_bank;
            self.prg_bank = saved.prg_bank;
        }
        if let Some(saved) = state.mapper185.as_ref() {
            self.mapper185_disabled_reads.set(saved.disabled_reads);
        }
        if let Some(saved) = state.mapper236.as_ref() {
            self.mapper236_mode = saved.mode;
            self.mapper236_outer_bank = saved.outer_bank;
        }
        if let Some(saved) = state.mapper227.as_ref() {
            self.mapper227_latch = saved.latch;
        }
        if let (Some(mapper246), Some(saved)) = (self.mapper246.as_mut(), state.mapper246.as_ref())
        {
            mapper246.prg_banks = saved.prg_banks;
            mapper246.chr_banks = saved.chr_banks;
        }
        if let (Some(ref mut mapper40), Some(saved)) =
            (self.mapper40.as_mut(), state.mapper40.as_ref())
        {
            mapper40.irq_counter = saved.irq_counter;
            mapper40.irq_enabled = saved.irq_enabled;
            mapper40.irq_pending.set(saved.irq_pending);
        }
        if let (Some(ref mut mapper42), Some(saved)) =
            (self.mapper42.as_mut(), state.mapper42.as_ref())
        {
            mapper42.irq_counter = saved.irq_counter;
            mapper42.irq_enabled = saved.irq_enabled;
            mapper42.irq_pending.set(saved.irq_pending);
        }
        if let (Some(ref mut mapper43), Some(saved)) =
            (self.mapper43.as_mut(), state.mapper43.as_ref())
        {
            mapper43.irq_counter = saved.irq_counter;
            mapper43.irq_enabled = saved.irq_enabled;
            mapper43.irq_pending.set(saved.irq_pending);
        }
        if let (Some(ref mut mapper50), Some(saved)) =
            (self.mapper50.as_mut(), state.mapper50.as_ref())
        {
            mapper50.irq_counter = saved.irq_counter;
            mapper50.irq_enabled = saved.irq_enabled;
            mapper50.irq_pending.set(saved.irq_pending);
        }
        if let (Some(ref mut g101), Some(saved)) =
            (self.irem_g101.as_mut(), state.irem_g101.as_ref())
        {
            g101.prg_banks = saved.prg_banks;
            g101.chr_banks = saved.chr_banks;
            g101.prg_mode = saved.prg_mode;
            self.prg_bank = saved.prg_banks[0];
            self.chr_bank = saved.chr_banks[0];
        }
        if let (Some(ref mut h3001), Some(saved)) =
            (self.irem_h3001.as_mut(), state.irem_h3001.as_ref())
        {
            h3001.prg_banks = saved.prg_banks;
            h3001.chr_banks = saved.chr_banks;
            h3001.prg_mode = saved.prg_mode;
            h3001.irq_reload = saved.irq_reload;
            h3001.irq_counter = saved.irq_counter;
            h3001.irq_enabled = saved.irq_enabled;
            h3001.irq_pending.set(saved.irq_pending);
            self.prg_bank = saved.prg_banks[0];
            self.chr_bank = saved.chr_banks[0];
        }
        if let (Some(ref mut vrc3), Some(saved)) = (self.vrc3.as_mut(), state.vrc3.as_ref()) {
            vrc3.irq_reload = saved.irq_reload;
            vrc3.irq_counter = saved.irq_counter;
            vrc3.irq_enable_on_ack = saved.irq_enable_on_ack;
            vrc3.irq_enabled = saved.irq_enabled;
            vrc3.irq_mode_8bit = saved.irq_mode_8bit;
            vrc3.irq_pending.set(saved.irq_pending);
        }

        if let (Some(ref mut sunsoft4), Some(saved)) =
            (self.sunsoft4.as_mut(), state.sunsoft4.as_ref())
        {
            sunsoft4.chr_banks = saved.chr_banks;
            sunsoft4.nametable_banks = saved.nametable_banks;
            sunsoft4.control = saved.control;
            sunsoft4.prg_bank = saved.prg_bank;
            sunsoft4.prg_ram_enabled = saved.prg_ram_enabled;
            sunsoft4.nametable_chr_rom = saved.control & 0x10 != 0;
            self.prg_bank = saved.prg_bank;
            self.chr_bank = saved.chr_banks[0];
            self.mirroring = Sunsoft4::decode_mirroring(saved.control);
        }
        if let (Some(ref mut sunsoft3), Some(saved)) =
            (self.sunsoft3.as_mut(), state.sunsoft3.as_ref())
        {
            sunsoft3.chr_banks = saved.chr_banks;
            sunsoft3.prg_bank = saved.prg_bank;
            sunsoft3.irq_counter = saved.irq_counter;
            sunsoft3.irq_enabled = saved.irq_enabled;
            sunsoft3.irq_pending.set(saved.irq_pending);
            sunsoft3.irq_write_high = saved.irq_write_high;
            self.prg_bank = saved.prg_bank;
            self.chr_bank = saved.chr_banks[0];
        }

        if let (Some(ref mut taito_tc0190), Some(saved)) =
            (self.taito_tc0190.as_mut(), state.taito_tc0190.as_ref())
        {
            taito_tc0190.prg_banks = saved.prg_banks;
            taito_tc0190.chr_banks = saved.chr_banks;
            taito_tc0190.irq_latch = saved.irq_latch;
            taito_tc0190.irq_counter = saved.irq_counter;
            taito_tc0190.irq_reload = saved.irq_reload;
            taito_tc0190.irq_enabled = saved.irq_enabled;
            taito_tc0190.irq_pending.set(saved.irq_pending);
            taito_tc0190.irq_delay = saved.irq_delay;
            self.prg_bank = saved.prg_banks[0];
            self.chr_bank = saved.chr_banks[0];
        }

        if let (Some(ref mut taito_x1005), Some(saved)) =
            (self.taito_x1005.as_mut(), state.taito_x1005.as_ref())
        {
            taito_x1005.prg_banks = saved.prg_banks;
            taito_x1005.chr_banks = saved.chr_banks;
            taito_x1005.ram_enabled = saved.ram_enabled;
            self.prg_bank = saved.prg_banks[0];
            self.chr_bank = saved.chr_banks[0];
            if self.mapper == 207 {
                let top = (saved.chr_banks[0] >> 7) & 1;
                let bottom = (saved.chr_banks[1] >> 7) & 1;
                self.mirroring = match (top, bottom) {
                    (0, 0) => Mirroring::OneScreenLower,
                    (1, 1) => Mirroring::OneScreenUpper,
                    (0, 1) => Mirroring::Horizontal,
                    (1, 0) => Mirroring::HorizontalSwapped,
                    _ => Mirroring::Horizontal,
                };
            }
        }

        if let (Some(ref mut taito_x1017), Some(saved)) =
            (self.taito_x1017.as_mut(), state.taito_x1017.as_ref())
        {
            taito_x1017.prg_banks = saved.prg_banks;
            taito_x1017.chr_banks = saved.chr_banks;
            taito_x1017.ram_enabled = saved.ram_enabled;
            taito_x1017.chr_invert = saved.chr_invert;
            self.prg_bank = saved.prg_banks[0];
            self.chr_bank = saved.chr_banks[0];
        }
    }
}
