use super::{HEIGHT, WIDTH};

pub const MAX_LOBES: usize = 15;
pub const MIN_LOBES: usize = 9;

const VERSION: u8 = 1;
const COLOR_LEN: usize = 3;
const LOBE_LEN: usize = 3;
const PERSONALITY_LEN: usize = 5;
const VERSION_OFFSET: usize = 0;
const LOBE_COUNT_OFFSET: usize = VERSION_OFFSET + 1;
const COLOR_OFFSET: usize = LOBE_COUNT_OFFSET + 1;
const LOBES_OFFSET: usize = COLOR_OFFSET + COLOR_LEN;
const PERSONALITY_OFFSET: usize = LOBES_OFFSET + MAX_LOBES * LOBE_LEN;

const BLOB_CONFIG_LEN: usize = PERSONALITY_OFFSET + PERSONALITY_LEN;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// User-adjustable traits in the inclusive `0..=MAX_VALUE` range.
pub struct Personality {
    pub energy: u8,
    pub attention: u8,
    pub confidence: u8,
    pub playfulness: u8,
    pub sleepiness: u8,
}

impl Personality {
    pub const MAX_VALUE: u8 = 100;

    pub fn random() -> Self {
        Self {
            energy: random_percent(),
            attention: random_percent(),
            confidence: random_percent(),
            playfulness: random_percent(),
            sleepiness: random_percent(),
        }
    }

    fn is_valid(self) -> bool {
        [
            self.energy,
            self.attention,
            self.confidence,
            self.playfulness,
            self.sleepiness,
        ]
        .into_iter()
        .all(|value| value <= Self::MAX_VALUE)
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
/// The versioned blob profile stored in ESP32 NVS.
pub struct BlobConfig {
    shape: ShapeConfig,
    personality: Personality,
}

impl BlobConfig {
    pub const SERIALIZED_LEN: usize = BLOB_CONFIG_LEN;

    pub(crate) const fn new(shape: ShapeConfig, personality: Personality) -> Self {
        Self { shape, personality }
    }

    pub fn personality(self) -> Personality {
        self.personality
    }

    pub(crate) const fn with_personality(self, personality: Personality) -> Self {
        Self {
            shape: self.shape,
            personality,
        }
    }

    pub fn serialize(self) -> [u8; BLOB_CONFIG_LEN] {
        let mut bytes = [0; BLOB_CONFIG_LEN];
        bytes[VERSION_OFFSET] = VERSION;
        bytes[LOBE_COUNT_OFFSET] = self.shape.lobe_count;
        bytes[COLOR_OFFSET..LOBES_OFFSET].copy_from_slice(&self.shape.color);

        for (index, lobe) in self.shape.lobes.iter().enumerate() {
            let offset = LOBES_OFFSET + index * LOBE_LEN;
            bytes[offset] = lobe.x;
            bytes[offset + 1] = lobe.y;
            bytes[offset + 2] = lobe.diameter;
        }

        bytes[PERSONALITY_OFFSET] = self.personality.energy;
        bytes[PERSONALITY_OFFSET + 1] = self.personality.attention;
        bytes[PERSONALITY_OFFSET + 2] = self.personality.confidence;
        bytes[PERSONALITY_OFFSET + 3] = self.personality.playfulness;
        bytes[PERSONALITY_OFFSET + 4] = self.personality.sleepiness;
        bytes
    }

    pub fn deserialize(bytes: &[u8]) -> Result<Self, BlobConfigError> {
        if bytes.len() != BLOB_CONFIG_LEN {
            return Err(BlobConfigError::InvalidLength);
        }
        if bytes[VERSION_OFFSET] != VERSION {
            return Err(BlobConfigError::UnsupportedVersion);
        }

        let mut lobes = [Lobe::EMPTY; MAX_LOBES];
        for (index, lobe) in lobes.iter_mut().enumerate() {
            let offset = LOBES_OFFSET + index * LOBE_LEN;
            *lobe = Lobe::new(bytes[offset], bytes[offset + 1], bytes[offset + 2]);
        }

        let shape = ShapeConfig::new(
            [
                bytes[COLOR_OFFSET],
                bytes[COLOR_OFFSET + 1],
                bytes[COLOR_OFFSET + 2],
            ],
            bytes[LOBE_COUNT_OFFSET],
            lobes,
        );
        if !shape.is_valid() {
            return Err(BlobConfigError::InvalidShape);
        }

        let personality = Personality {
            energy: bytes[PERSONALITY_OFFSET],
            attention: bytes[PERSONALITY_OFFSET + 1],
            confidence: bytes[PERSONALITY_OFFSET + 2],
            playfulness: bytes[PERSONALITY_OFFSET + 3],
            sleepiness: bytes[PERSONALITY_OFFSET + 4],
        };
        if !personality.is_valid() {
            return Err(BlobConfigError::InvalidPersonality);
        }

        Ok(Self { shape, personality })
    }

    pub(crate) const fn shape(self) -> ShapeConfig {
        self.shape
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BlobConfigError {
    InvalidLength,
    UnsupportedVersion,
    InvalidShape,
    InvalidPersonality,
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) struct ShapeConfig {
    pub(crate) color: [u8; COLOR_LEN],
    pub(crate) lobe_count: u8,
    pub(crate) lobes: [Lobe; MAX_LOBES],
}

impl ShapeConfig {
    pub(crate) const fn new(
        color: [u8; COLOR_LEN],
        lobe_count: u8,
        lobes: [Lobe; MAX_LOBES],
    ) -> Self {
        Self {
            color,
            lobe_count,
            lobes,
        }
    }

    fn is_valid(self) -> bool {
        let lobe_count = usize::from(self.lobe_count);
        (MIN_LOBES..=MAX_LOBES).contains(&lobe_count)
            && self
                .lobes
                .iter()
                .take(lobe_count)
                .all(|lobe| lobe.is_valid())
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) struct Lobe {
    pub(crate) x: u8,
    pub(crate) y: u8,
    pub(crate) diameter: u8,
}

impl Lobe {
    pub(crate) const EMPTY: Self = Self::new(0, 0, 0);

    pub(crate) const fn new(x: u8, y: u8, diameter: u8) -> Self {
        Self { x, y, diameter }
    }

    fn is_valid(self) -> bool {
        self.x < WIDTH as u8 && self.y < HEIGHT as u8 && (1..=WIDTH as u8).contains(&self.diameter)
    }
}

pub(crate) fn random_u32() -> u32 {
    // SAFETY: esp_random takes no pointers and has no preconditions.
    unsafe { esp_idf_svc::sys::esp_random() }
}

fn random_percent() -> u8 {
    (random_u32() % (u32::from(Personality::MAX_VALUE) + 1)) as u8
}
