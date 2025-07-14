use ps2_filetypes::Key;

pub struct Timeline {
    keys: Vec<Key>,
}

impl Timeline {
    pub fn new(keys: Vec<Key>) -> Self {
        Self { keys }
    }

    pub fn evaluate(&self, t: f32) -> f32 {
        if t <= self.keys[0].time {
            return self.keys[0].value
        }
        if t >= self.keys[self.keys.len() - 1].time {
            return self.keys[self.keys.len() - 1].value
        }

        for i in 1..self.keys.len() {
            let k0 = self.keys[i - 1];
            let k1 = self.keys[i];

            if k0.time <= t && t < k1.time {
                let dt = k1.time - k0.time;
                if dt == 0.0 {
                    return k0.value;
                }
                let alpha = (t - k0.time) / dt;
                return (1.0 - alpha) * k0.value + alpha * k1.value;
            }
        }

        unreachable!();
    }
}