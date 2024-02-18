use tokio::process::Command;

pub async fn get_video_duration(path: &str) -> Result<std::time::Duration, String> {
    let output = match Command::new("ffprobe")
        .arg("-v")
        .arg("error")
        .arg("-show_entries")
        .arg("format=duration")
        .arg("-of")
        .arg("default=noprint_wrappers=1:nokey=1")
        .arg(path)
        .output()
        .await
    {
        Ok(output) => output,
        Err(e) => return Err(e.to_string()),
    };

    let output = match String::from_utf8(output.stdout) {
        Ok(output) => output,
        Err(e) => return Err(e.to_string()),
    };

    let output = match output.trim().parse::<f64>() {
        Ok(output) => output,
        Err(e) => return Err(e.to_string()),
    };

    Ok(std::time::Duration::from_secs_f64(output))
}
