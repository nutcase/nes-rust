//! APU wrapper using the external `snes-apu` crate (SPC700 + DSP).
//! 現状: 精度よりも動作優先の簡易統合。セーブステートはダミー。
use snes_apu::apu::Apu as SpcApu;

// Clock ratio used to convert S-CPU cycles (3.579545MHz NTSC) to `snes-apu` internal cycles.
//
// `snes-apu` uses a 2.048MHz internal tick rate (32kHz * 64 cycles/sample), which corresponds to
// the SNES APU oscillator (24.576MHz / 12).
//
// ratio = 2_048_000 / 3_579_545.333... ≈ 0.5721397019
const DEFAULT_APU_CYCLE_SCALE: f64 = 0.572_139_701_913_725_3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BootState {
    /// 初期シグネチャ (AA/BB) をCPUに見せる段階
    ReadySignature,
    /// CPUからのキック(0xCC)を待ち、転送中
    Uploading,
    /// APUプログラム稼働中。以降は実ポート値をそのまま返す
    Running,
}

pub struct Apu {
    pub(crate) inner: Box<SpcApu>,
    sample_rate: u32,
    // Fractional SPC700 cycles accumulator (scaled from S-CPU cycles).
    cycle_accum: f64,
    // Total SPC700 cycles executed (debug/diagnostics).
    pub(crate) total_smp_cycles: u64,
    // Debug: last observed values for $F1/$FA change tracing.
    trace_last_f1: u8,
    trace_last_fa: u8,
    // Debug: last observed APU->CPU port values (for change tracing).
    trace_last_out_ports: [u8; 4],
    // CPU<-APU方向のホストポート（CPUが読み取る値）
    apu_to_cpu_ports: [u8; 4],
    // CPU->APU方向の直近値（SMP側が$F4-$F7で読む値）
    pub(crate) port_latch: [u8; 4],
    boot_state: BootState,
    boot_hle_enabled: bool,
    fast_upload: bool,
    fast_upload_bytes: u64,
    zero_write_seen: bool,
    last_port1: u8,
    upload_addr: u16,
    expected_index: u8,
    block_active: bool,
    pending_idx: Option<u8>,
    data_ready: bool,
    upload_done_count: u64,
    upload_bytes: u64,
    last_upload_idx: u8,
    end_zero_streak: u8,
    // Last value written to port0 during boot; echoed until next write.
    boot_port0_echo: u8,
    // Optional hard override for port0 echo (debug)
    force_port0: Option<u8>,
    // Whether skip-boot was requested (for consistent init)
    skip_boot: bool,
    // Debug: echo last CPU-written port values even after boot (SMW workaround)
    smw_apu_echo: bool,
    // SMW用: HLEハンドシェイクを継続して走らせるか
    smw_apu_hle_handshake: bool,
    smw_hle_end_zero_streak: u8,
    // SMW用: ポートreadは常に直近CPU書き込み(latch)を返す強制モード
    smw_apu_port_echo_strict: bool,
    // Debug/HLE: skip actual SPC upload and jump to running state
    fake_upload: bool,
}

unsafe impl Send for Apu {}
unsafe impl Sync for Apu {}

impl Apu {
    pub fn new() -> Self {
        let inner = SpcApu::new(); // comes with default IPL
        let boot_hle_enabled = std::env::var("APU_BOOT_HLE")
            .map(|v| v != "0" && v.to_lowercase() != "false")
            .unwrap_or(false); // デフォルト: HLE無効（実IPLで正確さ優先）
                               // 正確さ優先: デフォルトではフルサイズ転送を行う。
                               // 速さが欲しい場合のみ APU_FAST_UPLOAD=1 を明示する。
        let fast_upload = std::env::var("APU_FAST_UPLOAD")
            .map(|v| v != "0" && v.to_lowercase() != "false")
            .unwrap_or(false);
        let fast_upload_bytes = std::env::var("APU_FAST_BYTES")
            .ok()
            .and_then(|v| {
                u64::from_str_radix(v.trim_start_matches("0x"), 16)
                    .ok()
                    .or_else(|| v.parse().ok())
            })
            .unwrap_or(0x10000);
        let skip_boot = std::env::var("APU_SKIP_BOOT")
            .map(|v| v != "0" && v.to_lowercase() != "false")
            .unwrap_or(false); // trueなら即実行開始（デバッグ用）
        let fake_upload = std::env::var("APU_FAKE_UPLOAD")
            .map(|v| v != "0" && v.to_lowercase() != "false")
            .unwrap_or(false); // CPUが書き始めたら即実行状態にする簡易HLE
        let force_port0 = std::env::var("APU_FORCE_PORT0").ok().and_then(|v| {
            u8::from_str_radix(v.trim_start_matches("0x"), 16)
                .ok()
                .or_else(|| v.parse().ok())
        });
        let smw_apu_echo = std::env::var("SMW_APU_ECHO")
            .map(|v| v != "0" && v.to_lowercase() != "false")
            .unwrap_or(false);
        let smw_apu_hle_handshake = std::env::var("SMW_APU_HLE_HANDSHAKE")
            .map(|v| v != "0" && v.to_lowercase() != "false")
            .unwrap_or(false); // SMW専用。既定では無効（他ROMへの副作用回避）
        let smw_apu_port_echo_strict = std::env::var("SMW_APU_PORT_ECHO_STRICT")
            .map(|v| v != "0" && v.to_lowercase() != "false")
            .unwrap_or(false);
        let mut apu = Self {
            inner,
            sample_rate: 32000,
            cycle_accum: 0.0,
            total_smp_cycles: 0,
            trace_last_f1: 0,
            trace_last_fa: 0,
            trace_last_out_ports: [0; 4],
            apu_to_cpu_ports: [0; 4],
            port_latch: [0; 4],
            boot_state: if skip_boot || !boot_hle_enabled {
                BootState::Running
            } else {
                BootState::ReadySignature
            }, // AA/BBを必ず経由
            boot_hle_enabled,
            fast_upload,
            fast_upload_bytes,
            zero_write_seen: false,
            last_port1: 0,
            upload_addr: 0x0200,
            expected_index: 0,
            block_active: false,
            pending_idx: None,
            data_ready: false,
            upload_done_count: 0,
            upload_bytes: 0,
            last_upload_idx: 0,
            end_zero_streak: 0,
            boot_port0_echo: 0xAA,
            force_port0,
            skip_boot,
            smw_apu_echo,
            smw_apu_hle_handshake,
            smw_hle_end_zero_streak: 0,
            smw_apu_port_echo_strict,
            fake_upload,
        };
        apu.init_boot_ports();
        apu.trace_last_f1 = apu.inner.read_u8(0x00F1);
        apu.trace_last_fa = apu.inner.read_u8(0x00FA);
        for p in 0..4 {
            apu.trace_last_out_ports[p] = apu.inner.cpu_read_port(p as u8);
        }
        if std::env::var_os("TRACE_APU_BOOTSTATE").is_some() {
            println!(
                "[APU-BOOTSTATE] init: boot_hle_enabled={} skip_boot={} fast_upload={} boot_state={:?}",
                boot_hle_enabled, skip_boot, fast_upload, apu.boot_state
            );
        }
        // skip_bootでも AA/BB を見せてから CC エコーを開始する
        if skip_boot {
            apu.apu_to_cpu_ports = [0xAA, 0xBB, 0x00, 0x00];
            apu.boot_port0_echo = 0xAA;
            apu.finish_upload_and_start_with_ack(0xCC);
            apu.boot_port0_echo = 0xCC;
            apu.apu_to_cpu_ports[0] = 0xCC;
            apu.apu_to_cpu_ports[1] = 0xBB;
        }
        apu
    }

    pub fn reset(&mut self) {
        self.inner.reset();
        self.fast_upload = std::env::var("APU_FAST_UPLOAD")
            .map(|v| v != "0" && v.to_lowercase() != "false")
            .unwrap_or(false);
        self.fast_upload_bytes = std::env::var("APU_FAST_BYTES")
            .ok()
            .and_then(|v| {
                u64::from_str_radix(v.trim_start_matches("0x"), 16)
                    .ok()
                    .or_else(|| v.parse().ok())
            })
            .unwrap_or(0x10000);
        self.skip_boot = std::env::var("APU_SKIP_BOOT")
            .map(|v| v != "0" && v.to_lowercase() != "false")
            .unwrap_or(false);
        self.smw_apu_echo = std::env::var("SMW_APU_ECHO")
            .map(|v| v != "0" && v.to_lowercase() != "false")
            .unwrap_or(self.smw_apu_echo);
        self.smw_apu_hle_handshake = std::env::var("SMW_APU_HLE_HANDSHAKE")
            .map(|v| v != "0" && v.to_lowercase() != "false")
            .unwrap_or(false);
        self.smw_apu_port_echo_strict = std::env::var("SMW_APU_PORT_ECHO_STRICT")
            .map(|v| v != "0" && v.to_lowercase() != "false")
            .unwrap_or(self.smw_apu_port_echo_strict);
        self.smw_hle_end_zero_streak = 0;
        self.boot_state = if self.skip_boot || !self.boot_hle_enabled {
            BootState::Running
        } else {
            BootState::ReadySignature
        };
        self.zero_write_seen = false;
        self.last_port1 = 0;
        self.cycle_accum = 0.0;
        self.total_smp_cycles = 0;
        self.trace_last_f1 = self.inner.read_u8(0x00F1);
        self.trace_last_fa = self.inner.read_u8(0x00FA);
        for p in 0..4 {
            self.trace_last_out_ports[p] = self.inner.cpu_read_port(p as u8);
        }
        self.port_latch = [0; 4];
        self.upload_addr = 0x0200;
        self.expected_index = 0;
        self.block_active = false;
        self.pending_idx = None;
        self.data_ready = false;
        self.upload_done_count = 0;
        self.upload_bytes = 0;
        self.last_upload_idx = 0;
        self.end_zero_streak = 0;
        self.boot_port0_echo = 0xAA;
        self.init_boot_ports();
        if self.skip_boot {
            self.apu_to_cpu_ports = [0xAA, 0xBB, 0x00, 0x00];
            self.boot_port0_echo = 0xAA;
            self.finish_upload_and_start_with_ack(0xCC);
            self.boot_port0_echo = 0xCC;
            self.apu_to_cpu_ports[0] = 0xCC;
            self.apu_to_cpu_ports[1] = 0xBB;
        }
    }

    fn init_boot_ports(&mut self) {
        // Even when we skip the real IPL, seed ports with AA/BB so S-CPU handshake loops pass.
        if self.boot_state == BootState::ReadySignature || self.fast_upload {
            self.apu_to_cpu_ports = [0xAA, 0xBB, 0x00, 0x00];
            // CPU側から読む値（APUIO）は APU->CPU ラッチ。実機ではIPLが書くが、HLE時は先に用意する。
            self.inner.write_u8(0x00F4, 0xAA);
            self.inner.write_u8(0x00F5, 0xBB);
            self.inner.write_u8(0x00F6, 0x00);
            self.inner.write_u8(0x00F7, 0x00);
            // CPU->APU ラッチ（SMPが読む側）は既定で 0。
            self.port_latch = [0; 4];
            self.boot_port0_echo = 0xAA;
        } else {
            self.apu_to_cpu_ports = [0; 4];
        }
    }

    /// CPUサイクルに合わせてSPC700を回す。
    /// 仮想周波数: S-CPU 3.58MHz / SPC700 1.024MHz ⇒ およそ 1 : 3.5 で遅らせる。
    pub fn step(&mut self, cpu_cycles: u8) {
        // 比率調整。必要に応じて環境変数 APU_CYCLE_SCALE で上書き可。
        // `snes-apu` crate の内部サイクルは 2.048MHz 基準で、DSPサンプル(32kHz)は 64cycle ごとに生成される。
        // したがって S-CPU(3.579545MHz NTSC) からの比率は 2.048MHz / 3.579545MHz ≈ 0.57214。
        let scale: f64 = std::env::var("APU_CYCLE_SCALE")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(DEFAULT_APU_CYCLE_SCALE);

        self.cycle_accum += (cpu_cycles as f64) * scale;
        let run = self.cycle_accum.floor() as i32;
        self.cycle_accum -= run as f64;

        if run > 0 {
            self.total_smp_cycles = self.total_smp_cycles.saturating_add(run as u64);
            if let Some(smp) = self.inner.smp.as_mut() {
                smp.run(run);
            }
            if let Some(dsp) = self.inner.dsp.as_mut() {
                dsp.flush();
            }
        }

        // ハンドシェイク終了後は実ポート値を表側にも反映
        if self.boot_state == BootState::Running {
            for p in 0..4 {
                self.apu_to_cpu_ports[p] = self.inner.cpu_read_port(p as u8);
            }
        }

        self.maybe_trace_apu_control();
        self.maybe_trace_out_ports();
    }

    /// Master clock に合わせてSPC700を回す（S-CPU が停止している期間の進行用）。
    ///
    /// MDMAなどでS-CPUが止まっていても、実機ではAPUは独立して動作し続けるため、
    /// エミュレータ側でも「経過時間」ぶんだけSPC700/DSPを進める必要がある。
    pub fn step_master_cycles(&mut self, master_cycles: u64) {
        if master_cycles == 0 {
            return;
        }

        // APU_CYCLE_SCALE is defined in terms of "S-CPU cycles" (as used by `step()`).
        // Convert master cycles -> S-CPU cycles using our fixed divider (master/6).
        let scale_cpu: f64 = std::env::var("APU_CYCLE_SCALE")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(DEFAULT_APU_CYCLE_SCALE);
        let scale_master = scale_cpu / 6.0;

        self.cycle_accum += (master_cycles as f64) * scale_master;
        let run = self.cycle_accum.floor() as i32;
        self.cycle_accum -= run as f64;

        if run > 0 {
            self.total_smp_cycles = self.total_smp_cycles.saturating_add(run as u64);
            if let Some(smp) = self.inner.smp.as_mut() {
                smp.run(run);
            }
            if let Some(dsp) = self.inner.dsp.as_mut() {
                dsp.flush();
            }
        }

        if self.boot_state == BootState::Running {
            for p in 0..4 {
                self.apu_to_cpu_ports[p] = self.inner.cpu_read_port(p as u8);
            }
        }

        self.maybe_trace_apu_control();
        self.maybe_trace_out_ports();
    }

    fn maybe_trace_apu_control(&mut self) {
        if std::env::var_os("TRACE_BURNIN_APU_F1").is_none() {
            return;
        }
        let f1 = self.inner.read_u8(0x00F1);
        let fa = self.inner.read_u8(0x00FA);
        if f1 == self.trace_last_f1 && fa == self.trace_last_fa {
            return;
        }
        let smp_pc = self.inner.smp.as_ref().map(|s| s.reg_pc).unwrap_or(0);
        println!(
            "[APU-F1] apu_cycles={} smp_pc={:04X} $F1 {:02X}->{:02X} $FA {:02X}->{:02X}",
            self.total_smp_cycles, smp_pc, self.trace_last_f1, f1, self.trace_last_fa, fa
        );
        self.trace_last_f1 = f1;
        self.trace_last_fa = fa;
    }

    fn maybe_trace_out_ports(&mut self) {
        if std::env::var_os("TRACE_BURNIN_APU_PORT1").is_none() {
            return;
        }
        let cur = self.inner.cpu_read_port(1);
        if cur == self.trace_last_out_ports[1] {
            return;
        }
        use std::sync::atomic::{AtomicU32, Ordering};
        static CNT: AtomicU32 = AtomicU32::new(0);
        let n = CNT.fetch_add(1, Ordering::Relaxed);
        if n < 256 {
            let smp_pc = self.inner.smp.as_ref().map(|s| s.reg_pc).unwrap_or(0);
            let mut code = [0u8; 8];
            for (i, b) in code.iter_mut().enumerate() {
                *b = self.inner.read_u8(smp_pc.wrapping_add(i as u16) as u32);
            }
            println!(
                "[APU-PORT1] apu_cycles={} smp_pc={:04X} {:02X}->{:02X} code={:02X?}",
                self.total_smp_cycles, smp_pc, self.trace_last_out_ports[1], cur, code
            );
        }
        self.trace_last_out_ports[1] = cur;
    }

    /// CPU側ポート読み出し ($2140-$2143)
    pub fn read_port(&mut self, port: u8) -> u8 {
        let p = (port & 0x03) as usize;

        // 強制値（デバッグ/HLE）指定時は即返す
        if let Some(forced) = if p == 0 {
            crate::debug_flags::apu_force_port0()
        } else if p == 1 {
            crate::debug_flags::apu_force_port1()
        } else {
            None
        } {
            return forced;
        }

        match self.boot_state {
            BootState::Running => {
                // 実ハード同様、SMP側が書いた値をそのまま返す
                let v = if self.smw_apu_port_echo_strict {
                    // HLE完了後は実ポート値を返す。完了前はラッチ値でエコー。
                    if self.upload_done_count > 0 {
                        self.inner.cpu_read_port(p as u8)
                    } else {
                        self.apu_to_cpu_ports[p]
                    }
                } else if self.smw_apu_echo {
                    self.apu_to_cpu_ports[p]
                } else if self.smw_apu_hle_handshake {
                    self.apu_to_cpu_ports[p]
                } else {
                    self.inner.cpu_read_port(p as u8)
                };
                if !self.smw_apu_echo {
                    self.apu_to_cpu_ports[p] = v;
                }
                if std::env::var_os("TRACE_APU_PORT_ONCE").is_some()
                    || crate::debug_flags::trace_apu_port_all()
                    || (p == 0 && crate::debug_flags::trace_apu_port0())
                {
                    use std::sync::atomic::{AtomicU32, Ordering};
                    static CNT: AtomicU32 = AtomicU32::new(0);
                    let n = CNT.fetch_add(1, Ordering::Relaxed);
                    if n < 32 {
                        println!("[APU-R] port{} -> {:02X} (boot=Running)", p, v);
                    }
                }
                v
            }
            // ブート中: port0は「最後にCPUが書いた値」を保持して返す。port1-3も表キャッシュを返す。
            _ => {
                let v = if let Some(force) = self.force_port0 {
                    if p == 0 {
                        force
                    } else {
                        self.apu_to_cpu_ports[p]
                    }
                } else if p == 0 {
                    self.boot_port0_echo
                } else {
                    self.apu_to_cpu_ports[p]
                };
                if std::env::var_os("TRACE_APU_PORT_ONCE").is_some()
                    || crate::debug_flags::trace_apu_port_all()
                    || (p == 0 && crate::debug_flags::trace_apu_port0())
                {
                    use std::sync::atomic::{AtomicU32, Ordering};
                    static CNT: AtomicU32 = AtomicU32::new(0);
                    let n = CNT.fetch_add(1, Ordering::Relaxed);
                    if n < 32 {
                        println!(
                            "[APU-R] port{} -> {:02X} (boot={:?})",
                            p, v, self.boot_state
                        );
                    }
                }
                v
            }
        }
    }

    /// CPU側ポート書き込み ($2140-$2143)
    pub fn write_port(&mut self, port: u8, value: u8) {
        let p = (port & 0x03) as usize;
        self.port_latch[p] = value;
        // CPU->APU ラッチへ反映（SMP側が $F4-$F7 で読む）
        self.inner.cpu_write_port(p as u8, value);

        // 簡易HLE: 転送を省略して即稼働させる
        if self.fake_upload && self.boot_state != BootState::Running {
            self.finish_upload_and_start_with_ack(0);
        }

        if crate::debug_flags::trace_apu_port_all()
            || (p == 0 && crate::debug_flags::trace_apu_port0())
        {
            use std::sync::atomic::{AtomicU32, Ordering};
            static CNT: AtomicU32 = AtomicU32::new(0);
            let n = CNT.fetch_add(1, Ordering::Relaxed);
            if n < 512 {
                println!(
                    "[APU-W] port{} <- {:02X} state={:?} echo0={:02X} to_cpu=[{:02X} {:02X} {:02X} {:02X}]",
                    p,
                    value,
                    self.boot_state,
                    self.boot_port0_echo,
                    self.apu_to_cpu_ports[0],
                    self.apu_to_cpu_ports[1],
                    self.apu_to_cpu_ports[2],
                    self.apu_to_cpu_ports[3]
                );
            }
        }

        if !self.boot_hle_enabled {
            return;
        }

        match self.boot_state {
            BootState::ReadySignature => {
                // CPUが0xCCを書いたらIPL転送開始。CC以外の値で署名を潰さない
                // （SMC等の初期化で $2140 を0クリアするため）。
                if p == 0 {
                    if value != 0xCC {
                        // 署名は維持したまま無視
                        return;
                    }
                    // HLEでもアップロード状態に入り、CPUのインデックスエコーを行う
                    self.boot_state = BootState::Uploading;
                    if std::env::var_os("TRACE_APU_BOOTSTATE").is_some() {
                        println!("[APU-BOOTSTATE] -> Uploading (kick=0xCC)");
                    }
                    self.apu_to_cpu_ports[0] = 0xCC;
                    self.boot_port0_echo = 0xCC;
                    self.expected_index = 0;
                    self.block_active = false;
                    self.zero_write_seen = false;
                    self.pending_idx = None;
                    self.data_ready = false;
                    self.upload_bytes = 0;
                    self.last_upload_idx = 0;
                    // fast_upload は Uploading 中の閾値判定で早期完了する
                }
                if p == 2 {
                    self.upload_addr = (self.upload_addr & 0xFF00) | value as u16;
                } else if p == 3 {
                    self.upload_addr = (self.upload_addr & 0x00FF) | ((value as u16) << 8);
                } else if p == 1 {
                    self.last_port1 = value;
                }
            }
            BootState::Uploading => {
                // 転送先アドレス（毎ブロックごとに書き替えられる）
                if p == 2 {
                    self.upload_addr = (self.upload_addr & 0xFF00) | value as u16;
                    return;
                } else if p == 3 {
                    self.upload_addr = (self.upload_addr & 0x00FF) | ((value as u16) << 8);
                    return;
                }

                // port0/port1 の書き込み順は ROM により異なる（8bit書き込み / 16bit書き込み）。
                // 実機IPLは port0 の変化をトリガに port1 を読み取るため、ここでは
                // 「port0(idx) と port1(data) の両方が揃ったタイミング」で1バイトを確定する。
                match p {
                    0 => {
                        let idx = value;
                        // Always echo APUIO0 back (handshake ACK)
                        self.apu_to_cpu_ports[0] = idx;
                        self.boot_port0_echo = idx;

                        // SPC700 IPL protocol:
                        // - Data byte: APUIO0 must equal expected_index (starts at 0 for each block)
                        // - Command: APUIO0 != expected_index; APUIO1==0 means "start program at APUIO2/3",
                        //   otherwise it means "set new base address (APUIO2/3) and continue upload".
                        if idx == self.expected_index {
                            self.pending_idx = Some(idx);
                            if self.data_ready {
                                // port1 が先に来たケース: ここで確定
                                let data = self.last_port1;
                                self.data_ready = false;
                                self.pending_idx = None;

                                let addr = self.upload_addr.wrapping_add(idx as u16);
                                self.inner.write_u8(addr as u32, data);
                                self.upload_bytes = self.upload_bytes.saturating_add(1);
                                self.last_upload_idx = idx;
                                self.expected_index = self.expected_index.wrapping_add(1);
                            }
                        } else {
                            // Command / state sync
                            self.pending_idx = None;
                            self.data_ready = false;
                            self.expected_index = 0;
                            if self.last_port1 == 0 {
                                // Start program; ACK must echo the command value the CPU wrote.
                                self.finish_upload_and_start_with_ack(idx);
                                return;
                            }
                        }
                        return;
                    }
                    1 => {
                        self.last_port1 = value;
                        self.apu_to_cpu_ports[1] = value;
                        self.data_ready = true;
                        if let Some(idx) = self.pending_idx {
                            // port0 が先に来たケース: ここで確定
                            if idx == self.expected_index {
                                self.data_ready = false;
                                self.pending_idx = None;
                                let addr = self.upload_addr.wrapping_add(idx as u16);
                                self.inner.write_u8(addr as u32, value);
                                self.upload_bytes = self.upload_bytes.saturating_add(1);
                                self.last_upload_idx = idx;
                                self.expected_index = self.expected_index.wrapping_add(1);
                            }
                        }
                        return;
                    }
                    _ => {
                        self.apu_to_cpu_ports[p] = value;
                        return;
                    }
                }
            }
            BootState::Running => {
                // 稼働後は表側キャッシュだけ更新（HLE継続時もキャッシュを維持）
                self.apu_to_cpu_ports[p] = value;

                // SMW HLE 継続モード: 0,0 が2回続いたら即 start (upload_done) とみなす
                if self.smw_apu_hle_handshake && p == 0 {
                    if value == 0 {
                        self.smw_hle_end_zero_streak =
                            self.smw_hle_end_zero_streak.saturating_add(1);
                    } else {
                        self.smw_hle_end_zero_streak = 0;
                    }
                    if self.smw_hle_end_zero_streak >= 2 {
                        if std::env::var_os("TRACE_APU_BOOTSTATE").is_some() {
                            println!("[APU-BOOTSTATE] SMW force start (running echo)");
                        }
                        if std::env::var_os("TRACE_APU_BOOT").is_some() {
                            println!(
                                "[APU-HLE] Forced start after port0=0 twice (running-phase echo)"
                            );
                        }
                        self.finish_upload_and_start_with_ack(0);
                        self.smw_hle_end_zero_streak = 0;
                    }
                }
            }
        }
    }

    /// オーディオサンプル生成（ステレオ）
    pub fn generate_audio_samples(&mut self, samples: &mut [(i16, i16)]) {
        let need = samples.len() as i32;
        if need <= 0 {
            return;
        }

        // `step()` 側でSPC700/DSPを進めて output_buffer に溜める。
        // ここでは output_buffer から読むだけにして、二重にSMPを回さない。
        let Some(dsp) = self.inner.dsp.as_mut() else {
            for s in samples.iter_mut() {
                *s = (0, 0);
            }
            return;
        };

        dsp.flush();

        let avail = dsp.output_buffer.get_sample_count().max(0);
        let to_read = need.min(avail);

        if to_read > 0 {
            let mut left = vec![0i16; to_read as usize];
            let mut right = vec![0i16; to_read as usize];
            dsp.output_buffer.read(&mut left, &mut right, to_read);
            for i in 0..(to_read as usize) {
                samples[i] = (left[i], right[i]);
            }
        }

        // 足りない分は無音で埋める（リングバッファのアンダーラン対策）
        for s in samples.iter_mut().skip(to_read as usize) {
            *s = (0, 0);
        }
    }

    pub fn get_sample_rate(&self) -> u32 {
        self.sample_rate
    }

    // --- セーブステート（ダミー実装） ---
    pub fn to_save_state(&self) -> crate::savestate::ApuSaveState {
        crate::savestate::ApuSaveState::default()
    }

    pub fn load_from_save_state(&mut self, _st: &crate::savestate::ApuSaveState) {
        self.reset();
    }

    // 旧ハンドシェイクAPI互換ダミー
    pub fn set_handshake_enabled(&mut self, _enabled: bool) {}
    pub fn handshake_state_str(&self) -> &'static str {
        match self.boot_state {
            BootState::ReadySignature => "ipl-signature",
            BootState::Uploading => "ipl-upload",
            BootState::Running => "spc700",
        }
    }

    /// デバッグ/HLE用途: 任意のバイナリをARAmにロードして即実行開始する。
    /// ポートの初期値は 0x00 に揃え、boot_state を Running に移行する。
    pub fn load_and_start(&mut self, data: &[u8], base: u16, start_pc: u16) {
        // 書き込み先は base から。I/Oレジスタ(0xF0-0xFF)は避ける。
        let mut offset = base as usize;
        for &b in data.iter() {
            if offset >= 0x10000 {
                break;
            }
            if (offset & 0xFF00) == 0x00F0 {
                // スキップして次のページへ
                offset = (offset & 0xFF00) + 0x0100;
            }
            if offset >= 0x10000 {
                break;
            }
            self.inner.write_u8(offset as u32, b);
            offset += 1;
        }
        if let Some(smp) = self.inner.smp.as_mut() {
            smp.reg_pc = start_pc;
        }
        // IPL を無効化
        self.inner.write_u8(0x00F1, 0x00);
        // ポート初期値はAA/BB署名を維持してCPU側ハンドシェイクを満たす
        let init_ports = [0xAA, 0xBB, 0x00, 0x00];
        for p in 0..4 {
            let v = init_ports[p];
            self.inner.write_u8(0x00F4 + p as u32, v);
            self.apu_to_cpu_ports[p] = v;
        }
        self.port_latch = [0; 4];
        self.boot_port0_echo = 0xAA;
        self.boot_state = BootState::Running;
        if std::env::var_os("TRACE_APU_BOOTSTATE").is_some() {
            println!(
                "[APU-BOOTSTATE] load_and_start -> Running (base=${:04X} start_pc=${:04X} len={})",
                base,
                start_pc,
                data.len()
            );
        }
    }

    /// 転送完了後にSPCプログラムを実行状態へ進める。
    fn finish_upload_and_start(&mut self) {
        // 実機IPL同様、完了時のACKは 0 を返す
        self.finish_upload_and_start_with_ack(0);
    }

    fn finish_upload_and_start_with_ack(&mut self, ack: u8) {
        self.boot_state = BootState::Running;
        if std::env::var_os("TRACE_APU_BOOTSTATE").is_some() {
            println!(
                "[APU-BOOTSTATE] finish_upload_and_start ack={:02X} addr=${:04X}",
                ack, self.upload_addr
            );
        }
        self.block_active = false;
        self.data_ready = false;
        self.upload_done_count += 1;
        if std::env::var_os("TRACE_APU_PORT").is_some()
            || std::env::var_os("TRACE_APU_BOOT").is_some()
            || crate::debug_flags::trace_apu_port_all()
        {
            println!(
                "[APU-BOOT] upload complete count={} start_pc=${:04X} addr_base=${:04X}",
                self.upload_done_count, self.upload_addr, self.upload_addr
            );
        }
        // IPL ROM を無効化
        self.inner.write_u8(0x00F1, 0x00);
        // ジャンプ先をセット（IPLがジャンプする直前の初期レジスタ状態に寄せる）
        if let Some(smp) = self.inner.smp.as_mut() {
            let pc = if self.upload_addr == 0 {
                0x0200
            } else {
                self.upload_addr
            };
            // IPL直後の基本状態（Smp::reset 相当）。
            // これを揃えないと、HLEで中途半端なIPL実行状態のままジャンプしてSPC側が暴走しやすい。
            smp.reg_a = 0;
            smp.reg_x = 0;
            smp.reg_y = 0;
            smp.reg_sp = 0xEF;
            smp.set_psw(0x02);
            smp.reg_pc = pc;
        }
        // 実行開始をCPUへ知らせるためポート0にACK値を置く（既定=0）
        self.inner.write_u8(0x00F4, ack);
        self.apu_to_cpu_ports[0] = ack;
        // 初期ACKはそのままにして、以後は実値を返す
        for i in 0..4 {
            self.apu_to_cpu_ports[i] = self.inner.cpu_read_port(i as u8);
        }
    }
}
