use core::f32::consts::PI;

use super::config::Personality;

pub const SCALE_ONE: i32 = 256;

const BLINK_CLOSED: i32 = 32;

fn ease_in_out(t: f32) -> f32 {
    (1.0 - (t.clamp(0.0, 1.0) * PI).cos()) * 0.5
}

pub fn bob(ms: u32, personality: Personality) -> i32 {
    let bob_ms = 2_400 - u32::from(personality.energy) * 16;
    let bob_min = -1 - i32::from(personality.playfulness) / 50;
    let bob_max = i32::from(personality.confidence) / 50;
    let t = ms % (bob_ms * 2);
    let u = if t < bob_ms { t } else { bob_ms * 2 - t };
    let e = ease_in_out(u as f32 / bob_ms as f32);
    bob_min + ((bob_max - bob_min) as f32 * e) as i32
}

pub fn blink(ms: u32, personality: Personality) -> i32 {
    let blink_first_ms = 1_000 + u32::from(personality.sleepiness) * 8;
    if ms < blink_first_ms {
        return open_eyes(personality);
    }
    let blink_ms = 60 + u32::from(personality.sleepiness);
    let max_value = u32::from(Personality::MAX_VALUE);
    let blink_gap_ms = 2_500
        + u32::from(personality.attention) * 20
        + (max_value - u32::from(personality.energy)) * 10;
    let period = blink_ms * 2 + blink_gap_ms;
    let p = (ms - blink_first_ms) % period;
    let open = open_eyes(personality);
    let closed = (BLINK_CLOSED
        + (i32::from(Personality::MAX_VALUE) - i32::from(personality.confidence)) / 4)
        .min(open);
    if p < blink_ms {
        open - (open - closed) * p as i32 / blink_ms as i32
    } else if p < blink_ms * 2 {
        closed + (open - closed) * (p - blink_ms) as i32 / blink_ms as i32
    } else {
        open
    }
}

fn open_eyes(personality: Personality) -> i32 {
    SCALE_ONE - i32::from(personality.sleepiness) / 3
}
