use std::collections::HashMap;
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

    #[serde(default = "empty_path")]
    path: PathBuf,

    #[serde(default = "empty_string")]
    body: String,
}

fn empty_path() -> PathBuf {
    PathBuf::new()
}

fn empty_string() -> String {
    String::new()
}

const ABNORMAL_DASH: char = '⁃';
const SMART_QUOTES: [char; 4] = ['“', '‘', '’', '”'];

fn main() -> Result<()> {
    env_logger::init();

    let site = PathBuf::from("_site");
    fs::create_dir_all(&site).expect(format!("Failed to create {site:?} directory").as_str());
    remove_content_of_site_directory(&site)?;
    copy_static_files(&site)?;

    let episodes = load_episodes("_episodes")?;
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

fn load_episodes(path: &str) -> Result<Vec<Episode>> {
    let mut episodes = vec![];
    let serieses = fs::read_dir(path).expect("Failed to read {path} directory");
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

        // No duplicate URLs
        let mut files: HashMap<String, PathBuf> = HashMap::new();
        for episode in &episodes {
            let mp3 = episode.file.clone();
            if files.contains_key(&mp3) {
                bail!(
                    "The same mp3 file {} was used twice in {} and in {}",
                    mp3,
                    files.get(&mp3).unwrap().display(),
                    episode.path.display()
                );
            }
            files.insert(episode.file.clone(), episode.path.clone());
        }

        // No duplicate slugs
        // For collections, jekyll _only_ uses the basename (without date) of each
        // post for the slug, and doesn't error on duplicates. So we must check.
        // get the part of the filename after the date, that is the slug
        // _episodes/*/????-??-??-$slug)
        // bail!("Duplicate slugs found: ${files[*]}")
    }

    Ok(episodes)
}

fn load_episode(path: &PathBuf) -> Result<Episode> {
    log::debug!("Load episode: {}", path.display());
    let content = fs::read_to_string(&path)?;
    log::debug!("Loading episode: {}", content);
    if content.contains(ABNORMAL_DASH) {
        bail!("Abnormal dash found in: {}", path.display());
    }
    for quote in SMART_QUOTES {
        if content.contains(quote) {
            bail!(
                "Smart quote found in: {}. Please replace it with a normal quote.",
                path.display()
            );
        }
    }
    // timecodes should never start a line (should be in header or list)
    // '^\[@' "$episode"
    // bail!("Timecode not in list or header of {}", path.display()

    // timecode listings need to not have empty lines, or we'll get
    //
    //   <li><p>[@HH:MM:SS]
    //
    // which doesn't render right. it happens to work for timecode
    // listings that have sub-listings, but easiest to check that there
    // just aren't any gaps.

    //  if ! awk '/^\s*$/ { empty = 1; next; } /^\s*-\s*\[@[0-9]/ { if (in_list == 1 && empty == 1) { exit 1; } else { in_list = 1; empty = 0; next; } } /^\s*-/ { empty = 0; next; } { in_list = 0; empty = 0; }' "$episode"; then
    // bail!("Empty lines between list items in {}", path.display()

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
    episode.path = path.to_owned();
    episode.body = content[index + 4..].to_string();

    Ok(episode)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_episodes() {
        let episodes = load_episodes("_episodes").unwrap();
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
                assert_eq!(
                    err.to_string(),
                    "File does not contain the second '---', the end of the front-matter : \"test_cases/missing_end_of_header.md\""
                );
            }
        }
    }

    #[test]
    fn test_abnormal_dash() {
        let episode = load_episode(&PathBuf::from("test_cases/abnormal_dash.md"));
        match episode {
            Ok(_) => panic!("Expected error loading file with abnormal dash"),
            Err(err) => {
                assert_eq!(
                    err.to_string(),
                    "Abnormal dash found in: test_cases/abnormal_dash.md"
                )
            }
        }
    }

    #[test]
    fn test_smart_quote() {
        let episode = load_episode(&PathBuf::from("test_cases/smart_quote_1.md"));
        match episode {
            Ok(_) => panic!("Expected error loading file with smart quote"),
            Err(err) => {
                assert_eq!(
                    err.to_string(),
                    "Smart quote found in: test_cases/smart_quote_1.md. Please replace it with a normal quote."
                )
            }
        }
    }

    #[test]
    fn test_duplicate_mp3_file() {
        let result = load_episodes("test_cases/duplicate_file");
        match result {
            Ok(_) => panic!("Expected error loading duplicate mp3 files"),
            Err(err) => {
                assert!(
                    err.to_string()
                        .starts_with("The same mp3 file https://blabla.mp3 was used twice")
                )
            }
        }
    }
}
