use std::{
    f32::consts::{FRAC_PI_2, PI, TAU},
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use clap::Parser;
use image::{ImageBuffer, Rgba, RgbaImage};

#[derive(Debug, clap::Parser)]
pub struct Args {
    #[arg(short, long)]
    pub input_file_path: String,
    #[arg(short, long)]
    pub output_folder_path: String,
    #[arg(short, long)]
    pub face_size: Option<u32>,
}

#[derive(Debug, Clone, Copy)]
struct CubeFace {
    layer: u32,
    label: &'static str,
}

const CUBE_FACES: [CubeFace; 6] = [
    CubeFace {
        layer: 0,
        label: "pos_x",
    },
    CubeFace {
        layer: 1,
        label: "neg_x",
    },
    CubeFace {
        layer: 2,
        label: "pos_y",
    },
    CubeFace {
        layer: 3,
        label: "neg_y",
    },
    CubeFace {
        layer: 4,
        label: "pos_z",
    },
    CubeFace {
        layer: 5,
        label: "neg_z",
    },
];

fn main() -> Result<()> {
    let args = Args::parse();
    convert_panorama_to_cubemap(
        Path::new(&args.input_file_path),
        Path::new(&args.output_folder_path),
        args.face_size,
    )
}

fn convert_panorama_to_cubemap(
    input_file_path: &Path,
    output_folder_path: &Path,
    face_size: Option<u32>,
) -> Result<()> {
    let panorama = image::open(input_file_path)
        .with_context(|| format!("failed to open {}", input_file_path.display()))?
        .to_rgba8();
    let (panorama_width, panorama_height) = panorama.dimensions();
    if panorama_width == 0 || panorama_height == 0 {
        bail!("input panorama must not be empty");
    }

    let face_size = face_size.unwrap_or(panorama_width / 4);
    if face_size == 0 {
        bail!("face size must be greater than zero");
    }

    std::fs::create_dir_all(output_folder_path)
        .with_context(|| format!("failed to create {}", output_folder_path.display()))?;

    for face in CUBE_FACES {
        let image = render_face(&panorama, face, face_size);
        let output_path = output_path(output_folder_path, face);
        image
            .save(&output_path)
            .with_context(|| format!("failed to save {}", output_path.display()))?;
        println!("{}", output_path.display());
    }

    Ok(())
}

fn output_path(output_folder_path: &Path, face: CubeFace) -> PathBuf {
    output_folder_path.join(format!("{}_{}.png", face.layer, face.label))
}

fn render_face(panorama: &RgbaImage, face: CubeFace, face_size: u32) -> RgbaImage {
    ImageBuffer::from_fn(face_size, face_size, |x, y| {
        let s = 2.0 * ((x as f32 + 0.5) / face_size as f32) - 1.0;
        let t = 2.0 * ((y as f32 + 0.5) / face_size as f32) - 1.0;
        let direction = direction_for_face(face.layer, s, t);
        sample_panorama(panorama, direction)
    })
}

fn direction_for_face(layer: u32, s: f32, t: f32) -> [f32; 3] {
    let [x, y, z] = match layer {
        0 => [1.0, -t, -s],
        1 => [-1.0, -t, s],
        2 => [s, 1.0, t],
        3 => [s, -1.0, -t],
        4 => [s, -t, 1.0],
        5 => [-s, -t, -1.0],
        _ => unreachable!("cubemaps have exactly six faces"),
    };
    let length = (x * x + y * y + z * z).sqrt();
    [x / length, y / length, z / length]
}

fn sample_panorama(panorama: &RgbaImage, direction: [f32; 3]) -> Rgba<u8> {
    let [x, y, z] = direction;
    let longitude = z.atan2(x);
    let latitude = y.asin().clamp(-FRAC_PI_2, FRAC_PI_2);
    let u = (0.5 + longitude / TAU).rem_euclid(1.0);
    let v = 0.5 - latitude / PI;

    let source_x = u * panorama.width() as f32 - 0.5;
    let source_y = v * panorama.height() as f32 - 0.5;
    sample_bilinear(panorama, source_x, source_y)
}

fn sample_bilinear(image: &RgbaImage, source_x: f32, source_y: f32) -> Rgba<u8> {
    let width = image.width() as i32;
    let height = image.height() as i32;

    let x0 = source_x.floor() as i32;
    let y0 = source_y.floor() as i32;
    let x_weight = source_x - x0 as f32;
    let y_weight = source_y - y0 as f32;

    let x0 = x0.rem_euclid(width) as u32;
    let x1 = ((x0 + 1) % image.width()) as u32;
    let y0 = y0.clamp(0, height - 1) as u32;
    let y1 = (y0 + 1).min(image.height() - 1);

    let top_left = image.get_pixel(x0, y0);
    let top_right = image.get_pixel(x1, y0);
    let bottom_left = image.get_pixel(x0, y1);
    let bottom_right = image.get_pixel(x1, y1);

    let mut out = [0; 4];
    for channel in 0..4 {
        let top = lerp(
            top_left[channel] as f32,
            top_right[channel] as f32,
            x_weight,
        );
        let bottom = lerp(
            bottom_left[channel] as f32,
            bottom_right[channel] as f32,
            x_weight,
        );
        out[channel] = lerp(top, bottom, y_weight).round().clamp(0.0, 255.0) as u8;
    }
    Rgba(out)
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}
