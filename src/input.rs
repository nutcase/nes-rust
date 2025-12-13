// SNES Input Controller implementation

use crate::debug_flags;

#[derive(Debug, Clone, Default)]
pub struct SnesController {
    // コントローラーの状態（ビットフィールド）
    buttons: u16,
    // 強制的に押下したことにするボタン（デバッグ/ヘッドレス用）
    auto_buttons: u16,
    // シフトレジスタ（読み取り用）
    shift_register: u16,
    // ラッチされた状態
    latched_buttons: u16,
    // ストローブ状態
    strobe: bool,
}

// SNES コントローラーのボタン定義
#[allow(dead_code)]
pub mod button {
    pub const B: u16 = 0x0001;
    pub const Y: u16 = 0x0002;
    pub const SELECT: u16 = 0x0004;
    pub const START: u16 = 0x0008;
    pub const UP: u16 = 0x0010;
    pub const DOWN: u16 = 0x0020;
    pub const LEFT: u16 = 0x0040;
    pub const RIGHT: u16 = 0x0080;
    pub const A: u16 = 0x0100;
    pub const X: u16 = 0x0200;
    pub const L: u16 = 0x0400;
    pub const R: u16 = 0x0800;
}

impl SnesController {
    pub fn new() -> Self {
        Self {
            buttons: 0,
            auto_buttons: 0,
            shift_register: 0,
            latched_buttons: 0,
            strobe: false,
        }
    }

    /// ハードウェア出力形式（アクティブLOW）に変換したボタン状態を返す
    /// 1 = 未押下, 0 = 押下
    #[inline]
    pub fn active_low_bits(&self) -> u16 {
        // SNESは12ボタン分のみ有効ビット（0..11）。上位4ビットは常に1が返る。
        let combined = self.buttons | self.auto_buttons;
        (!combined) | 0xF000
    }

    // ボタン状態を設定
    #[allow(dead_code)]
    pub fn set_button(&mut self, button: u16, pressed: bool) {
        if pressed {
            self.buttons |= button;
        } else {
            self.buttons &= !button;
        }
    }

    // 複数ボタンの状態を一度に設定
    pub fn set_buttons(&mut self, buttons: u16) {
        self.buttons = buttons;
    }

    pub fn set_auto_buttons(&mut self, mask: u16) {
        self.auto_buttons = mask;
        // strobe High 時には shift_register を即更新するため、再ラッチしておく
        if self.strobe {
            self.latch_buttons();
        }
    }

    // 現在のボタン状態を取得
    pub fn get_buttons(&self) -> u16 {
        self.buttons
    }

    // ストローブ書き込み（$4016）
    pub fn write_strobe(&mut self, value: u8) {
        let new_strobe = value & 0x01 != 0;

        // ストローブがHigh->Lowに変わった時にボタン状態をラッチ
        if self.strobe && !new_strobe {
            self.latch_buttons();
        }

        self.strobe = new_strobe;

        // ストローブがHigh の間はシフトレジスタをリセット
        if self.strobe {
            self.shift_register = self.latched_buttons;
        }
    }

    // データ読み取り（$4016/$4017）
    pub fn read_data(&mut self) -> u8 {
        if self.strobe {
            // ストローブ中は常にAボタンの状態を返す
            // ハードはアクティブLOWなので未押下=1, 押下=0
            ((self.buttons & button::B) == 0) as u8
        } else {
            // シフトレジスタから1ビットずつ読み出し
            let result = (self.shift_register & 0x0001) as u8;
            self.shift_register >>= 1;
            // 読み切った後は1を返す（コントローラー接続確認）
            self.shift_register |= 0x8000;
            result
        }
    }

    fn latch_buttons(&mut self) {
        // ボタンの読み出し順序に合わせて並び替え
        // SNESの読み出し順序: B, Y, Select, Start, Up, Down, Left, Right, A, X, L, R
        // 実機出力に合わせアクティブLOW（未押下=1, 押下=0）に反転する
        self.latched_buttons = self.active_low_bits();
        self.shift_register = self.latched_buttons;
    }

    // デバッグ用：ボタン状態を文字列で表示
    #[allow(dead_code)]
    pub fn debug_buttons(&self) -> String {
        let mut result = String::new();

        if self.buttons & button::B != 0 {
            result.push_str("B ");
        }
        if self.buttons & button::Y != 0 {
            result.push_str("Y ");
        }
        if self.buttons & button::SELECT != 0 {
            result.push_str("Select ");
        }
        if self.buttons & button::START != 0 {
            result.push_str("Start ");
        }
        if self.buttons & button::UP != 0 {
            result.push_str("Up ");
        }
        if self.buttons & button::DOWN != 0 {
            result.push_str("Down ");
        }
        if self.buttons & button::LEFT != 0 {
            result.push_str("Left ");
        }
        if self.buttons & button::RIGHT != 0 {
            result.push_str("Right ");
        }
        if self.buttons & button::A != 0 {
            result.push_str("A ");
        }
        if self.buttons & button::X != 0 {
            result.push_str("X ");
        }
        if self.buttons & button::L != 0 {
            result.push_str("L ");
        }
        if self.buttons & button::R != 0 {
            result.push_str("R ");
        }

        if result.is_empty() {
            "None".to_string()
        } else {
            result
        }
    }
}

// 入力システム全体を管理
#[derive(Debug)]
pub struct InputSystem {
    pub controller1: SnesController,
    pub controller2: SnesController,
    pub controller3: SnesController,
    pub controller4: SnesController,
    multitap_enabled: bool,
}

impl InputSystem {
    pub fn new() -> Self {
        let mut controller1 = SnesController::new();
        // 環境変数 AUTO_JOYPAD1_MASK で常時押下するボタンを指定できる（例: 0x0100 = A）
        if let Ok(val) = std::env::var("AUTO_JOYPAD1_MASK") {
            if let Ok(mask) = u16::from_str_radix(val.trim_start_matches("0x"), 16)
                .or_else(|_| val.parse::<u16>())
            {
                controller1.set_auto_buttons(mask);
                if !debug_flags::quiet() {
                    println!("AUTO_JOYPAD1_MASK set: 0x{:04X}", mask);
                }
            }
        }
        Self {
            controller1,
            controller2: SnesController::new(),
            controller3: SnesController::new(),
            controller4: SnesController::new(),
            multitap_enabled: false,
        }
    }

    // コントローラー1の読み取り
    pub fn read_controller1(&mut self) -> u8 {
        self.controller1.read_data()
    }

    // コントローラー2の読み取り
    pub fn read_controller2(&mut self) -> u8 {
        self.controller2.read_data()
    }

    // For future multitap direct reads
    #[allow(dead_code)]
    pub fn read_controller3(&mut self) -> u8 {
        self.controller3.read_data()
    }
    #[allow(dead_code)]
    pub fn read_controller4(&mut self) -> u8 {
        self.controller4.read_data()
    }

    // ストローブ書き込み（両方のコントローラーに適用）
    pub fn write_strobe(&mut self, value: u8) {
        self.controller1.write_strobe(value);
        self.controller2.write_strobe(value);
    }

    // 外部からのキー入力を処理
    pub fn handle_key_input(&mut self, key_states: &KeyStates) {
        let mut buttons = 0u16;

        if key_states.up {
            buttons |= button::UP;
        }
        if key_states.down {
            buttons |= button::DOWN;
        }
        if key_states.left {
            buttons |= button::LEFT;
        }
        if key_states.right {
            buttons |= button::RIGHT;
        }
        if key_states.a {
            buttons |= button::A;
        }
        if key_states.b {
            buttons |= button::B;
        }
        if key_states.x {
            buttons |= button::X;
        }
        if key_states.y {
            buttons |= button::Y;
        }
        if key_states.l {
            buttons |= button::L;
        }
        if key_states.r {
            buttons |= button::R;
        }
        if key_states.start {
            buttons |= button::START;
        }
        if key_states.select {
            buttons |= button::SELECT;
        }

        self.controller1.set_buttons(buttons);
    }

    pub fn set_multitap_enabled(&mut self, enabled: bool) {
        self.multitap_enabled = enabled;
    }
    pub fn is_multitap_enabled(&self) -> bool {
        self.multitap_enabled
    }

    pub fn controller3_buttons(&self) -> u16 {
        self.controller3.get_buttons()
    }
    pub fn controller4_buttons(&self) -> u16 {
        self.controller4.get_buttons()
    }

    pub fn controller3_active_low(&self) -> u16 {
        self.controller3.active_low_bits()
    }

    pub fn controller4_active_low(&self) -> u16 {
        self.controller4.active_low_bits()
    }
}

// --- Save state helpers ---
impl InputSystem {
    pub fn to_save_state(&self) -> crate::savestate::InputSaveState {
        use crate::savestate::InputSaveState;
        InputSaveState {
            controller1_buttons: self.controller1.buttons,
            controller2_buttons: self.controller2.buttons,
            controller3_buttons: self.controller3.buttons,
            controller4_buttons: self.controller4.buttons,
            controller1_shift_register: self.controller1.shift_register,
            controller2_shift_register: self.controller2.shift_register,
            controller3_shift_register: self.controller3.shift_register,
            controller4_shift_register: self.controller4.shift_register,
            controller1_latched_buttons: self.controller1.latched_buttons,
            controller2_latched_buttons: self.controller2.latched_buttons,
            controller3_latched_buttons: self.controller3.latched_buttons,
            controller4_latched_buttons: self.controller4.latched_buttons,
            strobe: self.controller1.strobe || self.controller2.strobe,
            multitap_enabled: self.multitap_enabled,
        }
    }

    pub fn load_from_save_state(&mut self, st: &crate::savestate::InputSaveState) {
        self.controller1.buttons = st.controller1_buttons;
        self.controller2.buttons = st.controller2_buttons;
        self.controller3.buttons = st.controller3_buttons;
        self.controller4.buttons = st.controller4_buttons;
        self.controller1.shift_register = st.controller1_shift_register;
        self.controller2.shift_register = st.controller2_shift_register;
        self.controller3.shift_register = st.controller3_shift_register;
        self.controller4.shift_register = st.controller4_shift_register;
        self.controller1.latched_buttons = st.controller1_latched_buttons;
        self.controller2.latched_buttons = st.controller2_latched_buttons;
        self.controller3.latched_buttons = st.controller3_latched_buttons;
        self.controller4.latched_buttons = st.controller4_latched_buttons;
        self.controller1.strobe = st.strobe;
        self.controller2.strobe = st.strobe;
        self.controller3.strobe = st.strobe;
        self.controller4.strobe = st.strobe;
        self.multitap_enabled = st.multitap_enabled;
    }
}

// キーボード状態を表現する構造体
#[derive(Debug, Default)]
pub struct KeyStates {
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
    pub a: bool,
    pub b: bool,
    pub x: bool,
    pub y: bool,
    pub l: bool,
    pub r: bool,
    pub start: bool,
    pub select: bool,
}
