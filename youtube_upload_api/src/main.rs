#[derive(Serialize, Debug)]
struct YoutubeUpload {
    title: String,
    description: String,
    tags: Vec<String>,
    category: String,
    render_uri: String,
    thumbnail_uri: Option<String>,
    notify_subscribers: bool,
}

fn main() {
    println!("Hello, world!");
}
