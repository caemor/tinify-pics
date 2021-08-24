use std::{fs, io, path::Path};

use clap::{AppSettings, Clap};
use image::{
    imageops::{overlay, resize},
    DynamicImage, ImageResult,
};
use log::*;
use tinify_rs::{tinify, tinify::Source};

#[derive(Clap, Debug)]
#[clap(version = "0.1", author = "Chris <tinify@caemor.de")]
#[clap(setting = AppSettings::ColoredHelp)]
/// Tinifys (png, jpg) and convert (png) images with the tinify api.
/// It has two usage options: Single Image (`name`) and full Folder where everything inside a folder gets
/// tinified (and possibly converted)
struct Opts {
    /// Optional Tinify Key (alternative to .env file)
    #[clap(short, long)]
    key: Option<String>,
    /// Just convert a single picture (name = path to file)
    #[clap(short, long)]
    name: Option<String>,
    /// Folder for images to be converted
    #[clap(short, long, default_value = "tinify")]
    input_folder: String,
    /// Folder for tinified images (only for folder operations)
    #[clap(short, long)]
    output_folder: Option<String>,
    /// Appended Name pattern for tinified pictures
    #[clap(short, long, default_value = "_tiny")]
    pattern: String,
    /// Size for Resizing
    #[clap(short, long, default_value = "200")]
    size: u32,
    /// Option to only tinify picture for pngs (by default pngs get converted to size*size)
    #[clap(short, long)]
    tinify_only: bool,
}

fn main() -> ImageResult<()> {
    env_logger::init();
    let opts: Opts = Opts::parse();
    debug!("Opts: {:?}", opts);

    // Read tinify api key ...
    let key = match opts.key {
        Some(key) => key,
        None => dotenv::var("KEY")
            .expect("get `KEY` for tinify api from .env file or load it with `--key`"),
    };
    // ... and set it for the client
    tinify::set_key(&key);
    debug!("Tinify Key set!");

    // Check if resizing was desired
    let size = if opts.tinify_only {
        None
    } else {
        Some(opts.size)
    };

    // Tinify single image
    if let Some(name) = opts.name {
        tinify(name, None, &opts.pattern, size)?;
    } else
    // Tinify all images in folder
    {
        if !Path::new(&opts.input_folder).exists() {
            fs::create_dir(&opts.input_folder)?;
        }
        for entry in fs::read_dir(opts.input_folder)?
            .map(|res| res.map(|e| e.path()))
            .collect::<Result<Vec<_>, io::Error>>()?
        {
            tinify(
                entry
                    .to_str()
                    .expect("Unable to convert path to String")
                    .to_owned(),
                opts.output_folder.as_deref(),
                &opts.pattern,
                size,
            )?;
        }
    }

    Ok(())
}

// Tinifies Images and possibly converts them if given some `convert_size` and a png image
fn tinify(
    file: String,
    output: Option<&str>,
    pattern: &str,
    convert_size: Option<u32>,
) -> ImageResult<()> {
    // Generate output path and exit if it already exists
    let output = output
        .map(|x| x.to_string())
        .unwrap_or(file.clone())
        .replace(".png", &format!("{}.png", pattern))
        .replace(".jpg", &format!("{}.jpg", pattern));

    let is_png = output.ends_with(".png");

    // Filter alredy processed images
    if output.contains(&format!("{}{}", &pattern, &pattern)) {
        debug!("Image is an already tinified output! (file: '{}')", file);
        return Ok(());
    }

    // If output file already exists, stop processing and return Ok but output error message
    if Path::new(&output).exists() {
        info!(
            "Tinified Output for '{}' already exists! (output: '{}')",
            file, output
        );
        return Ok(());
    }

    let source = match (is_png, convert_size) {
        // Convert and tinify if png with size...
        (true, Some(new_size)) => convert_and_tinify(&file, new_size)?,
        // ... or else just tinify
        (_, _) => tinify::from_file(&file),
    };
    debug!("Image {} tinified", &file);

    // Write to file
    source.to_file(&output)?;
    debug!("Tinified image {} written to file", &output);

    Ok(())
}

// Converts an image to square format and resizes it to `new_size` and directly tinifies it
// Helper function for `tinify`
fn convert_and_tinify(file: &str, new_size: u32) -> ImageResult<Source> {
    // Open Image
    let logo = image::open(&file)
        .expect(&format!("Could not load image at {:?}", file))
        .to_rgba8();

    // Get proportions
    let width = logo.width();
    let height = logo.height();
    let max_length = width.max(height);

    // Create new background image
    let mut img: DynamicImage = DynamicImage::new_rgba8(max_length, max_length);

    // Overlay new background with logo
    overlay(
        &mut img,
        &logo,
        (max_length - width) / 2,
        (max_length - height) / 2,
    );

    // Resize
    let resized: DynamicImage = DynamicImage::ImageRgba8(resize(
        &img,
        new_size,
        new_size,
        image::imageops::FilterType::Lanczos3,
    ));

    // Write to buffer/vec
    let mut bytes: Vec<u8> = Vec::new();
    resized.write_to(&mut bytes, image::ImageOutputFormat::Png)?;

    // Tinify
    Ok(tinify::from_buffer(&bytes))
}
