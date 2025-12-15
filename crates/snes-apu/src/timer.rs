pub struct Timer {
    resolution: i32,
    is_running: bool,
    ticks: i32,
    // Target value written via $FA-$FC. Note: value 0 means 256 ticks.
    target: u8,
    counter_low: u8,
    counter_high: u8
}

impl Timer {
    pub fn new(resolution: i32) -> Timer {
        Timer {
            resolution: resolution,
            is_running: false,
            ticks: 0,
            target: 0,
            counter_low: 0,
            counter_high: 0
        }
    }

    pub fn reset(&mut self) {
        self.is_running = false;
        self.target = 0;
        self.counter_low = 0;
        self.counter_high = 0;
    }

    pub fn cpu_cycles_callback(&mut self, num_cycles: i32) {
        if !self.is_running {
            return;
        }
        self.ticks += num_cycles;
        // Timers tick when the internal divider reaches the configured resolution.
        // Using `>=` avoids an off-by-one that would slow timers (e.g., 33 instead of 32 cycles).
        while self.ticks >= self.resolution {
            self.ticks -= self.resolution;

            self.counter_low += 1;
            // Stage 2: post-increment compare against target. A target value of 0
            // corresponds to 256 ticks, which naturally matches on wrap to 0.
            if self.counter_low == self.target {
                self.counter_high += 1;
                self.counter_low = 0;
            }
        }
    }

    pub fn set_start_stop_bit(&mut self, value: bool) {
        // Writing 1 restarts the timer even if already running.
        if value {
            self.ticks = 0;
            self.counter_low = 0;
            self.counter_high = 0;
        }
        self.is_running = value;
    }

    pub fn set_target(&mut self, value: u8) {
        self.target = value;
    }

    pub fn read_counter(&mut self) -> u8 {
        let ret = self.counter_high & 0x0f;
        self.counter_high = 0;
        ret
    }
}
