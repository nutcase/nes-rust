pub mod cheat_search;
pub mod gl_game;
pub mod hex_viewer;

use cheat_search::CheatSearchUi;
use hex_viewer::HexViewerState;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ActiveTab {
    HexViewer,
    CheatSearch,
}

pub struct CheatToolUi {
    pub active_tab: ActiveTab,
    pub hex_viewer: HexViewerState,
    pub cheat_search_ui: CheatSearchUi,
    pub panel_visible: bool,
    /// Frozen snapshot shown in the panel. Updated only on Refresh.
    pub ram_snapshot: Vec<u8>,
    /// Set to true when Refresh is clicked; consumed by the main loop.
    pub refresh_requested: bool,
    /// When true, emulation is paused (no ticks).
    pub paused: bool,
    /// Reusable combined RAM buffer (cpu_ram ++ prg_ram) to avoid per-frame allocation.
    pub combined_ram: Vec<u8>,
}

impl CheatToolUi {
    pub fn new() -> Self {
        Self {
            active_tab: ActiveTab::HexViewer,
            hex_viewer: HexViewerState::new(),
            cheat_search_ui: CheatSearchUi::new(),
            panel_visible: false,
            ram_snapshot: vec![0u8; 0x800],
            refresh_requested: false,
            paused: false,
            combined_ram: Vec::new(),
        }
    }

    /// Copy live RAM into the display snapshot.
    pub fn refresh(&mut self, ram: &[u8]) {
        let prev = self.ram_snapshot.clone();
        if ram.len() != self.ram_snapshot.len() {
            self.ram_snapshot.resize(ram.len(), 0);
        }
        let len = ram.len().min(self.ram_snapshot.len());
        self.ram_snapshot[..len].copy_from_slice(&ram[..len]);
        self.hex_viewer.update_prev(&prev);
    }

    /// Update the reusable combined RAM buffer with current cpu_ram ++ prg_ram.
    pub fn update_combined_ram(&mut self, cpu_ram: &[u8], prg_ram: Option<&[u8]>) {
        self.combined_ram.clear();
        self.combined_ram.extend_from_slice(cpu_ram);
        if let Some(sram) = prg_ram {
            self.combined_ram.extend_from_slice(sram);
        }
    }

    pub fn show_panel(
        &mut self,
        ui: &mut egui::Ui,
        ram_writes: &mut Vec<(usize, u8)>,
        cheat_path: Option<&std::path::Path>,
    ) {
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.active_tab, ActiveTab::HexViewer, "Hex Viewer");
            ui.selectable_value(&mut self.active_tab, ActiveTab::CheatSearch, "Cheat Search");
            ui.separator();
            ui.checkbox(&mut self.paused, "Pause");
        });
        ui.separator();

        match self.active_tab {
            ActiveTab::HexViewer => {
                ui.horizontal(|ui| {
                    if ui.button("Refresh").clicked() {
                        self.refresh_requested = true;
                    }
                    ui.label("(Update hex display from live RAM)");
                });
                ui.separator();
                let snap = &self.ram_snapshot;
                self.hex_viewer.show(ui, snap, ram_writes);
            }
            ActiveTab::CheatSearch => {
                let ram = &self.combined_ram;
                self.cheat_search_ui.show(ui, ram, cheat_path);
            }
        }
    }
}
