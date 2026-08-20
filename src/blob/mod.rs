mod config;
mod idle;
mod shape;

use std::time::Instant;

use embedded_graphics::framebuffer::{buffer_size, Framebuffer};
use embedded_graphics::image::Image;
use embedded_graphics::pixelcolor::raw::{LittleEndian, RawU16};
use embedded_graphics::pixelcolor::{Rgb565, Rgb888};
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{Circle, Ellipse, PrimitiveStyle};

use idle::SCALE_ONE;

pub(crate) use config::random_u32;
pub use config::{BlobConfig, Personality};

pub const WIDTH: i32 = 128;
pub const HEIGHT: i32 = 160;

const SPRITE_ORIGIN: Point = Point::new(16, 29);
const SPRITE_WIDTH: usize = 96;
const SPRITE_HEIGHT: usize = 104;

type Sprite = Framebuffer<
    Rgb565,
    RawU16,
    LittleEndian,
    SPRITE_WIDTH,
    SPRITE_HEIGHT,
    { buffer_size::<Rgb565>(SPRITE_WIDTH, SPRITE_HEIGHT) },
>;

#[derive(Clone, Copy, Eq, PartialEq)]
struct Pose {
    bob: i32,
    blink: i32,
}

pub struct Blob {
    config: BlobConfig,
    born: Instant,
    pose: Pose,
    expression: Option<(i32, i32)>,
    body: Box<Sprite>,
    frame: Box<Sprite>,
}

impl Blob {
    pub fn generate() -> Self {
        Self::from_config(BlobConfig::new(shape::generate(), Personality::random()))
    }

    pub fn from_config(config: BlobConfig) -> Self {
        let mut blob = Self {
            config,
            born: Instant::now(),
            pose: Pose { bob: 0, blink: 0 },
            expression: None,
            body: Box::new(Framebuffer::new()),
            frame: Box::new(Framebuffer::new()),
        };
        blob.reset();
        blob
    }

    pub fn config(&self) -> BlobConfig {
        self.config
    }

    pub fn draw<D>(&mut self, display: &mut D) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = Rgb565>,
    {
        display.clear(background())?;
        self.blit(display)
    }

    pub fn regenerate(&mut self) {
        self.config = self
            .config
            .with_random_shape()
            .with_personality(Personality::random());
        self.reset();
    }

    pub fn randomize_personality(&mut self) {
        self.config = self.config.with_personality(Personality::random());
        self.reset();
    }

    pub fn set_expression_overlay(&mut self, bob_offset: i32, eye_scale: i32) -> bool {
        let expression = (bob_offset, eye_scale);
        if self.expression == Some(expression) {
            return false;
        }
        self.expression = Some(expression);
        self.render_frame();
        true
    }

    pub fn clear_expression_overlay(&mut self) -> bool {
        if self.expression.is_none() {
            return false;
        }
        self.expression = None;
        self.render_frame();
        true
    }

    fn reset(&mut self) {
        self.born = Instant::now();
        self.pose = self.pose_at(0);
        self.render_body();
        self.render_frame();
    }

    pub fn animate<D>(&mut self, display: &mut D) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = Rgb565>,
    {
        let elapsed = self.born.elapsed().as_millis() as u32;
        let pose = self.pose_at(elapsed);
        if pose == self.pose {
            return Ok(());
        }

        self.pose = pose;
        self.render_frame();
        self.blit(display)
    }

    fn pose_at(&self, elapsed: u32) -> Pose {
        let personality = self.config.personality();
        Pose {
            bob: idle::bob(elapsed, personality),
            blink: idle::blink(elapsed, personality),
        }
    }

    fn render_body(&mut self) {
        self.body.clear(background()).unwrap();
        let shape = self.config.shape();
        let fill = PrimitiveStyle::with_fill(rgb(shape.color[0], shape.color[1], shape.color[2]));

        for lobe in shape.lobes.iter().take(usize::from(shape.lobe_count)) {
            Circle::with_center(
                sprite_point(i32::from(lobe.x), i32::from(lobe.y)),
                u32::from(lobe.diameter),
            )
            .into_styled(fill)
            .draw(&mut *self.body)
            .unwrap();
        }
    }

    fn render_frame(&mut self) {
        self.frame.clear(background()).unwrap();
        let (bob_offset, eye_scale) = self.expression.unwrap_or((0, 100));
        let bob = self.pose.bob + bob_offset;

        let body = self.body.as_image();
        Image::new(&body, Point::new(0, bob))
            .draw(&mut *self.frame)
            .unwrap();

        let eye = PrimitiveStyle::with_fill(rgb(0x10, 0x18, 0x20));
        let eh = (16 * self.pose.blink as u32 * eye_scale as u32 / SCALE_ONE as u32 / 100).max(1);
        let cy = HEIGHT / 2 - 2;
        Ellipse::with_center(
            sprite_point(WIDTH / 2 - 16, cy) + Point::new(0, bob),
            Size::new(10, eh),
        )
        .into_styled(eye)
        .draw(&mut *self.frame)
        .unwrap();
        Ellipse::with_center(
            sprite_point(WIDTH / 2 + 16, cy) + Point::new(0, bob),
            Size::new(10, eh),
        )
        .into_styled(eye)
        .draw(&mut *self.frame)
        .unwrap();
    }

    fn blit<D>(&self, display: &mut D) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = Rgb565>,
    {
        Image::new(&self.frame.as_image(), SPRITE_ORIGIN).draw(display)
    }
}

fn background() -> Rgb565 {
    rgb(0, 0, 0)
}

fn rgb(r: u8, g: u8, b: u8) -> Rgb565 {
    Rgb565::from(Rgb888::new(r, g, b))
}

fn sprite_point(x: i32, y: i32) -> Point {
    Point::new(x - SPRITE_ORIGIN.x, y - SPRITE_ORIGIN.y)
}
