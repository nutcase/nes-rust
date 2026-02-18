use egui::{self, Color32, FontId};
use egui::text::LayoutJob;

const BYTES_PER_ROW: usize = 16;
const COLOR_ADDR: Color32 = Color32::from_rgb(0x88, 0x88, 0x88);
const COLOR_NORMAL: Color32 = Color32::from_rgb(0xCC, 0xCC, 0xCC);
const COLOR_CHANGED: Color32 = Color32::from_rgb(0xFF, 0x44, 0x44);
const COLOR_ASCII: Color32 = Color32::from_rgb(0x88, 0xAA, 0x88);

pub struct HexViewerState {
    prev_ram: Vec<u8>,
    goto_addr: String,
    scroll_to_row: Option<usize>,
    edit_addr: String,
    edit_val: String,
}

impl HexViewerState {
    pub fn new() -> Self {
        Self {
            prev_ram: vec![0u8; 0x800],
            goto_addr: String::new(),
            scroll_to_row: None,
            edit_addr: String::new(),
            edit_val: String::new(),
        }
    }

    pub fn update_prev(&mut self, ram: &[u8]) {
        let len = ram.len().min(self.prev_ram.len());
        self.prev_ram[..len].copy_from_slice(&ram[..len]);
    }

    pub fn show(&mut self, ui: &mut egui::Ui, ram: &[u8], ram_writes: &mut Vec<(usize, u8)>) {
        let total_rows = (ram.len() + BYTES_PER_ROW - 1) / BYTES_PER_ROW;
        let mono = FontId::monospace(12.0);

        // Toolbar: goto + edit
        ui.horizontal(|ui| {
            ui.label("Go to:");
            let goto_resp = ui.add(
                egui::TextEdit::singleline(&mut self.goto_addr).desired_width(50.0),
            );
            if (goto_resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))
                || ui.button("Go").clicked()
            {
                if let Ok(addr) = parse_hex(&self.goto_addr) {
                    self.scroll_to_row = Some(addr / BYTES_PER_ROW);
                }
            }

            ui.separator();
            ui.label("Edit:");
            ui.add(egui::TextEdit::singleline(&mut self.edit_addr).desired_width(40.0));
            ui.label("=");
            let val_resp = ui.add(
                egui::TextEdit::singleline(&mut self.edit_val).desired_width(25.0),
            );
            if (val_resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))
                || ui.button("Set").clicked()
            {
                if let (Ok(addr), Ok(val)) = (
                    parse_hex(&self.edit_addr),
                    u8::from_str_radix(self.edit_val.trim(), 16),
                ) {
                    if addr < ram.len() {
                        ram_writes.push((addr, val));
                    }
                }
            }
        });
        ui.separator();

        // Hex dump with virtual scrolling
        let row_height = 16.0;
        let mut scroll_area = egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .max_height(ui.available_height());

        if let Some(row) = self.scroll_to_row.take() {
            scroll_area = scroll_area.vertical_scroll_offset(row as f32 * row_height);
        }

        scroll_area.show_rows(ui, row_height, total_rows, |ui, row_range| {
            for row_idx in row_range {
                let base_addr = row_idx * BYTES_PER_ROW;
                let mut job = LayoutJob::default();

                // Address column
                append_text(&mut job, &format!("{:04X}: ", base_addr), &mono, COLOR_ADDR);

                // Hex bytes
                for col in 0..BYTES_PER_ROW {
                    let addr = base_addr + col;
                    if addr >= ram.len() {
                        append_text(&mut job, "   ", &mono, COLOR_NORMAL);
                        continue;
                    }
                    let byte_val = ram[addr];
                    let changed = addr < self.prev_ram.len() && byte_val != self.prev_ram[addr];
                    let color = if changed { COLOR_CHANGED } else { COLOR_NORMAL };
                    append_text(&mut job, &format!("{:02X} ", byte_val), &mono, color);
                }

                // ASCII column
                append_text(&mut job, " ", &mono, COLOR_ASCII);
                let end = (base_addr + BYTES_PER_ROW).min(ram.len());
                for addr in base_addr..base_addr + BYTES_PER_ROW {
                    if addr >= ram.len() {
                        append_text(&mut job, " ", &mono, COLOR_ASCII);
                    } else {
                        let b = ram[addr];
                        let ch = if (0x20..=0x7E).contains(&b) { b as char } else { '.' };
                        let mut buf = [0u8; 1];
                        ch.encode_utf8(&mut buf);
                        append_text(
                            &mut job,
                            std::str::from_utf8(&buf[..ch.len_utf8()]).unwrap_or("."),
                            &mono,
                            if addr < end { COLOR_ASCII } else { COLOR_NORMAL },
                        );
                    }
                }

                ui.label(job);
            }
        });
    }
}

fn append_text(job: &mut LayoutJob, text: &str, font: &FontId, color: Color32) {
    job.append(text, 0.0, egui::TextFormat {
        font_id: font.clone(),
        color,
        ..Default::default()
    });
}

fn parse_hex(s: &str) -> Result<usize, std::num::ParseIntError> {
    let s = s.trim().trim_start_matches("0x").trim_start_matches("0X").trim_start_matches('$');
    usize::from_str_radix(s, 16)
}
