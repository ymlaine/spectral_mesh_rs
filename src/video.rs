#[cfg(feature = "camera")]
use nokhwa::{
    pixel_format::RgbFormat,
    utils::{CameraIndex, RequestedFormat, RequestedFormatType},
    Camera,
};
use std::sync::mpsc::{channel, Receiver, Sender, TryRecvError};
use std::thread;

pub struct VideoCapture {
    receiver: Receiver<Vec<u8>>,
    pub width: u32,
    pub height: u32,
    current_frame: Vec<u8>,
    #[allow(dead_code)]
    handle: Option<thread::JoinHandle<()>>,
}

impl VideoCapture {
    #[cfg(feature = "camera")]
    pub fn new(width: u32, height: u32, device_index: u32) -> Result<Self, String> {
        let (sender, receiver) = channel();
        let frame_size = (width * height * 4) as usize;

        let handle = thread::spawn(move || {
            Self::camera_thread(sender, width, height, device_index);
        });

        Ok(Self {
            receiver,
            width,
            height,
            current_frame: vec![128u8; frame_size],
            handle: Some(handle),
        })
    }

    #[cfg(feature = "camera")]
    fn camera_thread(sender: Sender<Vec<u8>>, target_width: u32, target_height: u32, device_index: u32) {
        let index = CameraIndex::Index(device_index);

        let requested = RequestedFormat::new::<RgbFormat>(
            RequestedFormatType::AbsoluteHighestFrameRate
        );

        log::info!("Opening camera {}...", device_index);

        let mut camera = match Camera::new(index, requested) {
            Ok(cam) => {
                log::info!("Camera opened: {:?}", cam.info());
                cam
            }
            Err(e) => {
                log::error!("Failed to open camera: {}", e);
                return;
            }
        };

        if let Err(e) = camera.open_stream() {
            log::error!("Failed to open camera stream: {}", e);
            return;
        }

        let resolution = camera.resolution();
        log::info!("Camera stream started at {}x{}", resolution.width(), resolution.height());

        let mut frame_count = 0u64;

        loop {
            match camera.frame() {
                Ok(frame) => {
                    match frame.decode_image::<RgbFormat>() {
                        Ok(rgb_image) => {
                            let cam_width = rgb_image.width();
                            let cam_height = rgb_image.height();

                            // Resize to target resolution
                            let mut rgba = vec![0u8; (target_width * target_height * 4) as usize];

                            for ty in 0..target_height {
                                for tx in 0..target_width {
                                    // Map target coords to source coords (flip Y)
                                    let sx = (tx as f32 / target_width as f32 * cam_width as f32) as u32;
                                    let sy = ((target_height - 1 - ty) as f32 / target_height as f32 * cam_height as f32) as u32;

                                    let sx = sx.min(cam_width - 1);
                                    let sy = sy.min(cam_height - 1);

                                    if let Some(pixel) = rgb_image.get_pixel_checked(sx, sy) {
                                        let idx = ((ty * target_width + tx) * 4) as usize;
                                        rgba[idx] = pixel.0[0];     // R
                                        rgba[idx + 1] = pixel.0[1]; // G
                                        rgba[idx + 2] = pixel.0[2]; // B
                                        rgba[idx + 3] = 255;        // A
                                    }
                                }
                            }

                            frame_count += 1;
                            if frame_count % 60 == 0 {
                                log::debug!("Camera: {} frames captured", frame_count);
                            }

                            if sender.send(rgba).is_err() {
                                log::info!("Camera thread stopping (receiver dropped)");
                                break;
                            }
                        }
                        Err(e) => {
                            log::warn!("Failed to decode frame: {}", e);
                        }
                    }
                }
                Err(e) => {
                    log::warn!("Frame capture error: {}", e);
                    thread::sleep(std::time::Duration::from_millis(10));
                }
            }
        }
    }

    #[cfg(not(feature = "camera"))]
    pub fn new(width: u32, height: u32, _device_index: u32) -> Result<Self, String> {
        Err("Camera support not compiled. Enable 'camera' feature.".to_string())
    }

    pub fn get_frame(&mut self) -> Option<&[u8]> {
        let mut got_frame = false;
        loop {
            match self.receiver.try_recv() {
                Ok(frame) => {
                    self.current_frame = frame;
                    got_frame = true;
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => break,
            }
        }

        if got_frame {
            Some(&self.current_frame)
        } else {
            None
        }
    }

    pub fn current_frame(&self) -> &[u8] {
        &self.current_frame
    }
}

/// Dummy video source for testing without camera
pub struct DummyVideoSource {
    pub width: u32,
    pub height: u32,
    frame: Vec<u8>,
    frame_count: u32,
}

impl DummyVideoSource {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            frame: vec![0u8; (width * height * 4) as usize],
            frame_count: 0,
        }
    }

    pub fn update(&mut self) -> &[u8] {
        let phase = self.frame_count as f32 * 0.02;

        for y in 0..self.height {
            for x in 0..self.width {
                let idx = ((y * self.width + x) * 4) as usize;
                let fx = x as f32 / self.width as f32;
                let fy = y as f32 / self.height as f32;

                let v1 = (fx * 10.0 + phase).sin();
                let v2 = (fy * 10.0 + phase * 1.5).sin();
                let v3 = ((fx + fy) * 8.0 + phase * 0.5).sin();
                let v4 = ((fx * fx + fy * fy).sqrt() * 15.0 + phase * 2.0).sin();

                let r = ((v1 + v2 + 2.0) / 4.0 * 255.0) as u8;
                let g = ((v2 + v3 + 2.0) / 4.0 * 255.0) as u8;
                let b = ((v3 + v4 + 2.0) / 4.0 * 255.0) as u8;

                self.frame[idx] = r;
                self.frame[idx + 1] = g;
                self.frame[idx + 2] = b;
                self.frame[idx + 3] = 255;
            }
        }

        self.frame_count = self.frame_count.wrapping_add(1);
        &self.frame
    }
}
