use futures::future::join_all;
use futures_util::StreamExt;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use reqwest::Client;
use std::cmp::min;
use std::env;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

async fn fetch_file(client: &Client, url: &String, path: String, pb: ProgressBar) -> Result<()> {
    let res = client
        .get(url)
        .send()
        .await
        .or(Err(format!("Failed to GET from '{}'", &url)))?;

    let total_size = res
        .content_length()
        .ok_or(format!("Failed to get content length from '{}'", &url))?;

    // let pb = ProgressBar::new(total_size);
    pb.inc_length(total_size);
    pb.set_style(ProgressStyle::default_bar()
            .template("{msg}\n{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})")?
            .progress_chars("=> "));

    pb.set_message(format!("Downloading {}", url));

    let mut file = File::create(&path)
        .await
        .or(Err(format!("Failed to create file '{}'", path)))?;
    let mut downloaded: u64 = 0;
    let mut stream = res.bytes_stream();

    while let Some(item) = stream.next().await {
        let chunk = item.or(Err(format!("Error while downloading file")))?;
        file.write_all(&chunk)
            .await
            .or(Err(format!("Error while writing to file")))?;
        let new = min(downloaded + (chunk.len() as u64), total_size);
        downloaded = new;
        pb.set_position(new);
    }

    pb.finish_with_message(format!("Downloaded {} to {}", url, path));

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut urls: Vec<String> = env::args().collect();

    let urls = urls.split_off(1);
    // let url = &urls[1];
    // println!("Program Name: {}", url);
    // fetch_file(&Client::new(), url).await?;
    let client = Client::new();
    // let tasks: Vec<_> = urls
    //     .iter()
    //     .enumerate()
    //     .map(|(i, url)| fetch_file(&client, url, format!("file-{}", i)))
    //     .collect();

    let mut handles = vec![];
    let multi = MultiProgress::new();

    for (i, url) in urls.iter().enumerate() {
        let url = url.clone();
        let client = client.clone();
        let pb = multi.add(ProgressBar::new(0));
        let handle =
            tokio::spawn(async move { fetch_file(&client, &url, format!("file-{}", i), pb).await });
        handles.push(handle);
    }
    let results = join_all(handles).await;
    // multi.join()?;
    Ok(())
}
