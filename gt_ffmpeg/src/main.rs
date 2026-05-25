use std::path::PathBuf;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about = "Transcode a video into HLS output")]
struct Args {
    /// Input media file to transcode.
    input: PathBuf,

    /// Output directory for HLS segments and index.m3u8.
    output: PathBuf,

    /// Optional timestamp offset in seconds for the output timeline.
    #[arg(long)]
    offset: Option<f64>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt::init();

    let args = Args::parse();

    tokio::fs::create_dir_all(&args.output).await?;

    let entries = gt_ffmpeg::transcode::hls(
        &args.output.to_string_lossy(),
        &args.input.to_string_lossy(),
        args.offset,
    )
    .await?;

    for entry in entries {
        println!("{}\t{}", entry.duration, entry.path);
    }

    Ok(())
}
