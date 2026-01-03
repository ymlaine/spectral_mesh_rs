use noise::{NoiseFn, Perlin};

pub struct NoiseGenerator {
    perlin: Perlin,
    pub width: u32,
    pub height: u32,
    pixels: Vec<u8>,
}

impl NoiseGenerator {
    pub fn new(width: u32, height: u32, seed: u32) -> Self {
        Self {
            perlin: Perlin::new(seed),
            width,
            height,
            pixels: vec![0u8; (width * height) as usize],
        }
    }

    /// Generate Perlin noise texture
    /// theta: time/animation offset
    /// resolution: noise scale (smaller = smoother)
    pub fn generate(&mut self, theta: f32, resolution: f32) -> &[u8] {
        let resolution = resolution * 0.05;
        let theta = theta * 0.1;

        for y in 0..self.height {
            for x in 0..self.width {
                let noise_value = self.perlin.get([
                    (x as f64) * resolution as f64,
                    (y as f64) * resolution as f64,
                    theta as f64,
                ]);

                // Convert from [-1, 1] to [0, 255]
                let pixel = ((noise_value + 1.0) * 0.5 * 255.0) as u8;
                self.pixels[(y * self.width + x) as usize] = pixel;
            }
        }

        &self.pixels
    }

    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }
}

pub struct NoiseBank {
    pub x_noise: NoiseGenerator,
    pub y_noise: NoiseGenerator,
    pub z_noise: NoiseGenerator,
}

impl NoiseBank {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            x_noise: NoiseGenerator::new(width, height, 0),
            y_noise: NoiseGenerator::new(width, height, 1),
            z_noise: NoiseGenerator::new(width, height, 2),
        }
    }

    /// Update all noise textures with their respective parameters
    pub fn update(
        &mut self,
        x_theta: f32,
        x_resolution: f32,
        y_theta: f32,
        y_resolution: f32,
        z_theta: f32,
        z_resolution: f32,
    ) {
        self.x_noise.generate(x_theta, x_resolution);
        self.y_noise.generate(y_theta, y_resolution);
        self.z_noise.generate(z_theta, z_resolution);
    }
}
