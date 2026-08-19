use super::config::{random_u32, Lobe, ShapeConfig, MAX_LOBES, MIN_LOBES};
use super::{HEIGHT, WIDTH};

fn rnd(lo: i32, hi: i32) -> i32 {
    let span = (hi - lo + 1) as u32;
    lo + (random_u32() % span) as i32
}

pub(crate) fn generate() -> ShapeConfig {
    let cx = WIDTH / 2;
    let cy = HEIGHT / 2;
    let n = rnd(MIN_LOBES as i32, MAX_LOBES as i32) as usize;
    let mut lobes = [Lobe::EMPTY; MAX_LOBES];

    lobes[0] = Lobe::new(cx as u8, cy as u8, rnd(56, 68) as u8);
    for lobe in lobes.iter_mut().take(n).skip(1) {
        let deg = rnd(0, 359);
        let dist = rnd(10, 26) as f32;
        let rad = (deg as f32).to_radians();
        *lobe = Lobe::new(
            (cx + (dist * rad.cos()) as i32) as u8,
            (cy + (dist * rad.sin()) as i32) as u8,
            rnd(30, 46) as u8,
        );
    }

    ShapeConfig::new(
        [rnd(32, 255) as u8, rnd(32, 255) as u8, rnd(32, 255) as u8],
        n as u8,
        lobes,
    )
}
