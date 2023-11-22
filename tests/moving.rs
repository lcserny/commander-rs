#[cfg(test)]
mod tests {

    use std::{
        fs,
        path::{Path, PathBuf},
        sync::Arc,
    };

    use axum::{extract::State, Extension, Json};
    use commander::{
        db::DbClient,
        http::ApiContext,
        moving::{move_media, MediaMoveReq, EPISODE_SEGMENT_REGEX, SUBS_DIR},
        rename::MediaFileType,
        search::MediaFileGroup,
        tests::{create_file, create_test_settings, EmptyDb},
    };
    use regex::Regex;

    #[tokio::test]
    async fn moving_existing_movie_errors() {
        let settings = create_test_settings();

        let name = "some movie";
        let path = PathBuf::from(&settings.filesystem.downloads_path).join("doesnt matter");
        let file = "myMovie.mp4";

        fs::create_dir_all(PathBuf::from(&settings.filesystem.movies_path).join(name)).unwrap();
        create_file(path.join(file), 6);

        let media = MediaFileGroup {
            path: path.to_string_lossy().into_owned(),
            name: name.to_owned(),
            videos: vec![file.to_owned()],
        };

        let regex = Arc::new(Regex::new(EPISODE_SEGMENT_REGEX).unwrap());
        let db_client = DbClient::new(Arc::new(EmptyDb));
        let ctx = ApiContext {
            settings: Arc::new(settings),
            db_client,
        };
        let req = MediaMoveReq {
            file_group: media,
            media_type: MediaFileType::MOVIE,
        };

        let resp = move_media(State(regex), Extension(ctx), Json(req)).await;

        assert_eq!(1, resp.len());
        assert!(path.join(file).is_file());
    }

    #[tokio::test]
    async fn moving_existing_tv_merges() {
        let settings = create_test_settings();

        let name = "some show";
        let path = PathBuf::from(&settings.filesystem.downloads_path).join("doesnt matter");
        let file = "myShow.mp4";

        let p = PathBuf::from(&settings.filesystem.tv_path).join(name);
        fs::create_dir_all(&p).unwrap();
        create_file(path.join(file), 6);

        let media = MediaFileGroup {
            path: path.to_string_lossy().into_owned(),
            name: name.to_owned(),
            videos: vec![file.to_owned()],
        };

        let regex = Arc::new(Regex::new(EPISODE_SEGMENT_REGEX).unwrap());
        let db_client = DbClient::new(Arc::new(EmptyDb));
        let ctx = ApiContext {
            settings: Arc::new(settings),
            db_client,
        };
        let req = MediaMoveReq {
            file_group: media,
            media_type: MediaFileType::TV,
        };

        let resp = move_media(State(regex), Extension(ctx), Json(req)).await;

        assert_eq!(0, resp.len());
        assert!(!path.join(file).is_file());
        assert!(p.join(file).is_file());
    }

    #[tokio::test]
    async fn moving_from_downloads_root_doesnt_clean() {
        let settings = create_test_settings();

        let random_file = "someFile.txt";
        create_file( PathBuf::from(&settings.filesystem.downloads_path).join(random_file), 1,);

        let name = "some movieeee";
        let path = PathBuf::from(&settings.filesystem.downloads_path);
        let file = "mooveeee.mp4";

        create_file(path.join(file), 6);

        let media = MediaFileGroup {
            path: path.to_string_lossy().into_owned(),
            name: name.to_owned(),
            videos: vec![file.to_owned()],
        };

        let regex = Arc::new(Regex::new(EPISODE_SEGMENT_REGEX).unwrap());
        let db_client = DbClient::new(Arc::new(EmptyDb));
        let settings = Arc::new(settings);
        let ctx = ApiContext { settings: settings.clone(), db_client, };
        let req = MediaMoveReq { file_group: media, media_type: MediaFileType::MOVIE, };

        let resp = move_media(State(regex), Extension(ctx), Json(req)).await;

        assert_eq!(0, resp.len());
        assert!(!path.join(file).is_file());
        assert!(Path::new(&settings.filesystem.movies_path) .join(name) .join(file) .is_file());
        assert!(Path::new(&settings.filesystem.downloads_path) .join(random_file) .is_file());
    }

    #[tokio::test]
    async fn root_downloads_sub_skip() {
        let settings = create_test_settings();

        let name = "some movie with sub";
        let path = PathBuf::from(&settings.filesystem.downloads_path);
        let file = "mivi.mp4";
        create_file(path.join(file), 6);

        let sub = "mySub.srt";
        create_file(path.join(sub), 1,);

        let media = MediaFileGroup {
            path: path.to_string_lossy().into_owned(),
            name: name.to_owned(),
            videos: vec![file.to_owned()],
        };

        let regex = Arc::new(Regex::new(EPISODE_SEGMENT_REGEX).unwrap());
        let db_client = DbClient::new(Arc::new(EmptyDb));
        let settings = Arc::new(settings);
        let ctx = ApiContext { settings: settings.clone(), db_client, };
        let req = MediaMoveReq { file_group: media, media_type: MediaFileType::MOVIE, };

        let resp = move_media(State(regex), Extension(ctx), Json(req)).await;

        assert_eq!(0, resp.len());
        assert!(path.join(sub).is_file());
    }

    #[tokio::test]
    async fn movie_subs_moved_to_dest() {
        let settings = create_test_settings();

        let name = "some movie2 with sub";
        let path = PathBuf::from(&settings.filesystem.downloads_path).join(name);
        let file = "mivi2.mp4";
        create_file(path.join(file), 6);

        let sub = "sub.srt";
        create_file(path.join(sub), 1,);

        let media = MediaFileGroup {
            path: path.to_string_lossy().into_owned(),
            name: name.to_owned(),
            videos: vec![file.to_owned()],
        };

        let regex = Arc::new(Regex::new(EPISODE_SEGMENT_REGEX).unwrap());
        let db_client = DbClient::new(Arc::new(EmptyDb));
        let settings = Arc::new(settings);
        let ctx = ApiContext { settings: settings.clone(), db_client, };
        let req = MediaMoveReq { file_group: media, media_type: MediaFileType::MOVIE, };

        let resp = move_media(State(regex), Extension(ctx), Json(req)).await;

        assert_eq!(0, resp.len());
        assert!(!path.join(sub).is_file());
        assert!(Path::new(&settings.filesystem.movies_path).join(name).join(sub).is_file());
    }

    #[tokio::test]
    async fn tv_subs_moved_to_subs_folder() {
        let settings = create_test_settings();

        let name = "some show";
        let path = PathBuf::from(&settings.filesystem.downloads_path).join(name);
        let file = "show.mp4";
        create_file(path.join(file), 6);

        let sub = "showSub.srt";
        create_file(path.join(sub), 1,);

        let media = MediaFileGroup {
            path: path.to_string_lossy().into_owned(),
            name: name.to_owned(),
            videos: vec![file.to_owned()],
        };

        let regex = Arc::new(Regex::new(EPISODE_SEGMENT_REGEX).unwrap());
        let db_client = DbClient::new(Arc::new(EmptyDb));
        let settings = Arc::new(settings);
        let ctx = ApiContext { settings: settings.clone(), db_client, };
        let req = MediaMoveReq { file_group: media, media_type: MediaFileType::TV };

        let resp = move_media(State(regex), Extension(ctx), Json(req)).await;

        assert_eq!(0, resp.len());
        assert!(!path.join(sub).is_file());
        assert!(Path::new(&settings.filesystem.tv_path).join(name).join(SUBS_DIR).join(sub).is_file());
    }

    #[tokio::test]
    async fn nested_tv_subs_move() {
        let settings = create_test_settings();

        let name = "some show33";
        let path = PathBuf::from(&settings.filesystem.downloads_path).join(name);
        let file = "show.mp4";
        create_file(path.join(file), 6);

        let sub = "showSub.srt";
        let subdir = "show.s02e12.1080p";
        create_file(path.join(subdir).join(sub), 1,);

        let media = MediaFileGroup {
            path: path.to_string_lossy().into_owned(),
            name: name.to_owned(),
            videos: vec![file.to_owned()],
        };

        let regex = Arc::new(Regex::new(EPISODE_SEGMENT_REGEX).unwrap());
        let db_client = DbClient::new(Arc::new(EmptyDb));
        let settings = Arc::new(settings);
        let ctx = ApiContext { settings: settings.clone(), db_client, };
        let req = MediaMoveReq { file_group: media, media_type: MediaFileType::TV };

        let resp = move_media(State(regex), Extension(ctx), Json(req)).await;

        assert_eq!(0, resp.len());
        assert!(!path.join(subdir).join(sub).is_file());
        assert!(Path::new(&settings.filesystem.tv_path).join(name).join(SUBS_DIR)
            .join(format!("{}.{}", subdir, sub)).is_file());
    }

}
