use std::io;
use std::io::prelude::*;
use std::fs::File;
use std::time::Instant;
use clap::{Parser, Subcommand};
use v4l::context;
use v4l::buffer::Type;
use v4l::io::traits::CaptureStream;
use v4l::prelude::*;
use v4l::video::Capture;
use jpeg_decoder as jpeg;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[arg(short, long="output")]
    output: String,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    Test {
        #[arg(short, long)]
        list: bool,
    },
}

fn main() -> io::Result<()> {

    let args = Cli::parse();
    
    let devices = context::enum_devices();

    for dev in devices {
        println!("{}: {}", dev.index(), dev.name().unwrap());
    }


    let path = "/dev/video1";
    println!("Using device: {}\n", path);

    // Capture 4 frames by default
    let count = 4;

    // Allocate 4 buffers by default
    let buffer_count = 4;

    let dev = Device::with_path(path)?;
    let format = dev.format()?;
    let params = dev.params()?;
    println!("Active format:\n{}", format);
    println!("Active parameters:\n{}", params);

    // Setup a buffer stream and grab a frame, then print its data
    let mut stream = MmapStream::with_buffers(&dev, Type::VideoCapture, buffer_count)?;

    // warmup
    stream.next()?;

    let start = Instant::now();
    let mut megabytes_ps: f64 = 0.0;
    for i in 0..count {
        let t0 = Instant::now();
        let (buf, meta) = stream.next()?;
        // use only the last shot, the first ones are for warmup 
        if count -1 < i {
            continue;
        }
        let duration_us = t0.elapsed().as_micros();

        let cur = buf.len() as f64 / 1_048_576.0 * 1_000_000.0 / duration_us as f64;
        if i == 0 {
            megabytes_ps = cur;
        } else {
            // ignore the first measurement
            let prev = megabytes_ps * (i as f64 / (i + 1) as f64);
            let now = cur * (1.0 / (i + 1) as f64);
            megabytes_ps = prev + now;
        }

        println!("Buffer");
        println!("  sequence  : {}", meta.sequence);
        println!("  timestamp : {}", meta.timestamp);
        println!("  flags     : {}", meta.flags);
        println!("  length    : {}", buf.len());

        let _ =  match  &format.fourcc.repr {
            b"RGB3" => buf.to_vec(),
            b"MJPG" => {
                // decode the jpeg fram to RGB
                let mut decoder = jpeg::Decoder::new(buf);
                decoder.decode().expect("failed to decode JPEG")
                
            }
            _ => panic!("invalid buffer pixel format"),
        }; 

        let mut f = File::create(args.output.clone())?;
        f.write(buf)?;
    }

    println!();
    println!("FPS: {}", count as f64 / start.elapsed().as_secs_f64());
    println!("MB/s: {}", megabytes_ps);

    Ok(())
}

