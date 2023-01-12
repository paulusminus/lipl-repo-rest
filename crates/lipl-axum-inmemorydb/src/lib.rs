use std::{collections::HashMap, sync::{RwLock, Arc}, cmp::Ordering, iter::empty};


use async_trait::async_trait;
use lipl_core::{
    Error,
    LiplRepo,
    Lyric,
    LyricPost,
    Playlist,
    PlaylistPost,
    Result,
    Summary,
    Uuid,
    Yaml,
    RepoDb,
    reexport::serde_yaml,
    ext::VecExt,
};

#[derive(Clone)]
enum Record {
    Lyric(LyricPost),
    Playlist(PlaylistPost),
}

#[derive(Clone)]
pub struct InMemoryDb {
    db: Arc<RwLock<HashMap<Uuid, Record>>>,
}

impl From<RepoDb> for InMemoryDb {
    fn from(repo_db: RepoDb) -> Self {
        InMemoryDb::new(repo_db.lyrics.into_iter(), repo_db.playlists.into_iter())
    }
}

fn lyric_to_tuple(lyric: Lyric) -> (Uuid, Record) {
    (lyric.id, Record::Lyric(lyric.into()))
}

fn playlist_to_tuple(playlist: Playlist) -> (Uuid, Record) {
    (playlist.id, Record::Playlist(playlist.into()))
}

impl InMemoryDb {
    pub fn new(lyrics: impl Iterator<Item = Lyric>, playlists: impl Iterator<Item = Playlist>) -> Self {
        Self {
            db: Arc::new(
                RwLock::new(
                    HashMap::from_iter(
                        lyrics.map(lyric_to_tuple).chain(playlists.map(playlist_to_tuple)),
                    )
                )
            ),
        }
    }

    fn to_repo_db(&self) -> RepoDb {
        self.db.read().unwrap()
            .iter()
            .fold(
                (Vec::<Lyric>::new(), Vec::<Playlist>::new()),
                |acc, (uuid, record)| {
                    match record {
                        Record::Lyric(lyric_post) =>
                            (
                                acc.0.add_one((Some(*uuid), lyric_post.clone()).into()),
                                acc.1,
                            ),
                        Record::Playlist(playlist_post) =>
                            (
                                acc.0,
                                acc.1.add_one((Some(*uuid), playlist_post.clone()).into()),
                            )
                    }
                }
            )
            .into()
    }
}

impl Default for InMemoryDb {
    fn default() -> Self {
        Self::new(empty(), empty())
    }
}

fn compare_title(a: &Summary, b: &Summary) -> Ordering {
    a.title.cmp(&b.title)
}

fn lyric_compare_title(a: &Lyric, b: &Lyric) -> Ordering {
    a.title.cmp(&b.title)
}

fn playlist_compare_title(a: &Playlist, b: &Playlist) -> Ordering {
    a.title.cmp(&b.title)
}

impl Yaml for InMemoryDb {
    fn load<R>(r: R) -> Result<Self>
    where 
        R: std::io::Read,
        Self: Sized,
    {
        serde_yaml::from_reader::<_, RepoDb>(r)
            .map_err(Into::into)
            .map(InMemoryDb::from)
    }

    fn save<W>(&self, w: W) -> Result<()>
    where
        W: std::io::Write,
    {
        serde_yaml::to_writer(w, &self.to_repo_db())
            .map_err(Into::into)
    }
}

#[async_trait]
impl LiplRepo for InMemoryDb {
    async fn get_lyric_summaries(&self) ->  Result<Vec<Summary>> {
        let mut summaries = self.db.read().unwrap().iter().filter_map(|(key, record)| {
                if let Record::Lyric(lyric_post) = record {
                    Some(Summary { id: *key, title: lyric_post.title.clone() })
                }
                else {
                    None
                }
            })
            .collect::<Vec<_>>();

        summaries.sort_by(compare_title);
        Ok(summaries)
    }

    async fn get_lyrics(&self) ->  Result<Vec<Lyric>> {
        let mut lyrics = self.db.read().unwrap().iter().filter_map(|(key, record)| {
                if let Record::Lyric(lyric_post) = record {
                    Some(Lyric::from((Some(*key), lyric_post.clone())))
                }
                else {
                    None
                }
            })
            .collect::<Vec<_>>();

        lyrics.sort_by(lyric_compare_title);
        Ok(lyrics)
    }

    async fn get_lyric(&self, uuid: Uuid) -> Result<Lyric> {
        self.db.read().unwrap()
        .get(&uuid)
        .and_then(|record| {
            match record {
                Record::Lyric(lyric_post) => Some(Lyric::from((Some(uuid), lyric_post.clone()))),
                _ => None
            }
        })
        .ok_or(Error::NotFound(uuid))
    }

    async fn post_lyric(&self, lyric: Lyric) ->  Result<Lyric> {
        match self.db.write().unwrap().insert(lyric.clone().id, Record::Lyric(lyric.clone().into())) {
            Some(_) => Err(Error::Occupied),
            None => Ok(lyric)
        }
    }

    async fn delete_lyric(&self, uuid: Uuid) -> Result<()> {
        let mut db = self.db.write().unwrap();
        if db.remove(&uuid).is_some() {
            db.iter_mut().for_each(|(_, record)| {
                if let Record::Playlist(playlist_post) = record {
                    *playlist_post = PlaylistPost {
                        title: playlist_post.title.clone(),
                        members: playlist_post.members.clone().without(&uuid)
                    }
                }
            });
            Ok(())
        }
        else {
            Err(Error::NotFound(uuid))
        }
    }

    // async fn put_lyric(&self, uuid: Uuid, lyric_post: LyricPost) -> Result<Lyric> {
    //     let mut db = self.db.write().unwrap();
    //     let lyric = Lyric::from((Some(uuid), lyric_post.clone()));
    //     let entry = db.get_mut(&uuid).ok_or(Error::NotFound(uuid))?;
    //     *entry = Record::Lyric(lyric_post);
    //     Ok(lyric)
    // }

    async fn get_playlist_summaries(&self) -> Result<Vec<Summary>> {
        let mut summaries = self.db.read().unwrap().iter().filter_map(|(key, record)| {
            match record {
                Record::Playlist(playlist_post) => Some(Summary { id: *key, title: playlist_post.title.clone() }),
                _ => None
            }
        })
        .collect::<Vec<_>>();

        summaries.sort_by(compare_title);
        Ok(summaries)
    }

    async fn get_playlists(&self) -> Result<Vec<Playlist>> {
        let mut playlists = self.db.read().unwrap().iter().filter_map(|(key, record)| {
            match record {
                Record::Playlist(playlist_post) => Some(Playlist::from((Some(*key), playlist_post.clone()))),
                _ => None
            }
        })
        .collect::<Vec<_>>();

        playlists.sort_by(playlist_compare_title);
        Ok(playlists)
    }

    async fn get_playlist(&self, uuid: Uuid) -> Result<Playlist> {
        self.db.read().unwrap().get(&uuid)
        .and_then(|record| {
            match record {
                Record::Playlist(playlist_post) => Some(Playlist::from((Some(uuid), playlist_post.clone()))),
                _ => None,
            }
        })
        .ok_or(Error::NotFound(uuid))
    }

    async fn post_playlist(&self, playlist: Playlist) -> Result<Playlist> {
        match self.db.write().unwrap().insert(playlist.clone().id, Record::Playlist(playlist.clone().into())) {
            Some(_) => Err(Error::Occupied),
            None => Ok(playlist)
        }
    }

    async fn delete_playlist(&self, uuid: Uuid) -> Result<()> {
        self.db.write().unwrap().remove(&uuid).ok_or(Error::NotFound(uuid)).map(|_| ())
    }

    async fn stop(&self) -> Result<()> {
        Ok(())
    }

    // async fn playlist_put(&self, uuid: Uuid, playlist_post: PlaylistPost) -> Result<Playlist> {
    //     let playlist = Playlist::from((Some(uuid), playlist_post.clone()));
    //     self.db.write().unwrap().entry(uuid).and_modify(|v| *v = Record::Playlist(playlist_post));
    //     Ok(playlist)
    // }

}

// #[async_trait]
// impl PlaylistDb for InMemoryDb {
//     async fn playlist_list(&self) -> Result<Vec<Summary>> {
//         let mut summaries = self.db.read().unwrap().iter().filter_map(|(key, record)| {
//             match record {
//                 Record::Playlist(playlist_post) => Some(Summary { id: *key, title: playlist_post.title.clone() }),
//                 _ => None
//             }
//         })
//         .collect::<Vec<_>>();

//         summaries.sort_by(compare_title);
//         Ok(summaries)
//     }

//     async fn playlist_list_full(&self) -> Result<Vec<Playlist>> {
//         let mut playlists = self.db.read().unwrap().iter().filter_map(|(key, record)| {
//             match record {
//                 Record::Playlist(playlist_post) => Some(Playlist::from((Some(*key), playlist_post.clone()))),
//                 _ => None
//             }
//         })
//         .collect::<Vec<_>>();

//         playlists.sort_by(playlist_compare_title);
//         Ok(playlists)
//     }

//     async fn playlist_item(&self, uuid: Uuid) -> Result<Playlist> {
//         self.db.read().unwrap().get(&uuid)
//         .and_then(|record| {
//             match record {
//                 Record::Playlist(playlist_post) => Some(Playlist::from((Some(uuid), playlist_post.clone()))),
//                 _ => None,
//             }
//         })
//         .ok_or(Error::NotFound(uuid))
//     }

//     async fn playlist_post(&self, playlist_post: PlaylistPost) -> Result<Playlist> {
//         let uuid = Uuid::default();

//         match self.db.write().unwrap().insert(uuid, Record::Playlist(playlist_post.clone())) {
//             Some(_) => Err(Error::Occupied),
//             None => Ok(Playlist::from((Some(uuid), playlist_post)))
//         }
//     }

//     async fn playlist_delete(&self, uuid: Uuid) -> Result<()> {
//         self.db.write().unwrap().remove(&uuid).ok_or(Error::NotFound(uuid)).map(|_| ())
//     }

//     async fn playlist_put(&self, uuid: Uuid, playlist_post: PlaylistPost) -> Result<Playlist> {
//         let playlist = Playlist::from((Some(uuid), playlist_post.clone()));
//         self.db.write().unwrap().entry(uuid).and_modify(|v| *v = Record::Playlist(playlist_post));
//         Ok(playlist)
//     }
// }


#[cfg(test)]
mod tests {
    use std::iter::empty;

    use lipl_core::{LiplRepo};

    #[tokio::test]
    async fn post_playlist() {
        let db = super::InMemoryDb::new(empty(), empty());

        let playlist_post = super::PlaylistPost {
            title: "Alle 13 goed".to_owned(),
            members: vec![],
        };

        let playlist = db.post_playlist((None, playlist_post).into()).await.unwrap();

        let playlists = db.get_playlists().await.unwrap();
        assert_eq!(playlists[0].title, "Alle 13 goed".to_owned());
        assert_eq!(playlists[0].id, playlist.id);
    }
}
