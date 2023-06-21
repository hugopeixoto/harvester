use std::os::unix::fs::MetadataExt;
use regex::Regex;

#[cfg(test)]
mod test;


#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum MediaData {
    Movie { title: String, year: Option<u32> },
    ShowEpisode { name: String, season: u32, episode: u32 },
    Garbage,
}

pub struct ScannedFile {
    path: std::path::PathBuf,
    metadata: Option<MediaData>,
    inode: u64,
}

pub struct Analyzer {
    cleaner: Regex,
    title_season_episode: Regex,
    title_episode_dash: Regex,
    title_episode_quoted_name: Regex,
    title_episode: Regex,
    movie_year: Regex,
}

impl Analyzer {
    pub fn new() -> Self {
        let cleaner = Regex::new(r"([. _]*)\[[^]]+\]([. _]*)").unwrap();
        let title_season_episode = Regex::new(r"(.*) [sS](\d+)[eE](\d+) (.*)").unwrap();
        let title_episode_dash = Regex::new(r"^(.*) - (\d+)(v\d)?( END)?( .*)?$").unwrap();
        let title_episode_quoted_name = Regex::new(r"^(.*) [eE](\d+)( END)? '.*'?$").unwrap();
        let title_episode = Regex::new(r"^(.*) (\d+)( END)?( \((.*)\))?( v2)?$").unwrap();
        let movie_year = Regex::new(r"(.*[^-]) (\d{4})( [^-]|$)").unwrap();

        Self {
            cleaner,
            title_season_episode,
            title_episode_dash,
            title_episode_quoted_name,
            title_episode,
            movie_year,
        }
    }

    pub fn analyze_directory(&self, path: &std::path::PathBuf) -> Vec<ScannedFile> {
        println!("scanning {:?}...", path);

        let files = find_all_files(path)
            .iter()
            .map(|f| ScannedFile {
                path: f.clone(),
                metadata: self.analyze(f),
                inode: std::fs::metadata(f).unwrap().ino(),
            })
            .collect::<Vec<_>>();

        println!("found {} files.", files.len());
        files
    }


    pub fn analyze(&self, path: &std::path::PathBuf) -> Option<MediaData> {
        match path.extension().and_then(std::ffi::OsStr::to_str) {
            Some("mkv" | "mp4") => {
                let name = path.file_stem().unwrap().to_str().unwrap().to_lowercase();
                let name = self.cleaner.replace_all(&name, "");
                let name = name.replace("_", " ");
                let name = name.replace(".", " ");

                if let Some(x) = self.title_season_episode.captures(&name) {
                    Some(MediaData::ShowEpisode {
                        name: x.get(1).unwrap().as_str().to_string(),
                        season: x.get(2).unwrap().as_str().parse::<u32>().unwrap(),
                        episode: x.get(3).unwrap().as_str().parse::<u32>().unwrap(),
                    })
                } else if let Some(x) = self.title_episode_dash.captures(&name) {
                    Some(MediaData::ShowEpisode {
                        name: x.get(1).unwrap().as_str().to_string(),
                        season: 1,
                        episode: x.get(2).unwrap().as_str().parse::<u32>().unwrap(),
                    })
                } else if let Some(x) = self.title_episode_quoted_name.captures(&name) {
                    Some(MediaData::ShowEpisode {
                        name: x.get(1).unwrap().as_str().to_string(),
                        season: 1,
                        episode: x.get(2).unwrap().as_str().parse::<u32>().unwrap(),
                    })
                } else if let Some(x) = self.title_episode.captures(&name) {
                    Some(MediaData::ShowEpisode {
                        name: x.get(1).unwrap().as_str().to_string(),
                        season: 1,
                        episode: x.get(2).unwrap().as_str().parse::<u32>().unwrap(),
                    })
                } else if let Some(x) = self.movie_year.captures(&name) {
                    Some(MediaData::Movie {
                        title: x.get(1).unwrap().as_str().to_string(),
                        year: Some(x.get(2).unwrap().as_str().parse::<u32>().unwrap()),
                    })
                } else {
                    eprintln!("unknown filename pattern: {:?}", name);
                    None
                }
            }
            Some("srt" | "sub") => { Some(MediaData::Garbage) }
            Some("idx")         => { Some(MediaData::Garbage) }
            Some("ogg" | "mp3") => { Some(MediaData::Garbage) }
            Some("jpg" | "png") => { Some(MediaData::Garbage) }
            Some("ts" | "bdjo" | "clpi" | "mpls" | "m2ts" | "bdmv") => { Some(MediaData::Garbage) }
            Some("torrent" | "meta" | "exe" | "nfo" | "txt" | "md5") => { Some(MediaData::Garbage) }
            _ => { eprintln!("unknown extension: {:?}", path); None },
        }
    }
}

pub fn find_all_files_aux(path: &std::path::PathBuf, output: &mut Vec<std::path::PathBuf>) {
    if path.is_dir() {
        for subdir in path.read_dir().unwrap() {
            find_all_files_aux(&subdir.unwrap().path(), output);
        }
    } else {
        output.push(path.clone());
    }
}
pub fn find_all_files(path: &std::path::PathBuf) -> Vec<std::path::PathBuf> {
    let mut files = vec![];
    find_all_files_aux(path, &mut files);
    files
}

trait Runner {
    fn remove_dir(&self, path: &std::path::PathBuf);
    fn remove_file(&self, path: &std::path::PathBuf);
    fn create_dir_all(&self, path: &std::path::PathBuf);
    fn hard_link(&self, path: &std::path::PathBuf, link: &std::path::PathBuf);
}

struct RealRunner {}
impl Runner for RealRunner {
    fn remove_dir(&self, path: &std::path::PathBuf) {
        std::fs::remove_dir(path).unwrap();
    }
    fn remove_file(&self, path: &std::path::PathBuf) {
        std::fs::remove_file(path).unwrap();
    }
    fn create_dir_all(&self, path: &std::path::PathBuf) {
        std::fs::create_dir_all(path).unwrap();
    }
    fn hard_link(&self, original: &std::path::PathBuf, link: &std::path::PathBuf) {
        std::fs::hard_link(original, link).unwrap();
    }
}

struct DryRunner {}
impl Runner for DryRunner {
    fn remove_dir(&self, _path: &std::path::PathBuf) {}
    fn remove_file(&self, _path: &std::path::PathBuf) {}
    fn create_dir_all(&self, _path: &std::path::PathBuf) {}
    fn hard_link(&self, _original: &std::path::PathBuf, _link: &std::path::PathBuf) {}
}

fn create_links(runner: &dyn Runner, files: &Vec<ScannedFile>, target_dir: &std::path::PathBuf) -> Vec<(std::path::PathBuf, std::path::PathBuf)> {
    let mut links = vec![];

    for file in files.iter() {
        let link = match &file.metadata {
            Some(MediaData::ShowEpisode { name, season, episode }) => {
                target_dir
                    .join("shows")
                    .join(name)
                    .join(format!("Season {}", season))
                    .join(format!("episode {}.{}", episode, file.path.extension().unwrap().to_str().unwrap()))
            },
            Some(MediaData::Movie { title, year }) => {
                target_dir
                    .join("movies")
                    .join(match year {
                        Some(y) => format!("{} ({})", title, y),
                        None => title.to_string(),
                    })
                    .join(format!("movie.{}", file.path.extension().unwrap().to_str().unwrap()))
            },
            _ => { continue; }
        };

        if !link.exists() {
            println!("creating hard link: {:?}", link);
            links.push((file.path.clone(), link.clone()));
            runner.create_dir_all(&link.parent().unwrap().to_path_buf());
            runner.hard_link(&file.path, &link);
        }
    }

    links
}

fn remove_empty_directories(runner: &dyn Runner, path: &std::path::PathBuf) -> bool {
    let mut is_empty = true;
    for subdir in path.read_dir().unwrap() {
        let subpath = subdir.unwrap().path();
        if subpath.is_dir() {
            let sub_is_empty = remove_empty_directories(runner, &subpath);
            if sub_is_empty {
                println!("removing directory {:?}", subpath);
                runner.remove_dir(&subpath);
            } else {
                is_empty = false;
            }
        } else {
            is_empty = false;
        }
    }

    is_empty
}

fn remove_hardlinks(runner: &dyn Runner, source: &Vec<ScannedFile>, target_dir: &std::path::PathBuf) {
    let source_inodes = source.iter().map(|f| f.inode).collect::<std::collections::HashSet<_>>();

    for file in find_all_files(target_dir) {
        let inode = std::fs::metadata(&file).unwrap().ino();

        if source_inodes.contains(&inode) {
            println!("removing file {:?}", file);
            runner.remove_file(&file);
        } else {
            eprintln!("extra file found: {:?}", file);
        }
    }
}

fn main() {
    let args = std::env::args().collect::<Vec<_>>();

    if args.len() < 3 {
        eprintln!("usage: harvester <incoming> <jellyfin> [--dry]");
        return;
    }

    let incoming = std::path::PathBuf::from(&args[1]);
    let jellyfin = std::path::PathBuf::from(&args[2]);
    let dry_run = args.len() > 3 && args[3] == "--dry";

    let scanned_files = Analyzer::new().analyze_directory(&incoming);

    let runner: Box<dyn Runner> = if dry_run {
        Box::new(DryRunner {})
    } else {
        Box::new(RealRunner {})
    };

    remove_hardlinks(runner.as_ref(), &scanned_files, &jellyfin);
    create_links(runner.as_ref(), &scanned_files, &jellyfin);
    remove_empty_directories(runner.as_ref(), &jellyfin);
}
