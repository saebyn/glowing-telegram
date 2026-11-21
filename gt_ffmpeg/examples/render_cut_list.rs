//! Offline fixture driver for the production FFmpeg command builder.
//! Usage: cargo run -p gt_ffmpeg --example render_cut_list -- cut-list.json output.mp4
//! Accepts GT's internal CutList spelling (`duration`, not `durationFrames`).
use std::{env, error::Error, fs};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = env::args().skip(1).collect();
    if args.len() != 2 {
        return Err(
            "usage: render_cut_list <cut-list.json> <output.mp4>".into()
        );
    }
    let cut_list: types::CutList =
        serde_json::from_slice(&fs::read(&args[0])?)?;
    gt_ffmpeg::edit::validate_source_audio(
        &cut_list,
        std::path::Path::new("."),
        60.0,
    )
    .await
    .map_err(|error| error.to_string())?;
    let mut command = gt_ffmpeg::edit::build_ffmpeg_command(
        &cut_list,
        60.0,
        &args[1],
        (96, 64),
    );
    let status = command.status().await?;
    if !status.success() {
        return Err(format!("ffmpeg failed: {status}").into());
    }
    Ok(())
}
