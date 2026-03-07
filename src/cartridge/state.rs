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
    pub irq_latch: u8,
    pub irq_counter: u8,
    pub irq_reload: bool,
    pub irq_enabled: bool,
    pub irq_pending: bool,
    pub prg_ram_enabled: bool,
    pub prg_ram_write_protect: bool,
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
pub struct Mapper225State {
    pub nrom128: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mapper232State {
    pub outer_bank: u8,
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
pub struct Mapper212State {
    pub mode_32k: bool,
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
    pub mapper15: Option<Mapper15State>,
    #[serde(default)]
    pub mapper72: Option<Mapper72State>,
    #[serde(default)]
    pub mapper58: Option<Mapper58State>,
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
            irq_latch: m.irq_latch,
            irq_counter: m.irq_counter,
            irq_reload: m.irq_reload,
            irq_enabled: m.irq_enabled,
            irq_pending: m.irq_pending.get(),
            prg_ram_enabled: m.prg_ram_enabled,
            prg_ram_write_protect: m.prg_ram_write_protect,
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
        let mapper212 = if self.mapper == 212 {
            Some(Mapper212State {
                mode_32k: self.mapper212_32k_mode,
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
            fme7,
            bandai_fcg,
            mapper34,
            mapper93,
            mapper184,
            vrc1,
            mapper15,
            mapper72,
            mapper58,
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
            mapper236,
            mapper227,
            mapper246,
            sunsoft4,
            taito_tc0190,
            taito_x1005,
            taito_x1017,
            mapper233,
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
            mmc3.irq_latch = saved.irq_latch;
            mmc3.irq_counter = saved.irq_counter;
            mmc3.irq_reload = saved.irq_reload;
            mmc3.irq_enabled = saved.irq_enabled;
            mmc3.irq_pending.set(saved.irq_pending);
            mmc3.prg_ram_enabled = saved.prg_ram_enabled;
            mmc3.prg_ram_write_protect = saved.prg_ram_write_protect;
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
        }

        if let (Some(ref mut vrc1), Some(saved)) = (self.vrc1.as_mut(), state.vrc1.as_ref()) {
            vrc1.prg_banks = saved.prg_banks;
            vrc1.chr_bank_0 = saved.chr_bank_0;
            vrc1.chr_bank_1 = saved.chr_bank_1;
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
        if let Some(saved) = state.mapper225.as_ref() {
            self.mapper225_nrom128 = saved.nrom128;
        }
        if let Some(saved) = state.mapper232.as_ref() {
            self.mapper232_outer_bank = saved.outer_bank;
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
        if let Some(saved) = state.mapper212.as_ref() {
            self.mapper212_32k_mode = saved.mode_32k;
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

        if let (Some(ref mut taito_tc0190), Some(saved)) =
            (self.taito_tc0190.as_mut(), state.taito_tc0190.as_ref())
        {
            taito_tc0190.prg_banks = saved.prg_banks;
            taito_tc0190.chr_banks = saved.chr_banks;
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
