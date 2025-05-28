use std::{fs, path::PathBuf};

use anyhow::{Result, bail};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

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

fn main() -> Result<()> {
    env_logger::init();

    let site = PathBuf::from("_site");
    fs::create_dir_all(&site).expect(format!("Failed to create {site:?} directory").as_str());
    remove_content_of_site_directory(&site)?;
    copy_static_files(&site)?;

    let episodes = load_episodes()?;
    log::info!("{} episodes loaded", episodes.len());
    generate_html(&episodes)?;

    Ok(())
}

fn generate_html(episodes: &Vec<Episode>) -> Result<()> {
    for episode in episodes {
        log::debug!("Episode: {episode:?}");
        log::info!("Episode: {}", episode.title);
    }

    Ok(())
}

// Keep the folder itself so a static server can serve it without restarting
fn remove_content_of_site_directory(site: &PathBuf) -> Result<()> {
    if !site.exists() {
        return Ok(());
    }
    if !site.is_dir() {
        bail!("Site path is not a directory: {site:?}");
    }
    // Remove all files and directories in the _site directory
    for entry in fs::read_dir(site).expect("Failed to read _site directory") {
        let entry = entry.expect("Failed to read entry");
        let path = entry.path();
        if path.is_dir() {
            fs::remove_dir_all(&path)
                .expect(format!("Failed to remove directory: {path:?}").as_str());
        } else {
            fs::remove_file(&path).expect(format!("Failed to remove file: {path:?}").as_str());
        }
    }
    Ok(())
}

fn copy_static_files(site: &PathBuf) -> Result<()> {
    for file in ["style.css", "404.html", "robots.txt"] {
        fs::copy(file, site.join(file))
            .expect(format!("Failed to copy {file} to site directory").as_str());
    }
    copy_dir::copy_dir("images", site.join("images"))?;

    Ok(())
}

fn load_episodes() -> Result<Vec<Episode>> {
    let mut episodes = vec![];
    let serieses = fs::read_dir("_episodes").expect("Failed to read _episodes directory");
    for series in serieses {
        let series = series.expect("Failed to read entry");
        log::debug!("{series:?}");
        let entries = fs::read_dir(series.path()).expect("Failed to read directory");
        for entry in entries {
            let entry = entry.expect("Failed to read entry");
            log::debug!("{entry:?}");
            let path = entry.path();
            let extension = path
                .extension()
                .unwrap_or_else(|| panic!("Failed to get extension: {path:?}"));
            if extension != "md" {
                panic!("Not a markdown file: {}", path.display());
            }

            let episode = load_episode(&path)?;
            // if episode.slug.is_none() {
            //     eprintln!("Missing slug in front matter: {path:?}");
            // }
            // if episode.reddit.is_none() {
            //     eprintln!("Missing reddit in front matter: {path:?}");
            // }
            episodes.push(episode);
        }
    }

    Ok(episodes)
}

fn load_episode(path: &PathBuf) -> Result<Episode> {
    log::debug!("Load episode: {}", path.display());
    let content = fs::read_to_string(&path)?;
    log::debug!("Loading episode: {}", content);
    if !content.starts_with("---\n") {
        bail!("File does not start with '---': {}", path.display());
    }
    let index: Option<usize> = content[4..].find("---").map(|i| i + 4);
    let index = match index {
        Some(index) => index,
        None => {
            bail!("File does not contain the second '---', the end of the front-matter : {path:?}")
        }
    };
    log::debug!("Raw Front-matter: {}", &content[4..index]);
    let mut episode = match serde_yml::from_str::<Episode>(&content[4..index]) {
        Ok(front_matter) => front_matter,
        Err(err) => bail!("Failed to parse front matter: {err} in {path:?}"),
    };
    episode.body = content[index + 4..].to_string();

    Ok(episode)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_episodes() {
        let episodes = load_episodes().unwrap();
        assert!(!episodes.is_empty(), "No episodes loaded");
        for episode in &episodes {
            assert!(!episode.title.is_empty(), "Episode title is empty");
            assert!(!episode.file.is_empty(), "Episode file is empty");
            assert!(!episode.duration.is_empty(), "Episode duration is empty");
            assert!(!episode.length.is_empty(), "Episode length is empty");
        }
    }

    #[test]
    fn test_load_missing_file() {
        let episode = load_episode(&PathBuf::from("test_cases/blabla.md"));
        match episode {
            Ok(_) => panic!("Expected error loading missing file"),
            Err(err) => {
                assert_eq!(err.to_string(), "No such file or directory (os error 2)");
            }
        }
    }

    #[test]
    fn test_load_empty() {
        let episode = load_episode(&PathBuf::from("test_cases/empty.md"));
        match episode {
            Ok(_) => panic!("Expected error loading empty file"),
            Err(err) => {
                assert_eq!(
                    err.to_string(),
                    "File does not start with '---': test_cases/empty.md"
                );
            }
        }
    }

    #[test]
    fn test_load_invalid_date() {
        let episode = load_episode(&PathBuf::from("test_cases/invalid_date.md"));
        match episode {
            Ok(_) => panic!("Expected error loading file with invalid date"),
            Err(err) => {
                assert_eq!(
                    err.to_string(),
                    "Failed to parse front matter: date: input is out of range at line 2 column 7 in \"test_cases/invalid_date.md\""
                );
            }
        }
    }

    #[test]
    fn test_missing_end_of_header() {
        let episode = load_episode(&PathBuf::from("test_cases/missing_end_of_header.md"));
        match episode {
            Ok(_) => panic!("Expected error loading file with invalid date"),
            Err(err) => {
                assert!(err.to_string().starts_with(
                    "File does not contain the second '---', the end of the front-matter"
                ),);
            }
        }
    }
}
