use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};
use chrono::{DateTime, Utc};


#[derive(Serialize, Deserialize, Debug)]
struct Episode {
    title: String,
    date: DateTime<Utc>,
    slug: Option<String>,
    file: String,
    duration: String,
    length: String,
    reddit: Option<String>,

    #[serde(default = "empty_string")]
    body: String,
}

fn empty_string() -> String {
    String::new()
}

fn main() {

    let site = PathBuf::from("_site");
    fs::create_dir_all(&site).expect(format!("Failed to create {site:?} directory").as_str());
    for file in ["style.css", "404.html", "robots.txt"] {
        //let path = format!("src/{file}");
        //let path = PathBuf::from(path);
        fs::copy(file, site.join(file)).expect(format!("Failed to copy {file} to site directory").as_str());
    }
    // fs::copy("index.html", site.join("index.html"))
    //     .expect("Failed to copy index.html to site directory");

    let episodes = load_episodes();
    print!("{} episodes loaded", episodes.len());
}

fn load_episodes() -> Vec<Episode> {
    let mut episodes = vec![];
    let serieses = fs::read_dir("_episodes").expect("Failed to read _episodes directory");
    for series in serieses {
        let series = series.expect("Failed to read entry");
        //println!("{entry:?}");
        let entries = fs::read_dir(series.path()).expect("Failed to read directory");
        for entry in entries {
            let entry = entry.expect("Failed to read entry");
            //println!("{entry:?}");
            let path = entry.path();
            let extension = path
                .extension()
                .unwrap_or_else(|| panic!("Failed to get extension: {path:?}"));
            if extension != "md" {
                panic!("Not a markdown file: {}", path.display());
            }

            //println!("Found episode: {}", path.display());
            let content = fs::read_to_string(&path).expect("Failed to read file");
            if !content.starts_with("---\n") {
                panic!("File does not start with '---': {}", path.display());
            }
            let index: Option<usize> = content[4..].find("---").map(|i| i + 4);
            let index = index.unwrap_or_else(|| {
                panic!(
                    "File does not contain the second '---', the end of the front-matter : {path:?}"
                )
            });
            let mut episode = match serde_yml::from_str::<Episode>(&content[4..index]) {
                Ok(front_matter) => front_matter,
                Err(err) => panic!("Failed to parse front matter: {err} in {path:?}"),
            };
            episode.body = content[index + 4..].to_string();
            // if episode.slug.is_none() {
            //     eprintln!("Missing slug in front matter: {path:?}");
            // }
            // if episode.reddit.is_none() {
            //     eprintln!("Missing reddit in front matter: {path:?}");
            // }
            episodes.push(episode);
        }
    }

    episodes
}
