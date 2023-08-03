use std::collections::HashMap;
use std::{env, io};
use std::thread;
use std::sync::{Arc, Mutex};

use axum::{extract::Query, http::StatusCode, routing::get, Json, Router};
use dotenv::dotenv;
use hyper::{Body, Client, Request, Uri};
use hyper_tls::HttpsConnector;
use image::GenericImageView;
use image::{io::Reader as ImageReader, DynamicImage,imageops::FilterType};
use serde::Serialize;
use palette::LinSrgb;
use num_cpus;

#[derive(Serialize, Debug)]
struct Img {
    err: Option<String>,
    rgb: String,
}

async fn fix_url(url: &str) -> String {
    if url.starts_with("http://") || url.starts_with("https://") {
        url.to_string()
    } else {
        format!("http://{}", url)
    }
}

async fn download_image_and_parse(
    url: &str,
) -> Result<DynamicImage, Box<dyn std::error::Error + Send + Sync>> {
    // 将URL解析为Uri类型
    let url = fix_url(url).await;
    let uri = url.parse::<Uri>()?;

    // 创建一个新的hyper客户端
    let https = HttpsConnector::new();
    let client = Client::builder().build::<_, hyper::Body>(https);

    // 创建一个新的请求
    let request = Request::builder()
        .uri(uri)
        .header("User-Agent", "Mozilla/5.0")
        .body(Body::empty())?;

    // 发送请求并等待响应
    let response = client.request(request).await?;
    if response.status() == 404 {
        return Err(Box::new(io::Error::new(
            io::ErrorKind::NotFound,
            "图片不存在",
        )));
    }
    // 从响应中提取字节流
    let bytes = hyper::body::to_bytes(response.into_body()).await?;

    // 使用image库解析字节流中的图像
    let img = ImageReader::new(std::io::Cursor::new(bytes))
        .with_guessed_format()?
        .decode()?;

    Ok(img)
}

#[tokio::main]
async fn main() {
    dotenv().ok();
    let app = Router::new().route("/api", get(api));

    // run it with hyper
    let host = format!(
        "0.0.0.0:{}",
        match env::var("PORT") {
            Ok(s) => {
                if s.is_empty() {
                    "3000".to_string()
                } else {
                    s
                }
            }
            Err(_e) => "3000".to_string(),
        }
    );
    println!("服务将会运行在 {}", host);
    axum::Server::bind(&host.parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}
async fn api(Query(img): Query<HashMap<String, String>>) -> (StatusCode, Json<Img>) {
    if !img.contains_key("img") {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(Img {
                err: Some("请正确的传入参数".to_string()),
                rgb: "".to_string(),
            }),
        );
    }
    let url = img["img"].as_str();
    if url.is_empty() {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(Img {
                err: Some("请正确的传入参数".to_string()),
                rgb: "".to_string(),
            }),
        );
    }
    let img: DynamicImage;
    match download_image_and_parse(url).await {
        Ok(i) => {
            img = i;
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(Img {
                    err: Some(e.to_string()),
                    rgb: "".to_string(),
                }),
            );
        }
    };
    let r = Img {
        err: None,
        rgb: get_theme_color(&img).await,
    };
    (StatusCode::OK, Json(r))
}

/*
async fn get_theme_color(img: &DynamicImage) -> String {
    let img= img.resize(50, (img.height()*50)/img.width(), FilterType::Lanczos3);
    // Get the image dimensions
    let (width, height) = img.dimensions();
    // Calculate the average color of the image
    let mut sum_red: u32 = 0;
    let mut sum_green: u32 = 0;
    let mut sum_blue: u32 = 0;

    for x in 0..width {
        for y in 0..height {
            let pixel = img.get_pixel(x, y);
            sum_red += pixel[0] as u32;
            sum_green += pixel[1] as u32;
            sum_blue += pixel[2] as u32;
        }
    }

    let pixel_count = (width * height) as f32;
    let avg_red = (sum_red as f32 / pixel_count).round() as u8;
    let avg_green = (sum_green as f32 / pixel_count).round() as u8;
    let avg_blue = (sum_blue as f32 / pixel_count).round() as u8;

    // Create a palette color from the average color
    let avg_color = LinSrgb::new(
        avg_red as f32 / 255.0,
        avg_green as f32 / 255.0,
        avg_blue as f32 / 255.0,
    );

    // Convert the color to hexadecimal format
    format!("#{:X}", avg_color.into_format::<u8>())
}
*/
async fn get_theme_color(img: &DynamicImage) -> String {
    // Resize the image to 50 pixels width
    let img = img.resize(50, (img.height() * 50) / img.width(), FilterType::Lanczos3);

    // Get the image dimensions
    let (width, height) = img.dimensions();

    // Calculate the sum of RGB values of each pixel in parallel
    let sum_rgb = Arc::new(Mutex::new((0u32, 0u32, 0u32)));
    let pixels_per_thread = (width * height) / num_cpus::get() as u32;
    let mut handles = Vec::new();

    for tid in 0..num_cpus::get() {
        let sum_rgb = Arc::clone(&sum_rgb);
        let img = img.clone();
        let start = tid * pixels_per_thread as usize;
        let end = if tid == num_cpus::get() - 1 {
            width * height
        } else {
            (tid + 1) as u32 * pixels_per_thread
        };
        let handle = thread::spawn(move || {
            let mut sum_red = 0u32;
            let mut sum_green = 0u32;
            let mut sum_blue = 0u32;

            for p in start..end as usize {
                let x = p % width as usize;
                let y = p / width as usize;
                let pixel = img.get_pixel(x as u32, y as u32);
                sum_red += pixel[0] as u32;
                sum_green += pixel[1] as u32;
                sum_blue += pixel[2] as u32;
            }

            let mut sum_rgb = sum_rgb.lock().unwrap();
            sum_rgb.0 += sum_red;
            sum_rgb.1 += sum_green;
            sum_rgb.2 += sum_blue;
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    // Calculate the average RGB value
    let pixel_count = (width * height) as f32;
    let sum_rgb = sum_rgb.lock().unwrap();
    let avg_red = (sum_rgb.0 as f32 / pixel_count).round() as u8;
    let avg_green = (sum_rgb.1 as f32 / pixel_count).round() as u8;
    let avg_blue = (sum_rgb.2 as f32 / pixel_count).round() as u8;

    // Create a palette color from the average color
    let avg_color = LinSrgb::new(
        avg_red as f32 / 255.0,
        avg_green as f32 / 255.0,
        avg_blue as f32 / 255.0,
    );

    // Convert the color to hexadecimal format
    format!("#{:X}", avg_color.into_format::<u8>())
}
