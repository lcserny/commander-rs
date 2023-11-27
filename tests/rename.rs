#[cfg(test)]
mod name_tests {
    use std::sync::Arc;

    use commander::{tests::create_test_settings, rename::name::NameGenerator};

    fn check_normalized_formatted(input: &str, expected: &str) {
        let settings = Arc::new(create_test_settings());
        let generator = NameGenerator::new(settings);
        let normalized = generator.generate_base_info(input.to_owned());
        assert_eq!(expected.to_owned(), normalized.formatted());
    }

    #[test]
    fn check_prenormalized_origin_name() {
        check_normalized_formatted("   Some Movie (2021-10-12)", "Some Movie (2021)");
        check_normalized_formatted("   Another Movie (2020)", "Another Movie (2020)");
    }

    #[test]
    fn check_name_trim_regex_origin_name() {
        check_normalized_formatted("Bodyguard-S01-Series.1--BBC-2018-720p-w.subs-x265-HEVC", "Bodyguard");
        check_normalized_formatted("1922.1080p.[2017].x264", "1922");
    }

    #[test]
    fn check_replace_of_and_with_origin_name() {
        check_normalized_formatted("myMovie & me", "MyMovie And Me");
    }

    #[test]
    fn check_replace_of_special_chars_origin_name() {
        check_normalized_formatted(" hello__Sai***", "Hello Sai");
    }

    #[test]
    fn check_trim_and_spaces_are_merged_origin_name() {
        check_normalized_formatted("  Gnarly   Feels Move ", "Gnarly Feels Move");
    }

    #[test]
    fn check_capitalized_origin_name() {
        check_normalized_formatted("myName and sUE", "MyName And SUE");
    }

    #[test]
    fn check_year_retrieved_origin_name() {
        check_normalized_formatted(" hmmm a title in 2022 2019", "Hmmm A Title In 2022 (2019)");
    }
}

#[cfg(test)]
mod disk_tests {
    use std::{sync::Arc, path::Path};

    use commander::{tests::{create_test_settings, create_file}, rename::{name::BaseInfo, disk::DiskRenamer, Renamer, MediaFileType}};

    #[tokio::test]
    async fn chheck_similar_media() {
        let settings = Arc::new(create_test_settings());

        let empty_file = "empty";
        create_file(Path::new(&settings.filesystem.movies_path).join("My Coding Movee (2022)").join(empty_file), 1);
        create_file(Path::new(&settings.filesystem.movies_path).join("My Codig Movee (2022-12-01)").join(empty_file), 1);
        create_file(Path::new(&settings.filesystem.movies_path).join("Another Something (2022)").join(empty_file), 1);

        let base = BaseInfo::new("My Coding Novie".to_owned(), Some(1918));
        let renamer = DiskRenamer::new(settings);
        let options = renamer.find_options(&base, MediaFileType::MOVIE).await.unwrap();

        assert!(options.is_some());
        let options = options.unwrap();

        let descs = options.descriptions();
        assert_eq!(2, descs.len());
        assert_eq!("My Coding Movee", &descs[0].title);
        assert_eq!("2022", &descs[0].date);
        assert_eq!("My Codig Movee", &descs[1].title);
        assert_eq!("2022-12-01", &descs[1].date);
    }
}

#[cfg(test)]
mod cache_tests {
    use std::sync::Arc;

    use commander::{tests::{create_mongo_image, create_test_settings, MONGO_USER, MONGO_PASS, MONGO_PORT}, mongo::MongoDbWrapper, db::DbClient, rename::{name::BaseInfo, online_cache::{OnlineCacheItem, OnlineCacheRenamer}, MediaFileType::{MOVIE, TV}, Renamer}};
    use mongodb::Client;
    use testcontainers::clients;

    #[tokio::test]
    async fn check_cache_search() {
        let docker = clients::Cli::default();
        let container = docker.run(create_mongo_image());

        let mut settings = create_test_settings();
        settings.mongodb.connection_url = format!("mongodb://{}:{}@localhost:{}/?retryWrites=true&w=majority",
            MONGO_USER, MONGO_PASS, container.get_host_port_ipv4(MONGO_PORT)
        );
        let settings = Arc::new(settings);

        let mongo_client = Client::with_uri_str(&settings.mongodb.connection_url).await.unwrap();
        let db_wrapper = MongoDbWrapper::new(mongo_client, settings.clone());
        let db_client = DbClient::new(Arc::new(db_wrapper));

        let base = BaseInfo::new("My Movie".to_owned(), Some(2022));
        let desc = "my description";

        let item1 = OnlineCacheItem { 
            search_name: base.name().to_owned(), 
            search_year: base.year(), 
            cover_path: String::new(), 
            title: String::new(), 
            date: 0, 
            description: desc.to_owned(), 
            cast: vec![], 
            media_type: MOVIE, 
        };

        let item2 = OnlineCacheItem { 
            search_name: base.name().to_owned(), 
            search_year: base.year(), 
            cover_path: String::new(), 
            title: String::new(), 
            date: 0, 
            description: desc.to_owned(), 
            cast: vec![], 
            media_type: TV, 
        };

        db_client.online_cache_repo().save_items(vec![item1, item2]).await.unwrap();

        let renamer = OnlineCacheRenamer::new(db_client);
        let options = renamer.find_options(&base,MOVIE).await.unwrap();

        assert!(options.is_some());
        let options = options.unwrap();

        assert_eq!(1, options.descriptions().len());
        assert_eq!(desc.to_owned(), options.descriptions()[0].description);
        assert_eq!("1970-01-01".to_owned(), options.descriptions()[0].date);
    }

    #[tokio::test]
    async fn check_cache_no_year_search() {
        let docker = clients::Cli::default();
        let container = docker.run(create_mongo_image());

        let mut settings = create_test_settings();
        settings.mongodb.connection_url = format!("mongodb://{}:{}@localhost:{}/?retryWrites=true&w=majority",
            MONGO_USER, MONGO_PASS, container.get_host_port_ipv4(MONGO_PORT)
        );
        let settings = Arc::new(settings);

        let mongo_client = Client::with_uri_str(&settings.mongodb.connection_url).await.unwrap();
        let db_wrapper = MongoDbWrapper::new(mongo_client, settings.clone());
        let db_client = DbClient::new(Arc::new(db_wrapper));

        let base = BaseInfo::new("Another Movie".to_owned(), None);
        let desc = "another description";

        let item1 = OnlineCacheItem { 
            search_name: base.name().to_owned(), 
            search_year: base.year(), 
            cover_path: String::new(), 
            title: String::new(), 
            date: 0, 
            description: desc.to_owned(), 
            cast: vec![], 
            media_type: MOVIE, 
        };

        let item2 = OnlineCacheItem { 
            search_name: base.name().to_owned(), 
            search_year: Some(2022), 
            cover_path: String::new(), 
            title: String::new(), 
            date: 0, 
            description: desc.to_owned(), 
            cast: vec![], 
            media_type: MOVIE, 
        };

        db_client.online_cache_repo().save_items(vec![item1, item2]).await.unwrap();

        let renamer = OnlineCacheRenamer::new(db_client);
        let options = renamer.find_options(&base,MOVIE).await.unwrap();

        assert!(options.is_some());
        let options = options.unwrap();

        assert_eq!(2, options.descriptions().len());
    }
}

#[cfg(test)]
mod tmdb_tests {
    use std::sync::Arc;

    use async_trait::async_trait;
    use commander::{tests::{create_test_settings, create_mongo_image, MONGO_USER, MONGO_PASS, MONGO_PORT}, mongo::MongoDbWrapper, db::DbClient, rename::{name::BaseInfo, tmdb::{TmdbRenamer, TmdbSearcher, Tv, Movie}, Renamer, MediaFileType::{MOVIE, TV}, MediaRenameOrigin}};
    use mongodb::Client;
    use testcontainers::clients;

    struct FakeSearcher {
        tvs: Vec<Tv>,
        movies: Vec<Movie>,
        fail: bool,
    }

    impl FakeSearcher {
        fn new(tvs: Vec<Tv>, movies: Vec<Movie>, fail: bool) -> Self {
            FakeSearcher { tvs, movies, fail }
        }
    }

    #[async_trait]
    impl TmdbSearcher for FakeSearcher {
        async fn search_tv(&self, _query: &str, _year: Option<i32>) -> eyre::Result<Vec<Tv>> {
            match self.fail {
                true => Err(eyre::eyre!("expected error")),
                false => Ok(self.tvs.clone()),
            }
        }

        async fn search_movie(&self, _query: &str, _year: Option<i32>) -> eyre::Result<Vec<Movie>> {
            match self.fail {
                true => Err(eyre::eyre!("expected error")),
                false => Ok(self.movies.clone()),
            }
        }
    }

    #[tokio::test]
    async fn check_movie_get_and_cache() {
        let docker = clients::Cli::default();
        let container = docker.run(create_mongo_image());

        let mut settings = create_test_settings();
        settings.mongodb.connection_url = format!("mongodb://{}:{}@localhost:{}/?retryWrites=true&w=majority",
            MONGO_USER, MONGO_PASS, container.get_host_port_ipv4(MONGO_PORT)
        );
        let settings = Arc::new(settings);

        let mongo_client = Client::with_uri_str(&settings.mongodb.connection_url).await.unwrap();
        let db_wrapper = MongoDbWrapper::new(mongo_client, settings.clone());
        let db_client = DbClient::new(Arc::new(db_wrapper));

        let title = "fight club";
        let year = Some(2000);

        let movie = Movie { 
            title: title.to_owned(), 
            poster_path: None, 
            release_date: year.unwrap().to_string(), 
            overview: String::new(), 
            id: 0, 
            cast: vec![],
        };

        let base = BaseInfo::new(title.to_owned(), year);

        let searcher = FakeSearcher::new(vec![], vec![movie], false);
        let renamer = TmdbRenamer::new(settings, searcher, db_client.clone());

        let options = renamer.find_options(&base, MOVIE).await.unwrap();

        assert!(options.is_some());
        let options = options.unwrap();

        assert_eq!(MediaRenameOrigin::TMDB, options.origin()); 
        assert_eq!(1, options.descriptions().len()); 
        assert_eq!(title.to_owned(), options.descriptions()[0].title); 
        assert_eq!(year.unwrap().to_string(), options.descriptions()[0].date); 
        
        let items = db_client.online_cache_repo()
            .retrieve_all_by_base_and_type(&base,MOVIE).await.unwrap();

        assert_eq!(1, items.len());
        assert_eq!(title.to_owned(), items[0].title);
    }
    
    #[tokio::test]
    async fn check_tv_get_and_cache() {
        let docker = clients::Cli::default();
        let container = docker.run(create_mongo_image());

        let mut settings = create_test_settings();
        settings.mongodb.connection_url = format!("mongodb://{}:{}@localhost:{}/?retryWrites=true&w=majority",
            MONGO_USER, MONGO_PASS, container.get_host_port_ipv4(MONGO_PORT)
        );
        let settings = Arc::new(settings);

        let mongo_client = Client::with_uri_str(&settings.mongodb.connection_url).await.unwrap();
        let db_wrapper = MongoDbWrapper::new(mongo_client, settings.clone());
        let db_client = DbClient::new(Arc::new(db_wrapper));

        let title = "game of thrones";
        let year = Some(2011);

        let tv = Tv { 
            name: title.to_owned(), 
            poster_path: Some(String::new()), 
            first_air_date: year.unwrap().to_string(), 
            overview: String::new(), 
            id: 0, 
            cast: vec![],
        };

        let base = BaseInfo::new(title.to_owned(), year);

        let searcher = FakeSearcher::new(vec![tv], vec![], false);
        let renamer = TmdbRenamer::new(settings, searcher, db_client.clone());

        let options = renamer.find_options(&base, TV).await.unwrap();

        assert!(options.is_some());
        let options = options.unwrap();

        assert_eq!(MediaRenameOrigin::TMDB, options.origin()); 
        assert_eq!(1, options.descriptions().len()); 
        assert_eq!(title.to_owned(), options.descriptions()[0].title); 
        assert_eq!(year.unwrap().to_string(), options.descriptions()[0].date); 
        
        let items = db_client.online_cache_repo()
            .retrieve_all_by_base_and_type(&base, TV).await.unwrap();

        assert_eq!(1, items.len());
        assert_eq!(title.to_owned(), items[0].title);
    }
}