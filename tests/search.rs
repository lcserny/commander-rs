#[cfg(test)]
mod tests {
    use std::{path::PathBuf, sync::Arc};

    use axum::Extension;
    use commander::{
        db::DbClient,
        http::ApiContext,
        search::search_media,
        tests::{create_file, create_test_settings, EmptyDb},
    };

    /*
    [
      {
        "path": "/downloads/some movie folder",
        "name": "some movie folder", // this is showed in UI, used by rename
        "videos": [ // also shown in UI under, but you can't change these individually
          "video1.mp4"
        ]
      },
      {
        "path": "/downloads/some tv folder",
        "name": "some tv folder",
        "videos": [ // used by move, just resolve <path> to them
          "video1.mp4",
          "video2.mp4",
          "video3.mp4",
        ]
      },
      {
        "path": "/downloads/some nested folder", // easier to delete
        "name": "some nested folder", // notice the nested structure
        "videos": [
          "another folder/video1.mp4",
          "another folder/video2.mp4"
        ]
      },
      {
        "path": "/downloads", // notice no parent folder
        "name": "video5", // notice its generated from file name without extension
        "videos": [
          "video5.mp4",
        ]
      },
    ]
    */

    // FIXME: stil fails sometimes...
    #[tokio::test]
    async fn check_search_finds_correct_media() {
        let settings = create_test_settings();
        let db_client = DbClient::new(Arc::new(EmptyDb));

        let downloads_path = PathBuf::from(&settings.filesystem.downloads_path);
        create_file(downloads_path.join("video1.mp4"), 6);
        create_file(downloads_path.join(&settings.search.exclude_paths[0]).join("excluded.mp4"), 6);
        create_file(downloads_path.join("video3.mkv"), 6);
        create_file(downloads_path.join("small.mp4"), 0);
        create_file(downloads_path.join("nested folder/nested.mp4"), 6);
        create_file(downloads_path.join("1/2/3/4/5/deep.mp4"), 6);

        let ctx = ApiContext { settings: Arc::new(settings), db_client, };
        let mut videos = search_media(Extension(ctx)).await.unwrap();
        videos.sort_by(|v1, v2| v1.name.cmp(&v2.name));
        videos.iter_mut().for_each(|mfg| mfg.videos.sort_by(|v1, v2| v1.cmp(v2)));

        let downloads_str = downloads_path.to_string_lossy().into_owned();
        assert_eq!(3, videos.len());
        assert!(videos[0].path.contains("nested folder"));
        assert_eq!("nested folder", &videos[0].name);
        assert_eq!("nested.mp4", &videos[0].videos[0]);
        assert_eq!(downloads_str, videos[1].path);
        assert_eq!("video1.mp4", &videos[1].videos[0]);
        assert_eq!(downloads_str, videos[2].path);
        assert_eq!("video3.mkv", &videos[2].videos[0]);
    }
}
