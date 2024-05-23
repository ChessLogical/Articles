#[macro_use] extern crate rocket;
#[macro_use] extern crate log;

use rocket::response::content::RawHtml;
use rocket::serde::{Serialize, Deserialize};
use rocket::State;
use sled::Db;
use std::sync::Arc;
use rocket::data::Data;
use rocket::http::ContentType;
use rocket::response::Redirect;
use rocket_multipart_form_data::{MultipartFormDataOptions, MultipartFormDataField, MultipartFormData, mime};
use rocket::http::Status;
use infer;
use simplelog::{CombinedLogger, WriteLogger, Config, LevelFilter};
use std::fs::OpenOptions;
use tokio::fs;
use chrono::Utc;
use uuid::Uuid;

#[derive(FromForm, Serialize, Deserialize, Clone)]
struct MessageForm {
    title: String,
    message: String,
}

#[derive(Serialize, Deserialize, Clone)]
struct Post {
    title: String,
    message: String,
    directory: String,
    media: Option<String>,
    timestamp: i64,
}

struct AppState {
    db: Arc<Db>,
    admin_password: String,
}

impl AppState {
    fn new(db: Arc<Db>, admin_password: String) -> Self {
        AppState { db, admin_password }
    }

    fn save_post(&self, post: &Post) -> sled::Result<()> {
        let key = post.directory.as_bytes();
        let value = serde_json::to_vec(post).map_err(|e| sled::Error::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;
        self.db.insert(key, value)?;
        Ok(())
    }

    fn get_posts(&self) -> sled::Result<Vec<Post>> {
        let mut posts = Vec::new();
        for result in self.db.iter() {
            let (_, value) = result?;
            let post: Post = serde_json::from_slice(&value).map_err(|e| sled::Error::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;
            posts.push(post);
        }
        Ok(posts)
    }

    fn get_post(&self, directory: &str) -> Option<Post> {
        self.db.get(directory.as_bytes()).ok().flatten().and_then(|v| serde_json::from_slice(&v).ok())
    }

    fn delete_post(&self, directory: &str) -> sled::Result<()> {
        self.db.remove(directory.as_bytes())?;
        Ok(())
    }

    fn title_exists(&self, title: &str) -> bool {
        self.get_posts().unwrap().iter().any(|post| post.title == title)
    }
}

fn load_template(name: &str) -> String {
    std::fs::read_to_string(format!("templates/{}.html", name)).expect("Template file not found")
}

fn generate_unique_id() -> String {
    let timestamp = Utc::now().timestamp();
    let random_id = Uuid::new_v4();
    format!("{}-{}", timestamp, random_id)
}

#[get("/")]
async fn index(state: &State<AppState>) -> RawHtml<String> {
    let posts = state.get_posts().unwrap();
    let mut titles = String::new();
    let colors = vec![
        "#1B263B", "#3E92CC", "#4E9F3D", "#FF6700", "#6A0572",
        "#FFB400", "#A5FFD6", "#F72585", "#7209B7", "#3A86FF",
        "#8338EC", "#FB5607", "#FF006E", "#EF476F", "#06D6A0"
    ];

    for (index, post) in posts.iter().rev().enumerate() {
        let color = colors[index % colors.len()];
        titles.push_str(&format!(
            r#"<div class="post-title">
                <a href="/articles/{}/" class="title-button" style="background-color:{};">{}</a>
            </div>"#,
            post.directory,
            color,
            html_escape::encode_text(&post.title)
        ));
    }

    let template = load_template("index");
    let content = template.replace("{{titles}}", &titles);

    RawHtml(content)
}

#[get("/new_post")]
fn new_post_form() -> RawHtml<String> {
    let template = load_template("post_form");
    RawHtml(template)
}

fn generate_post_html(title: &str, message: &str, media_path: Option<&str>, delete_option: &str) -> String {
    let template = load_template("post");
    let media_html = if let Some(path) = media_path {
        match path.split('.').last().unwrap_or("") {
            "mp4" => format!(r#"<div class="media-container"><video controls><source src="{}" type="video/mp4"></video></div>"#, path),
            "mp3" => format!(r#"<div class="media-container"><audio controls><source src="{}" type="audio/mp3"></audio></div>"#, path),
            _ => format!(r#"<div class="media-container"><img src="{}" alt="Image" style="max-width: 400px; max-height: 400px; width: auto; height: auto;" /></div>"#, path),
        }
    } else {
        String::new()
    };

    template.replace("{{title}}", &html_escape::encode_text(title))
            .replace("{{message}}", &html_escape::encode_text(message))
            .replace("{{media}}", &media_html)
            .replace("{{delete_option}}", delete_option)
}

#[post("/submit", data = "<data>")]
async fn submit(content_type: &ContentType, data: Data<'_>, state: &State<AppState>) -> Result<Redirect, Status> {
    let mut options = MultipartFormDataOptions::new();
    options.allowed_fields.push(MultipartFormDataField::text("title"));
    options.allowed_fields.push(MultipartFormDataField::text("message"));
    options.allowed_fields.push(MultipartFormDataField::file("media")
        .content_type_by_string(Some(mime::IMAGE_STAR)).unwrap()
        .content_type_by_string(Some("video/mp4")).unwrap()
        .content_type_by_string(Some("audio/mpeg")).unwrap());

    let multipart_form_data = match MultipartFormData::parse(content_type, data, options).await {
        Ok(data) => data,
        Err(e) => {
            error!("Failed to parse multipart form data: {:?}", e);
            return Ok(Redirect::to(uri!(error("Failed to parse multipart form data"))));
        }
    };

    let title = match multipart_form_data.texts.get("title") {
        Some(field) => field[0].text.to_string(),
        None => {
            return Ok(Redirect::to(uri!(error("Title field is missing"))));
        }
    };

    let message = match multipart_form_data.texts.get("message") {
        Some(field) => field[0].text.to_string(),
        None => {
            return Ok(Redirect::to(uri!(error("Message field is missing"))));
        }
    };

    let media_field = multipart_form_data.files.get("media");

    if state.title_exists(&title) {
        return Ok(Redirect::to(uri!(error("Title already exists"))));
    }

    let unique_id = generate_unique_id();
    let directory = format!("articles/{}", unique_id);
    let filename = format!("{}/index.html", directory);

    if let Err(e) = fs::create_dir_all(&directory).await {
        error!("Failed to create directory: {:?}", e);
        return Ok(Redirect::to(uri!(error("Failed to create directory"))));
    }

    let media_path = if let Some(media_field) = media_field {
        let temp_path = &media_field[0].path;
        let metadata = match fs::metadata(temp_path).await {
            Ok(metadata) => metadata,
            Err(e) => {
                error!("Failed to read media file metadata: {:?}", e);
                return Ok(Redirect::to(uri!(error("Failed to read media file metadata"))));
            }
        };

        if metadata.len() > 20 * 1024 * 1024 {
            error!("File size exceeds 20 MB: {}", metadata.len());
            return Ok(Redirect::to(uri!(error("File size exceeds 20 MB"))));
        }

        let content = match fs::read(temp_path).await {
            Ok(content) => content,
            Err(e) => {
                error!("Failed to read media file: {:?}", e);
                return Ok(Redirect::to(uri!(error("Failed to read media file"))));
            }
        };
        let kind = match infer::get(&content) {
            Some(kind) => kind,
            None => {
                error!("Failed to detect media file type");
                return Ok(Redirect::to(uri!(error("Failed to detect media file type"))));
            }
        };

        if kind.mime_type() == "image/jpeg" || kind.mime_type() == "image/png" ||
           kind.mime_type() == "image/gif" || kind.mime_type() == "image/webp" ||
           kind.mime_type() == "image/bmp" || kind.mime_type() == "video/mp4" ||
           kind.mime_type() == "audio/mpeg" {
            let extension = kind.extension();
            let media_filename = format!("{}/media.{}", directory, extension);
            if let Err(e) = fs::rename(temp_path, &media_filename).await {
                error!("Failed to rename media file: {:?}", e);
                return Ok(Redirect::to(uri!(error("Failed to rename media file"))));
            }
            Some(format!("media.{}", extension))
        } else {
            error!("Invalid media file type: {}", kind.mime_type());
            return Ok(Redirect::to(uri!(error("Invalid media file type"))));
        }
    } else {
        None
    };

    let new_post = Post {
        title: title.clone(),
        message: message.clone(),
        directory: unique_id.clone(),
        media: media_path.clone(),
        timestamp: Utc::now().timestamp(),
    };
    state.save_post(&new_post).unwrap();

    let delete_option = format!(
        r#"<div style="text-align: right; margin-top: 20px;">
            <form action="/delete_post" method="post">
                <input type="hidden" name="directory" value="{}">
                <button type="submit" style="background-color: red; color: white; padding: 5px 10px; border: none; border-radius: 3px; cursor: pointer;">Delete</button>
            </form>
           </div>"#,
        unique_id
    );
    let post_html = generate_post_html(&title, &message, media_path.as_deref(), &delete_option);
    if let Err(e) = fs::write(&filename, post_html).await {
        error!("Failed to write post HTML file: {:?}", e);
        return Ok(Redirect::to(uri!(error("Failed to write post HTML file"))));
    }

    Ok(Redirect::to(uri!(show_post(directory = unique_id))))
}

#[post("/delete_post", data = "<form>")]
async fn delete_post(form: rocket::form::Form<std::collections::HashMap<String, String>>, state: &State<AppState>) -> Result<Redirect, Status> {
    let directory = match form.get("directory") {
        Some(dir) => dir,
        None => return Ok(Redirect::to(uri!(error("Directory field is missing")))),
    };

    if let Some(post) = state.get_post(directory) {
        if Utc::now().timestamp() - post.timestamp <= 120 {
            if state.delete_post(directory).is_ok() {
                let path = format!("articles/{}", directory);
                if fs::remove_dir_all(&path).await.is_ok() {
                    info!("Post deleted successfully within 2 minutes: {}", directory);
                    return Ok(Redirect::to(uri!(index)));
                }
            }
        }
    }

    error!("Failed to delete post: {}", directory);
    Ok(Redirect::to(uri!(error("Failed to delete post"))))
}

#[get("/articles/<directory>")]
async fn show_post(directory: String, state: &State<AppState>) -> RawHtml<String> {
    if let Some(post) = state.get_post(&directory) {
        let delete_option = if Utc::now().timestamp() - post.timestamp <= 120 {
            format!(
                r#"<div style="text-align: right; margin-top: 20px;">
                    <form action="/delete_post" method="post">
                        <input type="hidden" name="directory" value="{}">
                        <button type="submit" style="background-color: red; color: white; padding: 5px 10px; border: none; border-radius: 3px; cursor: pointer;">Delete</button>
                    </form>
                   </div>"#,
                directory
            )
        } else {
            String::new()
        };
        let content = fs::read_to_string(format!("articles/{}/index.html", directory))
            .await
            .unwrap_or_else(|_| generate_post_html(&post.title, &post.message, post.media.as_deref(), &delete_option));

        RawHtml(content)
    } else {
        RawHtml(load_template("error").replace("{{error_message}}", "Post not found"))
    }
}

#[get("/favicon.gif")]
async fn favicon() -> Option<rocket::fs::NamedFile> {
    rocket::fs::NamedFile::open("favicon.gif").await.ok()
}

#[get("/error/<msg>")]
fn error(msg: String) -> RawHtml<String> {
    let template = load_template("error");
    let content = template.replace("{{error_message}}", &msg);
    RawHtml(content)
}

#[launch]
fn rocket() -> _ {
    CombinedLogger::init(
        vec![
            WriteLogger::new(
                LevelFilter::Error,
                Config::default(),
                OpenOptions::new().append(true).create(true).open("error.log").unwrap(),
            ),
        ]
    ).unwrap();

    let db = Arc::new(sled::open("my_db").expect("Failed to open sled database"));
    let admin_password = "default_admin_password".to_string();
    let app_state = AppState::new(db, admin_password);

    rocket::build()
        .mount("/", routes![index, new_post_form, submit, delete_post, favicon, show_post, error])
        .manage(app_state)
}
